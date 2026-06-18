import { childLogger } from './logger';

const logger = childLogger('reconnect');

type ConnectionStatus =
  | 'disconnected'
  | 'qr_pending'
  | 'connected'
  | 'reconnecting'
  | 'logged_out';

const DisconnectReason = {
  connectionClosed: 428,
  connectionLost: 408,
  connectionReplaced: 440,
  timedOut: 408,
  loggedOut: 401,
  restartRequired: 515,
  multideviceMismatch: 411,
  badSession: 500,
} as const;

interface ReconnectConfig {
  initialDelayMs: number;
  maxDelayMs: number;
  maxAttempts: number;
  maxQrRetries: number;
  backendWebhookUrl: string;
  internalToken: string;
}

const DEFAULT_CONFIG: Omit<ReconnectConfig, 'backendWebhookUrl' | 'internalToken'> = {
  initialDelayMs: 2000,
  maxDelayMs: 60000,
  maxAttempts: 5,
  maxQrRetries: 3,
};

interface ReconnectState {
  attempts: number;
  qrRetries: number;
  isReconnecting: boolean;
  status: ConnectionStatus;
  reconnectTimer: ReturnType<typeof setTimeout> | null;
}

interface SessionCallbacks {
  reconnect(realmId: string): Promise<void>;
  regenerateQr(realmId: string): Promise<void>;
  setStatus(realmId: string, status: ConnectionStatus): void;
  cleanup(realmId: string): void;
}

export function isRecoverableDisconnect(statusCode: number): boolean {
  const recoverableCodes: number[] = [
    DisconnectReason.connectionClosed,
    DisconnectReason.connectionLost,
    DisconnectReason.restartRequired,
    DisconnectReason.multideviceMismatch,
  ];
  return recoverableCodes.includes(statusCode);
}

function isRemoteLogout(statusCode: number): boolean {
  return statusCode === DisconnectReason.loggedOut;
}

export function calculateBackoffDelay(
  attempt: number,
  initialDelayMs: number,
  maxDelayMs: number,
): number {
  const exponentialDelay = initialDelayMs * Math.pow(2, attempt);
  const clampedDelay = Math.min(exponentialDelay, maxDelayMs);
  const jitter = Math.floor(Math.random() * 500);
  return clampedDelay + jitter;
}

async function notifyBackend(
  realmId: string,
  status: ConnectionStatus,
  config: Pick<ReconnectConfig, 'backendWebhookUrl' | 'internalToken'>,
): Promise<void> {
  try {
    const response = await fetch(`${config.backendWebhookUrl}/internal/whatsapp/status`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Internal-Token': config.internalToken,
      },
      body: JSON.stringify({
        realmId,
        status,
        timestamp: Date.now(),
      }),
    });

    if (!response.ok) {
      logger.warn(
        { realmId, status, httpStatus: response.status },
        'Backend notification failed',
      );
    }
  } catch (error) {
    logger.error(
      { realmId, status, error: (error as Error).message },
      'Failed to notify backend of status change',
    );
  }
}

function createReconnectState(): ReconnectState {
  return {
    attempts: 0,
    qrRetries: 0,
    isReconnecting: false,
    status: 'disconnected',
    reconnectTimer: null,
  };
}

function loadReconnectConfig(): ReconnectConfig {
  const backendWebhookUrl = process.env.BACKEND_WEBHOOK_URL || 'http://backend:8080';
  const internalToken = process.env.BAILEYS_INTERNAL_TOKEN || '';

  if (!internalToken) {
    logger.warn('BAILEYS_INTERNAL_TOKEN not set — backend notifications will fail');
  }

  return {
    ...DEFAULT_CONFIG,
    initialDelayMs: Number.parseInt(process.env.RECONNECT_INITIAL_DELAY_MS || '2000', 10),
    maxDelayMs: Number.parseInt(process.env.RECONNECT_MAX_DELAY_MS || '60000', 10),
    maxAttempts: Number.parseInt(process.env.RECONNECT_MAX_ATTEMPTS || '5', 10),
    maxQrRetries: Number.parseInt(process.env.MAX_QR_RETRIES || '3', 10),
    backendWebhookUrl,
    internalToken,
  };
}

class ReconnectHandler {
  private readonly states: Map<string, ReconnectState> = new Map();
  private readonly config: ReconnectConfig;
  private readonly callbacks: SessionCallbacks;

