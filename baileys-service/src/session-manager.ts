import makeWASocket, {
  DisconnectReason,
  useMultiFileAuthState,
  WASocket,
  ConnectionState,
  AuthenticationState,
} from '@whiskeysockets/baileys';
import { Boom } from '@hapi/boom';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import pino from 'pino';

const logger = pino({ name: 'session-manager' });

// --- Types ---

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

// --- Encryption helpers ---

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
  // Format: [iv (12)] [authTag (16)] [ciphertext]
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

// --- Encrypted auth state adapter ---

function getSessionDir(realmId: string): string {
  return path.resolve('./sessions', realmId);
}

function ensureDir(dir: string): void {
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
}

/**
 * Wraps Baileys' useMultiFileAuthState to encrypt files at rest.
 * Files are stored as encrypted blobs in ./sessions/{realmId}/
 */
async function useEncryptedAuthState(realmId: string): Promise<{
  state: AuthenticationState;
  saveCreds: () => Promise<void>;
}> {
  const sessionDir = getSessionDir(realmId);
  ensureDir(sessionDir);

  // On load, decrypt any .enc files back to their original form for Baileys
  const encFiles = fs.readdirSync(sessionDir);
  for (const file of encFiles) {
    if (!file.endsWith('.enc')) continue;
    const filePath = path.join(sessionDir, file);
    const originalPath = filePath.slice(0, -4); // remove .enc
    if (!fs.existsSync(originalPath)) {
      try {
        const encData = fs.readFileSync(filePath);
        const decrypted = decrypt(encData);
        fs.writeFileSync(originalPath, decrypted);
      } catch (err) {
        logger.error({ realmId, file, err }, 'Failed to decrypt session file');
      }
    }
  }

  // Load auth state from decrypted files
  const { state, saveCreds: baseSaveCreds } = await useMultiFileAuthState(sessionDir);

  // Wrap saveCreds to encrypt files after each save
  const encryptSessionFiles = (): void => {
    const currentFiles = fs.readdirSync(sessionDir);
    for (const file of currentFiles) {
      const filePath = path.join(sessionDir, file);
      if (file.endsWith('.enc') || !fs.statSync(filePath).isFile()) continue;
      const content = fs.readFileSync(filePath);
      const encrypted = encrypt(content);
      fs.writeFileSync(filePath + '.enc', encrypted);
      fs.unlinkSync(filePath);
    }
  };

  const saveCreds = async (): Promise<void> => {
    await baseSaveCreds();
    encryptSessionFiles();
  };

  return { state, saveCreds };
}

// --- Session Manager ---

const MAX_CONNECTIONS = Number.parseInt(process.env.MAX_CONNECTIONS || '100', 10);
const BACKEND_WEBHOOK_URL = process.env.BACKEND_WEBHOOK_URL || 'http://backend:8080';
const INTERNAL_TOKEN = process.env.BAILEYS_INTERNAL_TOKEN || '';

const sessions: Map<string, SessionInfo> = new Map();

/**
 * Forward an incoming WhatsApp message to the backend webhook.
 */
async function forwardToBackend(realmId: string, message: any): Promise<void> {
  if (!INTERNAL_TOKEN) {
    logger.warn({ realmId }, 'BAILEYS_INTERNAL_TOKEN not set, skipping webhook forward');
    return;
  }

  const remoteJid = message.key?.remoteJid;
  if (!remoteJid || remoteJid.endsWith('@g.us') || remoteJid === 'status@broadcast') {
    // Skip group messages and status broadcasts
    return;
  }

  // Extract sender phone from JID (format: 18091234567@s.whatsapp.net)
  const senderPhone = '+' + remoteJid.replace('@s.whatsapp.net', '');

  // Determine message type and content
  let messageType: 'text' | 'image' = 'text';
  let content = '';
  let caption: string | undefined;

  const msg = message.message;
  if (!msg) return;

  if (msg.conversation) {
    content = msg.conversation;
  } else if (msg.extendedTextMessage?.text) {
    content = msg.extendedTextMessage.text;
  } else if (msg.imageMessage) {
    messageType = 'image';
    caption = msg.imageMessage.caption || undefined;
    // For images, content remains empty — the backend can fetch media if needed
  } else {
    // Unsupported message type, skip
    return;
  }

  const payload = {
    realmId,
    senderPhone,
    messageType,
    content,
    caption: caption || null,
    messageId: message.key?.id || '',
    timestamp: message.messageTimestamp
      ? Number(message.messageTimestamp)
      : Math.floor(Date.now() / 1000),
  };

  const webhookUrl = `${BACKEND_WEBHOOK_URL}/internal/whatsapp/incoming`;

  try {
    const response = await fetch(webhookUrl, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Internal-Token': INTERNAL_TOKEN,
      },
      body: JSON.stringify(payload),
    });

    if (!response.ok) {
      logger.error(
        { realmId, status: response.status, senderPhone },
        'Backend webhook returned non-OK status'
      );
    }
  } catch (err: any) {
    logger.error({ realmId, err: err.message, senderPhone }, 'Failed to forward message to backend');
  }
}

