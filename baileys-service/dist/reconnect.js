"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.ReconnectHandler = exports.DisconnectReason = void 0;
exports.isRecoverableDisconnect = isRecoverableDisconnect;
exports.isRemoteLogout = isRemoteLogout;
exports.calculateBackoffDelay = calculateBackoffDelay;
exports.notifyBackend = notifyBackend;
exports.createReconnectState = createReconnectState;
exports.loadReconnectConfig = loadReconnectConfig;
const pino_1 = __importDefault(require("pino"));
const logger = (0, pino_1.default)({ name: 'baileys-reconnect' });
/**
 * Baileys DisconnectReason codes.
 * Reference: @whiskeysockets/baileys DisconnectReason enum.
 */
exports.DisconnectReason = {
    connectionClosed: 428,
    connectionLost: 408,
    connectionReplaced: 440,
    timedOut: 408,
    loggedOut: 401,
    restartRequired: 515,
    multideviceMismatch: 411,
    badSession: 500,
};
const DEFAULT_CONFIG = {
    initialDelayMs: 2000,
    maxDelayMs: 60000,
    maxAttempts: 5,
    maxQrRetries: 3,
};
/**
 * Determines if a disconnect reason is recoverable (should trigger auto-reconnect).
 */
function isRecoverableDisconnect(statusCode) {
    const recoverableCodes = [
        exports.DisconnectReason.connectionClosed,
        exports.DisconnectReason.connectionLost,
        exports.DisconnectReason.restartRequired,
        exports.DisconnectReason.multideviceMismatch,
    ];
    return recoverableCodes.includes(statusCode);
}
/**
 * Determines if the disconnect was a remote logout.
 */
function isRemoteLogout(statusCode) {
    return statusCode === exports.DisconnectReason.loggedOut;
}
/**
 * Calculates exponential backoff delay with jitter.
 * Formula: min(initialDelay * 2^attempt, maxDelay) + random jitter (0-500ms)
 */
function calculateBackoffDelay(attempt, initialDelayMs, maxDelayMs) {
    const exponentialDelay = initialDelayMs * Math.pow(2, attempt);
    const clampedDelay = Math.min(exponentialDelay, maxDelayMs);
    const jitter = Math.floor(Math.random() * 500);
    return clampedDelay + jitter;
}
/**
 * Notifies the backend about a session status change.
 */
async function notifyBackend(realmId, status, config) {
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
            logger.warn({ realmId, status, httpStatus: response.status }, 'Backend notification failed');
        }
    }
    catch (error) {
        logger.error({ realmId, status, error: error.message }, 'Failed to notify backend of status change');
    }
}
/**
 * Creates a new reconnect state for a session.
 */
function createReconnectState() {
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
function loadReconnectConfig() {
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
class ReconnectHandler {
    states = new Map();
    config;
    callbacks;
    constructor(config, callbacks) {
        this.config = config;
        this.callbacks = callbacks;
    }
    /**
     * Handles a disconnect event for a session.
     * Determines the appropriate action based on the disconnect reason.
     */
    async handleDisconnect(realmId, statusCode) {
        const state = this.getOrCreateState(realmId);
        if (isRemoteLogout(statusCode)) {
            await this.handleRemoteLogout(realmId, state);
        }
        else if (isRecoverableDisconnect(statusCode)) {
            await this.handleRecoverableDisconnect(realmId, state);
        }
        else {
            // Non-recoverable, non-logout disconnect — give up
            logger.info({ realmId, statusCode }, 'Non-recoverable disconnect, transitioning to disconnected');
            this.transitionToDisconnected(realmId, state);
        }
    }
    /**
     * Handles QR code expiry for a session.
     * Regenerates QR up to maxQrRetries times, then transitions to disconnected.
     */
    async handleQrExpiry(realmId) {
        const state = this.getOrCreateState(realmId);
        state.qrRetries += 1;
        if (state.qrRetries < this.config.maxQrRetries) {
            logger.info({ realmId, attempt: state.qrRetries, max: this.config.maxQrRetries }, 'QR expired, regenerating');
            try {
                await this.callbacks.regenerateQr(realmId);
            }
            catch (error) {
                logger.error({ realmId, error: error.message }, 'Failed to regenerate QR');
                this.transitionToDisconnected(realmId, state);
            }
        }
        else {
            logger.warn({ realmId, qrRetries: state.qrRetries }, 'Max QR retries reached, transitioning to disconnected');
            this.transitionToDisconnected(realmId, state);
        }
    }
    /**
     * Resets reconnect state for a session (e.g., on successful connection).
     */
    resetState(realmId) {
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
    removeState(realmId) {
        const state = this.states.get(realmId);
        if (state) {
            this.clearTimer(state);
        }
        this.states.delete(realmId);
    }
    /**
     * Returns the current reconnect state for a session, if any.
     */
    getState(realmId) {
        return this.states.get(realmId);
    }
    async handleRemoteLogout(realmId, state) {
        logger.info({ realmId }, 'Remote logout detected, transitioning to logged_out');
        this.clearTimer(state);
        state.status = 'logged_out';
        state.isReconnecting = false;
        state.attempts = 0;
        this.callbacks.setStatus(realmId, 'logged_out');
        this.callbacks.cleanup(realmId);
        await notifyBackend(realmId, 'logged_out', this.config);
    }
    async handleRecoverableDisconnect(realmId, state) {
        if (state.attempts >= this.config.maxAttempts) {
            logger.warn({ realmId, attempts: state.attempts }, 'Max reconnection attempts reached, giving up');
            this.transitionToDisconnected(realmId, state);
            return;
        }
        state.isReconnecting = true;
        state.status = 'reconnecting';
        this.callbacks.setStatus(realmId, 'reconnecting');
        const delay = calculateBackoffDelay(state.attempts, this.config.initialDelayMs, this.config.maxDelayMs);
        logger.info({ realmId, attempt: state.attempts + 1, maxAttempts: this.config.maxAttempts, delayMs: delay }, 'Scheduling reconnection attempt');
        state.attempts += 1;
        this.clearTimer(state);
        state.reconnectTimer = setTimeout(async () => {
            try {
                await this.callbacks.reconnect(realmId);
                // If reconnect succeeds, resetState will be called by the session manager
                // on the 'connection.update' event with status 'open'
            }
            catch (error) {
                logger.error({ realmId, attempt: state.attempts, error: error.message }, 'Reconnection attempt failed');
                // Recursively try again (will check maxAttempts at the top)
                await this.handleRecoverableDisconnect(realmId, state);
            }
        }, delay);
    }
    transitionToDisconnected(realmId, state) {
        this.clearTimer(state);
        state.status = 'disconnected';
        state.isReconnecting = false;
        state.attempts = 0;
        state.qrRetries = 0;
        this.callbacks.setStatus(realmId, 'disconnected');
        this.callbacks.cleanup(realmId);
    }
    getOrCreateState(realmId) {
        let state = this.states.get(realmId);
        if (!state) {
            state = createReconnectState();
            this.states.set(realmId, state);
        }
        return state;
    }
    clearTimer(state) {
        if (state.reconnectTimer) {
            clearTimeout(state.reconnectTimer);
            state.reconnectTimer = null;
        }
    }
}
exports.ReconnectHandler = ReconnectHandler;
//# sourceMappingURL=reconnect.js.map