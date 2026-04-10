import pino from 'pino';

const logger = pino({ name: 'baileys-reconnect' });

/**
 * Connection states matching the design document state machine.
 */
export type ConnectionStatus =
  | 'disconnected'
  | 'qr_pending'
  | 'connected'
  | 'reconnecting'
  | 'logged_out';

/**
 * Baileys DisconnectReason codes.
 * Reference: @whiskeysockets/baileys DisconnectReason enum.
 */
export const DisconnectReason = {
  connectionClosed: 428,
  connectionLost: 408,
  connectionReplaced: 440,
  timedOut: 408,
  loggedOut: 401,
  restartRequired: 515,
  multideviceMismatch: 411,
  badSession: 500,
} as const;

/**
 * Configuration for the reconnect strategy.
 */
export interface ReconnectConfig {
  /** Initial backoff delay in milliseconds. Default: 2000 */
  initialDelayMs: number;
  /** Maximum backoff delay in milliseconds. Default: 60000 */
  maxDelayMs: number;
  /** Maximum number of reconnection attempts. Default: 5 */
  maxAttempts: number;
  /** Maximum QR regeneration attempts before giving up. Default: 3 */
  maxQrRetries: number;
  /** Backend webhook URL for status notifications */
  backendWebhookUrl: string;
  /** Internal token for authenticating with the backend */
  internalToken: string;
}

const DEFAULT_CONFIG: Omit<ReconnectConfig, 'backendWebhookUrl' | 'internalToken'> = {
  initialDelayMs: 2000,
  maxDelayMs: 60000,
  maxAttempts: 5,
  maxQrRetries: 3,
};

/**
 * Tracks reconnection state for a single session.
 */
export interface ReconnectState {
  /** Current number of reconnection attempts */
  attempts: number;
  /** Current number of QR regeneration attempts */
  qrRetries: number;
  /** Whether a reconnection is currently in progress */
  isReconnecting: boolean;
  /** Current connection status */
  status: ConnectionStatus;
  /** Timer handle for pending reconnect delay */
  reconnectTimer: ReturnType<typeof setTimeout> | null;
}

/**
 * Callback interface for the session manager to implement.
 * The reconnect handler calls these to trigger actual Baileys operations.
 */
export interface SessionCallbacks {
  /** Attempt to reconnect the WASocket (without new QR) */
  reconnect(realmId: string): Promise<void>;
  /** Regenerate QR code for the session */
  regenerateQr(realmId: string): Promise<void>;
  /** Update the session's connection status */
  setStatus(realmId: string, status: ConnectionStatus): void;
  /** Clean up session resources after permanent failure */
  cleanup(realmId: string): void;
}

/**
 * Determines if a disconnect reason is recoverable (should trigger auto-reconnect).
 */
export function isRecoverableDisconnect(statusCode: number): boolean {
  const recoverableCodes: number[] = [
    DisconnectReason.connectionClosed,
    DisconnectReason.connectionLost,
    DisconnectReason.restartRequired,
    DisconnectReason.multideviceMismatch,
  ];
  return recoverableCodes.includes(statusCode);
}

/**
 * Determines if the disconnect was a remote logout.
 */
export function isRemoteLogout(statusCode: number): boolean {
  return statusCode === DisconnectReason.loggedOut;
}

/**
 * Calculates exponential backoff delay with jitter.
 * Formula: min(initialDelay * 2^attempt, maxDelay) + random jitter (0-500ms)
 */
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

/**
 * Notifies the backend about a session status change.
 */
export async function notifyBackend(
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

/**
 * Creates a new reconnect state for a session.
 */
export function createReconnectState(): ReconnectState {
  return {
    attempts: 0,
    qrRetries: 0,
    isReconnecting: false,
    status: 'disconnected',
    reconnectTimer: null,
  };
}

/**
 * Loads reconnect configuration from environment variables with defaults.
 */
export function loadReconnectConfig(): ReconnectConfig {
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

/**
 * Manages auto-reconnect logic for Baileys sessions.
 *
 * Handles three disconnect scenarios:
 * 1. Recoverable disconnect → exponential backoff reconnection (up to 5 attempts)
 * 2. Remote logout → transition to logged_out, notify backend
 * 3. QR expiry → regenerate QR up to 3 times, then disconnect
 */
export class ReconnectHandler {
  private readonly states: Map<string, ReconnectState> = new Map();
  private readonly config: ReconnectConfig;
  private readonly callbacks: SessionCallbacks;

  constructor(config: ReconnectConfig, callbacks: SessionCallbacks) {
    this.config = config;
    this.callbacks = callbacks;
  }

  /**
   * Handles a disconnect event for a session.
   * Determines the appropriate action based on the disconnect reason.
   */
  async handleDisconnect(realmId: string, statusCode: number): Promise<void> {
    const state = this.getOrCreateState(realmId);

    if (isRemoteLogout(statusCode)) {
      await this.handleRemoteLogout(realmId, state);
    } else if (isRecoverableDisconnect(statusCode)) {
      await this.handleRecoverableDisconnect(realmId, state);
    } else {
      // Non-recoverable, non-logout disconnect — give up
      logger.info(
        { realmId, statusCode },
        'Non-recoverable disconnect, transitioning to disconnected',
      );
      this.transitionToDisconnected(realmId, state);
    }
  }

  /**
   * Handles QR code expiry for a session.
   * Regenerates QR up to maxQrRetries times, then transitions to disconnected.
   */
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

  /**
   * Resets reconnect state for a session (e.g., on successful connection).
   */
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

  /**
   * Removes all state for a session (e.g., on manual disconnect).
   */
  removeState(realmId: string): void {
    const state = this.states.get(realmId);
    if (state) {
      this.clearTimer(state);
    }
    this.states.delete(realmId);
  }

  /**
   * Returns the current reconnect state for a session, if any.
   */
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
        // If reconnect succeeds, resetState will be called by the session manager
        // on the 'connection.update' event with status 'open'
      } catch (error) {
        logger.error(
          { realmId, attempt: state.attempts, error: (error as Error).message },
          'Reconnection attempt failed',
        );
        // Recursively try again (will check maxAttempts at the top)
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