  constructor(config: ReconnectConfig, callbacks: SessionCallbacks) {
    this.config = config;
    this.callbacks = callbacks;
  }

  async handleDisconnect(realmId: string, statusCode: number): Promise<void> {
    const state = this.getOrCreateState(realmId);

    if (isRemoteLogout(statusCode)) {
      await this.handleRemoteLogout(realmId, state);
    } else if (isRecoverableDisconnect(statusCode)) {
      await this.handleRecoverableDisconnect(realmId, state);
    } else {
      logger.info(
        { realmId, statusCode },
        'Non-recoverable disconnect, transitioning to disconnected',
      );
      this.transitionToDisconnected(realmId, state);
    }
  }

  async handleQrExpiry(realmId: string): Promise<void> {
    const state = this.getOrCreateState(realmId);
    state.qrRetries += 1;

    if (state.qrRetries < this.config.maxQrRetries) {
      logger.info(
        { realmId, attempt: state.qrRetries, max: this.config.maxQrRetries },
        'QR expired, regenerating',
      );
      try {
        await this.callbacks.regenerateQr(realmId);
      } catch (error) {
        logger.error(
          { realmId, error: (error as Error).message },
          'Failed to regenerate QR',
        );
        this.transitionToDisconnected(realmId, state);
      }
    } else {
      logger.warn(
        { realmId, qrRetries: state.qrRetries },
        'Max QR retries reached, transitioning to disconnected',
      );
      this.transitionToDisconnected(realmId, state);
    }
  }

  resetState(realmId: string): void {
    const state = this.states.get(realmId);
    if (state) {
      this.clearTimer(state);
      state.attempts = 0;
      state.qrRetries = 0;
      state.isReconnecting = false;
      state.status = 'connected';
    }
  }

  removeState(realmId: string): void {
    const state = this.states.get(realmId);
    if (state) {
      this.clearTimer(state);
    }
    this.states.delete(realmId);
  }

  getState(realmId: string): ReconnectState | undefined {
    return this.states.get(realmId);
  }

  private async handleRemoteLogout(realmId: string, state: ReconnectState): Promise<void> {
    logger.info({ realmId }, 'Remote logout detected, transitioning to logged_out');

    this.clearTimer(state);
    state.status = 'logged_out';
    state.isReconnecting = false;
    state.attempts = 0;

    this.callbacks.setStatus(realmId, 'logged_out');
    this.callbacks.cleanup(realmId);

    await notifyBackend(realmId, 'logged_out', this.config);
  }

  private async handleRecoverableDisconnect(
    realmId: string,
    state: ReconnectState,
  ): Promise<void> {
    if (state.attempts >= this.config.maxAttempts) {
      logger.warn(
        { realmId, attempts: state.attempts },
        'Max reconnection attempts reached, giving up',
      );
      this.transitionToDisconnected(realmId, state);
      return;
    }

    state.isReconnecting = true;
    state.status = 'reconnecting';
    this.callbacks.setStatus(realmId, 'reconnecting');

    const delay = calculateBackoffDelay(
      state.attempts,
      this.config.initialDelayMs,
      this.config.maxDelayMs,
    );

    logger.info(
      { realmId, attempt: state.attempts + 1, maxAttempts: this.config.maxAttempts, delayMs: delay },
      'Scheduling reconnection attempt',
    );

    state.attempts += 1;

    this.clearTimer(state);
    state.reconnectTimer = setTimeout(async () => {
      try {
        await this.callbacks.reconnect(realmId);
      } catch (error) {
        logger.error(
          { realmId, attempt: state.attempts, error: (error as Error).message },
          'Reconnection attempt failed',
        );
        await this.handleRecoverableDisconnect(realmId, state);
      }
    }, delay);
  }

  private transitionToDisconnected(realmId: string, state: ReconnectState): void {
    this.clearTimer(state);
    state.status = 'disconnected';
    state.isReconnecting = false;
    state.attempts = 0;
    state.qrRetries = 0;

    this.callbacks.setStatus(realmId, 'disconnected');
    this.callbacks.cleanup(realmId);
  }

  private getOrCreateState(realmId: string): ReconnectState {
    let state = this.states.get(realmId);
    if (!state) {
      state = createReconnectState();
      this.states.set(realmId, state);
    }
    return state;
  }

  private clearTimer(state: ReconnectState): void {
    if (state.reconnectTimer) {
      clearTimeout(state.reconnectTimer);
      state.reconnectTimer = null;
    }
  }
}
