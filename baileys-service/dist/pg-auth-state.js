"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.encrypt = encrypt;
exports.decrypt = decrypt;
exports.getPool = getPool;
exports.closePool = closePool;
exports.usePostgresAuthState = usePostgresAuthState;
exports.deleteAuthState = deleteAuthState;
exports.listStoredRealms = listStoredRealms;
const baileys_1 = require("@whiskeysockets/baileys");
const pg_1 = require("pg");
const node_crypto_1 = __importDefault(require("node:crypto"));
const pino_1 = __importDefault(require("pino"));
const logger = (0, pino_1.default)({ name: 'pg-auth-state' });
// --- Encryption (same AES-256-GCM as before) ---
const ALGORITHM = 'aes-256-gcm';
const IV_LENGTH = 12;
const AUTH_TAG_LENGTH = 16;
function getEncryptionKey() {
    const keyHex = process.env.SESSION_ENCRYPTION_KEY;
    if (!keyHex || keyHex.length < 64) {
        throw new Error('SESSION_ENCRYPTION_KEY must be set and be at least 64 hex characters (32 bytes)');
    }
    return Buffer.from(keyHex.slice(0, 64), 'hex');
}
function encrypt(plaintext) {
    const key = getEncryptionKey();
    const iv = node_crypto_1.default.randomBytes(IV_LENGTH);
    const cipher = node_crypto_1.default.createCipheriv(ALGORITHM, key, iv);
    const encrypted = Buffer.concat([cipher.update(plaintext), cipher.final()]);
    const authTag = cipher.getAuthTag();
    return Buffer.concat([iv, authTag, encrypted]);
}
function decrypt(data) {
    const key = getEncryptionKey();
    if (data.length < IV_LENGTH + AUTH_TAG_LENGTH) {
        throw new Error('Invalid encrypted data: too short');
    }
    const iv = data.subarray(0, IV_LENGTH);
    const authTag = data.subarray(IV_LENGTH, IV_LENGTH + AUTH_TAG_LENGTH);
    const ciphertext = data.subarray(IV_LENGTH + AUTH_TAG_LENGTH);
    const decipher = node_crypto_1.default.createDecipheriv(ALGORITHM, key, iv);
    decipher.setAuthTag(authTag);
    return Buffer.concat([decipher.update(ciphertext), decipher.final()]);
}
// --- PostgreSQL Auth State ---
let pool = null;
function getPool() {
    if (!pool) {
        const connectionString = process.env.WA_DATABASE_URL;
        if (!connectionString) {
            throw new Error('WA_DATABASE_URL must be set for PostgreSQL auth state');
        }
        pool = new pg_1.Pool({
            connectionString,
            max: 5,
            idleTimeoutMillis: 30000,
            connectionTimeoutMillis: 5000,
        });
    }
    return pool;
}
async function closePool() {
    if (pool) {
        await pool.end();
        pool = null;
    }
}
/**
 * PostgreSQL-backed auth state for Baileys.
 * Stores encrypted credentials and Signal keys in whatsapp_auth_* tables.
 */
async function usePostgresAuthState(realmId) {
    const db = getPool();
    // --- Load or initialize creds ---
    const credsRow = await db.query('SELECT creds_data FROM whatsapp_auth_creds WHERE realm_id = $1', [realmId]);
    let creds;
    if (credsRow.rows.length > 0) {
        try {
            const decrypted = decrypt(credsRow.rows[0].creds_data);
            creds = JSON.parse(decrypted.toString('utf-8'), baileys_1.BufferJSON.reviver);
        }
        catch (err) {
            logger.error({ realmId, err }, 'Failed to decrypt creds, reinitializing');
            creds = (0, baileys_1.initAuthCreds)();
        }
    }
    else {
        creds = (0, baileys_1.initAuthCreds)();
    }
    // --- saveCreds: persist device identity ---
    const saveCreds = async () => {
        const serialized = JSON.stringify(creds, baileys_1.BufferJSON.replacer);
        const encrypted = encrypt(Buffer.from(serialized, 'utf-8'));
        await db.query(`INSERT INTO whatsapp_auth_creds (realm_id, creds_data, updated_at)
       VALUES ($1, $2, NOW())
       ON CONFLICT (realm_id) DO UPDATE SET creds_data = $2, updated_at = NOW()`, [realmId, encrypted]);
    };
    // --- keys: Signal Protocol key store ---
    const keys = {
        get: async (type, ids) => {
            const result = {};
            if (ids.length === 0)
                return result;
            const rows = await db.query(`SELECT key_id, key_data FROM whatsapp_auth_keys
         WHERE realm_id = $1 AND category = $2 AND key_id = ANY($3)`, [realmId, type, ids]);
            for (const row of rows.rows) {
                try {
                    const decrypted = decrypt(row.key_data);
                    let value = JSON.parse(decrypted.toString('utf-8'), baileys_1.BufferJSON.reviver);
                    if (type === 'app-state-sync-key' && value) {
                        value = baileys_1.proto.Message.AppStateSyncKeyData.fromObject(value);
                    }
                    result[row.key_id] = value;
                }
                catch (err) {
                    logger.error({ realmId, type, keyId: row.key_id, err }, 'Failed to decrypt key');
                }
            }
            return result;
        },
        set: async (data) => {
            const client = await db.connect();
            try {
                await client.query('BEGIN');
                for (const category of Object.keys(data)) {
                    for (const id of Object.keys(data[category])) {
                        const value = data[category][id];
                        if (value) {
                            const serialized = JSON.stringify(value, baileys_1.BufferJSON.replacer);
                            const encrypted = encrypt(Buffer.from(serialized, 'utf-8'));
                            await client.query(`INSERT INTO whatsapp_auth_keys (realm_id, category, key_id, key_data, updated_at)
                 VALUES ($1, $2, $3, $4, NOW())
                 ON CONFLICT (realm_id, category, key_id)
                 DO UPDATE SET key_data = $4, updated_at = NOW()`, [realmId, category, id, encrypted]);
                        }
                        else {
                            await client.query(`DELETE FROM whatsapp_auth_keys
                 WHERE realm_id = $1 AND category = $2 AND key_id = $3`, [realmId, category, id]);
                        }
                    }
                }
                await client.query('COMMIT');
            }
            catch (err) {
                await client.query('ROLLBACK');
                throw err;
            }
            finally {
                client.release();
            }
        },
    };
    return { state: { creds, keys }, saveCreds };
}
/**
 * Remove all auth data for a realm from the database.
 */
async function deleteAuthState(realmId) {
    const db = getPool();
    const client = await db.connect();
    try {
        await client.query('BEGIN');
        await client.query('DELETE FROM whatsapp_auth_keys WHERE realm_id = $1', [realmId]);
        await client.query('DELETE FROM whatsapp_auth_creds WHERE realm_id = $1', [realmId]);
        await client.query('COMMIT');
    }
    catch (err) {
        await client.query('ROLLBACK');
        throw err;
    }
    finally {
        client.release();
    }
}
/**
 * List all realm IDs that have stored credentials (for session restoration on startup).
 */
async function listStoredRealms() {
    const db = getPool();
    const result = await db.query('SELECT realm_id FROM whatsapp_auth_creds');
    return result.rows.map((row) => row.realm_id);
}
//# sourceMappingURL=pg-auth-state.js.map