import makeWASocket, {
  DisconnectReason,
  makeCacheableSignalKeyStore,
  WASocket,
  ConnectionState,
} from '@whiskeysockets/baileys';
import { Boom } from '@hapi/boom';
import pino from 'pino';
import { usePostgresAuthState, deleteAuthState, listStoredRealms } from './pg-auth-state';
import { isRecoverableDisconnect } from './reconnect';

const RECONNECT_DELAY_MS = 1000;

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

  // Load auth state from PostgreSQL
  const { state, saveCreds } = await usePostgresAuthState(realmId);

  // Wrap keys with in-memory cache to minimize DB round-trips
  const cachedKeys = makeCacheableSignalKeyStore(state.keys, pino({ level: 'silent' }) as any);

  // Create WASocket
  const socket = makeWASocket({
    auth: { creds: state.creds, keys: cachedKeys },
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
        // Clean up auth state from database on logout
        deleteAuthState(realmId).catch((err) =>
          logger.error({ realmId, err }, 'Failed to delete auth state on logout')
        );
      } else if (typeof statusCode === 'number' && isRecoverableDisconnect(statusCode)) {
        // Recoverable disconnects include 515 (restartRequired) emitted right after QR scan
        // success, plus transient codes 408/428/440/411. Reconnect using the creds persisted
        // via `creds.update` so pairing can complete.
        sessionInfo.status = 'disconnected';
        sessionInfo.socket = null;
        sessionInfo.qrCode = null;
        logger.info({ realmId, statusCode }, 'Recoverable disconnect, reconnecting');
        setTimeout(() => {
          startSession(realmId).catch((err) =>
            logger.error({ realmId, err: err.message }, 'Failed to reconnect session')
          );
        }, RECONNECT_DELAY_MS);
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
 * Restore all sessions that have stored credentials in the database.
 * Called on startup to reconnect previously authenticated sessions.
 */
export async function restoreSessions(): Promise<void> {
  const realms = await listStoredRealms();
  logger.info({ count: realms.length }, 'Restoring sessions from database');

  for (const realmId of realms) {
    try {
      await startSession(realmId);
      logger.info({ realmId }, 'Session restored');
    } catch (err: any) {
      logger.error({ realmId, err: err.message }, 'Failed to restore session');
    }
  }
}
