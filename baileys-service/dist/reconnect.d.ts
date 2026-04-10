/**
 * Connection states matching the design document state machine.
 */
export type ConnectionStatus = 'disconnected' | 'qr_pending' | 'connected' | 'reconnecting' | 'logged_out';
/**
 * Baileys DisconnectReason codes.
 * Reference: @whiskeysockets/baileys DisconnectReason enum.
 */
export declare const DisconnectReason: {
    readonly connectionClosed: 428;
    readonly connectionLost: 408;
    readonly connectionReplaced: 440;
    readonly timedOut: 408;
    readonly loggedOut: 401;
    readonly restartRequired: 515;
    readonly multideviceMismatch: 411;
    readonly badSession: 500;
};
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
export declare function isRecoverableDisconnect(statusCode: number): boolean;
/**
 * Determines if the disconnect was a remote logout.
 */
export declare function isRemoteLogout(statusCode: number): boolean;
/**
 * Calculates exponential backoff delay with jitter.
 * Formula: min(initialDelay * 2^attempt, maxDelay) + random jitter (0-500ms)
 */
export declare function calculateBackoffDelay(attempt: number, initialDelayMs: number, maxDelayMs: number): number;
/**
 * Notifies the backend about a session status change.
 */
export declare function notifyBackend(realmId: string, status: ConnectionStatus, config: Pick<ReconnectConfig, 'backendWebhookUrl' | 'internalToken'>): Promise<void>;
/**
 * Creates a new reconnect state for a session.
 */
export declare function createReconnectState(): ReconnectState;
/**
 * Loads reconnect configuration from environment variables with defaults.
 */
export declare function loadReconnectConfig(): ReconnectConfig;
/**
 * Manages auto-reconnect logic for Baileys sessions.
 *
 * Handles three disconnect scenarios:
 * 1. Recoverable disconnect → exponential backoff reconnection (up to 5 attempts)
 * 2. Remote logout → transition to logged_out, notify backend
 * 3. QR expiry → regenerate QR up to 3 times, then disconnect
 */
export declare class ReconnectHandler {
    private readonly states;
    private readonly config;
    private readonly callbacks;
    constructor(config: ReconnectConfig, callbacks: SessionCallbacks);
    /**
     * Handles a disconnect event for a session.
     * Determines the appropriate action based on the disconnect reason.
     */
    handleDisconnect(realmId: string, statusCode: number): Promise<void>;
    /**
     * Handles QR code expiry for a session.
     * Regenerates QR up to maxQrRetries times, then transitions to disconnected.
     */
    handleQrExpiry(realmId: string): Promise<void>;
    /**
     * Resets reconnect state for a session (e.g., on successful connection).
     */
    resetState(realmId: string): void;
    /**
     * Removes all state for a session (e.g., on manual disconnect).
     */
    removeState(realmId: string): void;
    /**
     * Returns the current reconnect state for a session, if any.
     */
    getState(realmId: string): ReconnectState | undefined;
    private handleRemoteLogout;
    private handleRecoverableDisconnect;
    private transitionToDisconnected;
    private getOrCreateState;
    private clearTimer;
}
//# sourceMappingURL=reconnect.d.ts.map