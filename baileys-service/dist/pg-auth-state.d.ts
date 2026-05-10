import { AuthenticationState } from '@whiskeysockets/baileys';
import { Pool } from 'pg';
export declare function encrypt(plaintext: Buffer): Buffer;
export declare function decrypt(data: Buffer): Buffer;
export declare function getPool(): Pool;
export declare function closePool(): Promise<void>;
/**
 * PostgreSQL-backed auth state for Baileys.
 * Stores encrypted credentials and Signal keys in whatsapp_auth_* tables.
 */
export declare function usePostgresAuthState(realmId: string): Promise<{
    state: AuthenticationState;
    saveCreds: () => Promise<void>;
}>;
/**
 * Remove all auth data for a realm from the database.
 */
export declare function deleteAuthState(realmId: string): Promise<void>;
/**
 * List all realm IDs that have stored credentials (for session restoration on startup).
 */
export declare function listStoredRealms(): Promise<string[]>;
//# sourceMappingURL=pg-auth-state.d.ts.map