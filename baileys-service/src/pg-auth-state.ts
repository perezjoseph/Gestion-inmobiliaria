import {
  AuthenticationCreds,
  AuthenticationState,
  BufferJSON,
  initAuthCreds,
  proto,
  SignalDataTypeMap,
} from '@whiskeysockets/baileys';
import { Pool } from 'pg';
import crypto from 'node:crypto';
import { childLogger } from './logger';

const logger = childLogger('pg-auth-state');

const ALGORITHM = 'aes-256-gcm';
const IV_LENGTH = 12;
const AUTH_TAG_LENGTH = 16;

function getEncryptionKey(): Buffer {
  const keyHex = process.env.SESSION_ENCRYPTION_KEY;
  if (!keyHex || keyHex.length < 64) {
    throw new Error(
      'SESSION_ENCRYPTION_KEY must be set and be at least 64 hex characters (32 bytes)'
    );
  }
  return Buffer.from(keyHex.slice(0, 64), 'hex');
}

export function encrypt(plaintext: Buffer): Buffer {
  const key = getEncryptionKey();
  const iv = crypto.randomBytes(IV_LENGTH);
  const cipher = crypto.createCipheriv(ALGORITHM, key, iv);
  const encrypted = Buffer.concat([cipher.update(plaintext), cipher.final()]);
  const authTag = cipher.getAuthTag();
  return Buffer.concat([iv, authTag, encrypted]);
}

export function decrypt(data: Buffer): Buffer {
  const key = getEncryptionKey();
  if (data.length < IV_LENGTH + AUTH_TAG_LENGTH) {
    throw new Error('Invalid encrypted data: too short');
  }
  const iv = data.subarray(0, IV_LENGTH);
  const authTag = data.subarray(IV_LENGTH, IV_LENGTH + AUTH_TAG_LENGTH);
  const ciphertext = data.subarray(IV_LENGTH + AUTH_TAG_LENGTH);
  const decipher = crypto.createDecipheriv(ALGORITHM, key, iv);
  decipher.setAuthTag(authTag);
  return Buffer.concat([decipher.update(ciphertext), decipher.final()]);
}

let pool: Pool | null = null;

function getPool(): Pool {
  if (!pool) {
    const connectionString = process.env.WA_DATABASE_URL;
    if (!connectionString) {
      throw new Error('WA_DATABASE_URL must be set for PostgreSQL auth state');
    }
    pool = new Pool({
      connectionString,
      max: 5,
      idleTimeoutMillis: 30000,
      connectionTimeoutMillis: 5000,
    });
  }
  return pool;
}

async function closePool(): Promise<void> {
  if (pool) {
    await pool.end();
    pool = null;
  }
}

export async function usePostgresAuthState(realmId: string): Promise<{
  state: AuthenticationState;
  saveCreds: () => Promise<void>;
}> {
  const db = getPool();

  const credsRow = await db.query(
    'SELECT creds_data FROM whatsapp_auth_creds WHERE realm_id = $1',
    [realmId]
  );

  let creds: AuthenticationCreds;
  if (credsRow.rows.length > 0) {
    try {
      const decrypted = decrypt(credsRow.rows[0].creds_data);
      creds = JSON.parse(decrypted.toString('utf-8'), BufferJSON.reviver);
    } catch (err) {
      logger.error({ realmId, err }, 'Failed to decrypt creds, reinitializing');
      creds = initAuthCreds();
    }
  } else {
    creds = initAuthCreds();
  }

  const saveCreds = async (): Promise<void> => {
    const serialized = JSON.stringify(creds, BufferJSON.replacer);
    const encrypted = encrypt(Buffer.from(serialized, 'utf-8'));

    await db.query(
      `INSERT INTO whatsapp_auth_creds (realm_id, creds_data, updated_at)
       VALUES ($1, $2, NOW())
       ON CONFLICT (realm_id) DO UPDATE SET creds_data = $2, updated_at = NOW()`,
      [realmId, encrypted]
    );
  };

  const keys = {
    get: async <T extends keyof SignalDataTypeMap>(
      type: T,
      ids: string[]
    ): Promise<{ [id: string]: SignalDataTypeMap[T] }> => {
      const result: { [id: string]: SignalDataTypeMap[T] } = {};
      if (ids.length === 0) return result;

      const rows = await db.query(
        `SELECT key_id, key_data FROM whatsapp_auth_keys
         WHERE realm_id = $1 AND category = $2 AND key_id = ANY($3)`,
        [realmId, type, ids]
      );

      for (const row of rows.rows) {
        try {
          const decrypted = decrypt(row.key_data);
          let value = JSON.parse(decrypted.toString('utf-8'), BufferJSON.reviver);

          if (type === 'app-state-sync-key' && value) {
            value = proto.Message.AppStateSyncKeyData.fromObject(value);
          }

          result[row.key_id] = value;
        } catch (err) {
          logger.error({ realmId, type, keyId: row.key_id, err }, 'Failed to decrypt key');
        }
      }

      return result;
    },

    set: async (data: { [category: string]: { [id: string]: any } }): Promise<void> => {
      const client = await db.connect();
      try {
        await client.query('BEGIN');

        for (const category of Object.keys(data)) {
          for (const id of Object.keys(data[category])) {
            const value = data[category][id];

            if (value) {
              const serialized = JSON.stringify(value, BufferJSON.replacer);
              const encrypted = encrypt(Buffer.from(serialized, 'utf-8'));

              await client.query(
                `INSERT INTO whatsapp_auth_keys (realm_id, category, key_id, key_data, updated_at)
                 VALUES ($1, $2, $3, $4, NOW())
                 ON CONFLICT (realm_id, category, key_id)
                 DO UPDATE SET key_data = $4, updated_at = NOW()`,
                [realmId, category, id, encrypted]
              );
            } else {
              await client.query(
                `DELETE FROM whatsapp_auth_keys
                 WHERE realm_id = $1 AND category = $2 AND key_id = $3`,
                [realmId, category, id]
              );
            }
          }
        }

        await client.query('COMMIT');
      } catch (err) {
        await client.query('ROLLBACK');
        throw err;
      } finally {
        client.release();
      }
    },
  };

  return { state: { creds, keys }, saveCreds };
}

export async function deleteAuthState(realmId: string): Promise<void> {
  const db = getPool();
  const client = await db.connect();
  try {
    await client.query('BEGIN');
    await client.query('DELETE FROM whatsapp_auth_keys WHERE realm_id = $1', [realmId]);
    await client.query('DELETE FROM whatsapp_auth_creds WHERE realm_id = $1', [realmId]);
    await client.query('COMMIT');
  } catch (err) {
    await client.query('ROLLBACK');
    throw err;
  } finally {
    client.release();
  }
}

export async function listStoredRealms(): Promise<string[]> {
  const db = getPool();
  const result = await db.query('SELECT realm_id FROM whatsapp_auth_creds');
  return result.rows.map((row) => row.realm_id);
}
