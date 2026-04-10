import { WASocket } from '@whiskeysockets/baileys';
export type ConnectionStatus = 'disconnected' | 'qr_pending' | 'connected' | 'logged_out';
export interface SessionInfo {
    realmId: string;
    status: ConnectionStatus;
    qrCode: string | null;
    socket: WASocket | null;
}
export interface ConnectionCounts {
    disconnected: number;
    qr_pending: number;
    connected: number;
    logged_out: number;
}
export declare function encrypt(plaintext: Buffer): Buffer;
export declare function decrypt(data: Buffer): Buffer;
export declare function getSession(realmId: string): SessionInfo | undefined;
export declare function getConnectionCounts(): ConnectionCounts;
export declare function getActiveConnectionCount(): number;
/**
 * Start a WhatsApp session for an organization.
 * Returns the session info (with QR code when in qr_pending state).
 * Throws if max connections reached.
 */
export declare function startSession(realmId: string): Promise<SessionInfo>;
/**
 * Stop a WhatsApp session for an organization.
 */
export declare function stopSession(realmId: string): Promise<void>;
/**
 * Get the current status of a session.
 */
export declare function getStatus(realmId: string): {
    status: ConnectionStatus;
    qrCode: string | null;
};
/**
 * Send a message through an active session.
 */
export declare function sendMessage(realmId: string, recipientPhone: string, content: string): Promise<void>;
//# sourceMappingURL=session-manager.d.ts.map