export function getSession(realmId: string): SessionInfo | undefined {
  return sessions.get(realmId);
}

export function getConnectionCounts(): ConnectionCounts {
  const counts: ConnectionCounts = {
    disconnected: 0,
    qr_pending: 0,
    connected: 0,
    logged_out: 0,
  };
  for (const session of sessions.values()) {
    counts[session.status]++;
  }
  return counts;
}

export function getActiveConnectionCount(): number {
  let count = 0;
  for (const session of sessions.values()) {
    if (session.status === 'connected' || session.status === 'qr_pending') {
      count++;
    }
  }
  return count;
}

/**
 * Start a WhatsApp session for an organization.
 * Returns the session info (with QR code when in qr_pending state).
 * Throws if max connections reached.
 */
export async function startSession(realmId: string): Promise<SessionInfo> {
  // If session already exists and is connected, return it
  const existing = sessions.get(realmId);
  if (existing && (existing.status === 'connected' || existing.status === 'qr_pending')) {
    return existing;
  }

  // Enforce max concurrent connections
  if (getActiveConnectionCount() >= MAX_CONNECTIONS) {
    throw new Error(`Maximum concurrent connections reached (${MAX_CONNECTIONS})`);
  }

  // Initialize session info
  const sessionInfo: SessionInfo = {
    realmId,
    status: 'disconnected',
    qrCode: null,
    socket: null,
  };
  sessions.set(realmId, sessionInfo);

  // Load encrypted auth state
  const { state, saveCreds } = await useEncryptedAuthState(realmId);

  // Create WASocket
  const socket = makeWASocket({
    auth: state,
    logger: pino({ level: 'silent' }) as any,
    printQRInTerminal: false,
  });

  sessionInfo.socket = socket;

  // Handle connection updates
  socket.ev.on('connection.update', (update: Partial<ConnectionState>) => {
    const { connection, lastDisconnect, qr } = update;

    if (qr) {
      sessionInfo.status = 'qr_pending';
      sessionInfo.qrCode = qr;
      logger.info({ realmId }, 'QR code generated');
    }

    if (connection === 'open') {
      sessionInfo.status = 'connected';
      sessionInfo.qrCode = null;
      logger.info({ realmId }, 'Connection established');
    }

    if (connection === 'close') {
      const statusCode = (lastDisconnect?.error as Boom)?.output?.statusCode;

      if (statusCode === DisconnectReason.loggedOut) {
        sessionInfo.status = 'logged_out';
        sessionInfo.socket = null;
        sessionInfo.qrCode = null;
        logger.info({ realmId }, 'Session logged out remotely');
        // Clean up session files on logout
        cleanupSessionFiles(realmId);
      } else {
        sessionInfo.status = 'disconnected';
        sessionInfo.socket = null;
        sessionInfo.qrCode = null;
        logger.info({ realmId, statusCode }, 'Connection closed');
      }
    }
  });

  // Persist credentials on update
  socket.ev.on('creds.update', saveCreds);

  // Forward incoming messages to backend webhook
  socket.ev.on('messages.upsert', async ({ messages: incomingMessages }) => {
    for (const msg of incomingMessages) {
      // Skip messages sent by us
      if (msg.key.fromMe) continue;
      await forwardToBackend(realmId, msg);
    }
  });

  return sessionInfo;
}

/**
 * Stop a WhatsApp session for an organization.
 */
export async function stopSession(realmId: string): Promise<void> {
  const session = sessions.get(realmId);
  if (!session) return;

  if (session.socket) {
    try {
      await session.socket.logout();
    } catch {
      // Socket may already be closed
      session.socket?.end(undefined);
    }
  }

  session.status = 'disconnected';
  session.socket = null;
  session.qrCode = null;
  logger.info({ realmId }, 'Session stopped');
}

/**
 * Get the current status of a session.
 */
export function getStatus(realmId: string): { status: ConnectionStatus; qrCode: string | null } {
  const session = sessions.get(realmId);
  if (!session) {
    return { status: 'disconnected', qrCode: null };
  }
  return { status: session.status, qrCode: session.qrCode };
}

/**
 * Send a message through an active session.
 */
export async function sendMessage(
  realmId: string,
  recipientPhone: string,
  content: string
): Promise<void> {
  const session = sessions.get(realmId);
  if (session?.status !== 'connected' || !session?.socket) {
    throw new Error(`No active connection for realm ${realmId}`);
  }

  const jid = `${recipientPhone.replace('+', '')}@s.whatsapp.net`;
  await session.socket.sendMessage(jid, { text: content });
}

/**
 * Remove session files from disk.
 */
function cleanupSessionFiles(realmId: string): void {
  const sessionDir = getSessionDir(realmId);
  if (fs.existsSync(sessionDir)) {
    fs.rmSync(sessionDir, { recursive: true, force: true });
  }
}
