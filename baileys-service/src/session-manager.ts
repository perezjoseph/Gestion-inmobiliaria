import makeWASocket, {
  DisconnectReason,
  makeCacheableSignalKeyStore,
  WASocket,
  ConnectionState,
} from '@whiskeysockets/baileys';
import { Boom } from '@hapi/boom';
import pino from 'pino';
import { usePostgresAuthState, deleteAuthState, listStoredRealms } from './pg-auth-state';
import { calculateBackoffDelay, isRecoverableDisconnect } from './reconnect';

const RECONNECT_INITIAL_DELAY_MS = Number.parseInt(
  process.env.RECONNECT_INITIAL_DELAY_MS || '2000',
  10,
);
const RECONNECT_MAX_DELAY_MS = Number.parseInt(process.env.RECONNECT_MAX_DELAY_MS || '60000', 10);
const RECONNECT_MAX_ATTEMPTS = Number.parseInt(process.env.RECONNECT_MAX_ATTEMPTS || '5', 10);

const logger = pino({ name: 'session-manager' });

// --- Types ---

export type ConnectionStatus = 'disconnected' | 'qr_pending' | 'connected' | 'logged_out';

export interface SessionInfo {
  realmId: string;
  status: ConnectionStatus;
  qrCode: string | null;
  socket: WASocket | null;
  connectedPhone: string | null;
  connectedAt: string | null;
  reconnectAttempts: number;
  reconnectTimer: ReturnType<typeof setTimeout> | null;
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
 * Handle a recoverable disconnect with capped exponential backoff.
 * Extracted to keep the connection.update handler under complexity limits.
 */
function scheduleReconnect(sessionInfo: SessionInfo, statusCode: number): void {
  const { realmId } = sessionInfo;

  // Don't stack reconnect timers if one is already pending.
  if (sessionInfo.reconnectTimer) {
    logger.debug({ realmId, statusCode }, 'Reconnect already scheduled, skipping');
    return;
  }

  if (sessionInfo.reconnectAttempts >= RECONNECT_MAX_ATTEMPTS) {
    logger.warn(
      { realmId, statusCode, attempts: sessionInfo.reconnectAttempts },
      'Max reconnection attempts reached, giving up',
    );
    sessionInfo.reconnectAttempts = 0;
    return;
  }

  const delayMs = calculateBackoffDelay(
    sessionInfo.reconnectAttempts,
    RECONNECT_INITIAL_DELAY_MS,
    RECONNECT_MAX_DELAY_MS,
  );
  sessionInfo.reconnectAttempts += 1;

  logger.info(
    {
      realmId,
      statusCode,
      attempt: sessionInfo.reconnectAttempts,
      maxAttempts: RECONNECT_MAX_ATTEMPTS,
      delayMs,
    },
    'Recoverable disconnect, scheduling reconnect',
  );

  sessionInfo.reconnectTimer = setTimeout(() => {
    sessionInfo.reconnectTimer = null;
    startSession(realmId).catch((err) =>
      logger.error({ realmId, err: err.message }, 'Failed to reconnect session'),
    );
  }, delayMs);
}

/**
 * Route a close event to the appropriate handler (logout / recoverable / fatal).
 */
function handleConnectionClose(sessionInfo: SessionInfo, statusCode: number | undefined): void {
  const { realmId } = sessionInfo;

  if (statusCode === DisconnectReason.loggedOut) {
    sessionInfo.status = 'logged_out';
    sessionInfo.socket = null;
    sessionInfo.qrCode = null;
    sessionInfo.connectedPhone = null;
    sessionInfo.connectedAt = null;
    sessionInfo.reconnectAttempts = 0;
    logger.info({ realmId }, 'Session logged out remotely');
    deleteAuthState(realmId).catch((err) =>
      logger.error({ realmId, err }, 'Failed to delete auth state on logout'),
    );
    return;
  }

  if (typeof statusCode === 'number' && isRecoverableDisconnect(statusCode)) {
    // Recoverable disconnects include 515 (restartRequired) emitted right after QR scan
    // success, plus transient codes 408/428/411. Reconnect using the creds persisted
    // via `creds.update` so pairing can complete.
    sessionInfo.status = 'disconnected';
    sessionInfo.socket = null;
    sessionInfo.qrCode = null;
    scheduleReconnect(sessionInfo, statusCode);
    return;
  }

  sessionInfo.status = 'disconnected';
  sessionInfo.socket = null;
  sessionInfo.qrCode = null;
  logger.info({ realmId, statusCode }, 'Connection closed');
}

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

  // Reuse existing session info across reconnect attempts so the backoff counter
  // isn't reset. Only create a fresh one if this is the first start for the realm.
  const sessionInfo: SessionInfo = existing ?? {
    realmId,
    status: 'disconnected',
    qrCode: null,
    socket: null,
    connectedPhone: null,
    connectedAt: null,
    reconnectAttempts: 0,
    reconnectTimer: null,
  };
  sessionInfo.status = 'disconnected';
  sessionInfo.qrCode = null;
  sessionInfo.socket = null;
  sessions.set(realmId, sessionInfo);

  // Load auth state from PostgreSQL
  const { state, saveCreds } = await usePostgresAuthState(realmId);

  // If creds already have a paired identity, pre-populate the phone number
  // (happens on session restore after pod restart)
  if (state.creds.me?.id) {
    const phone = '+' + state.creds.me.id.replace(/:.*$/, '').replace('@s.whatsapp.net', '');
    sessionInfo.connectedPhone = phone;
  }

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
      sessionInfo.reconnectAttempts = 0;
      if (sessionInfo.reconnectTimer) {
        clearTimeout(sessionInfo.reconnectTimer);
        sessionInfo.reconnectTimer = null;
      }

      // Extract connected phone from creds.me (populated after pairing)
      const me = state.creds.me;
      if (me?.id) {
        // me.id format: "18091234567:123@s.whatsapp.net" or "18091234567@s.whatsapp.net"
        const phone = '+' + me.id.replace(/:.*$/, '').replace('@s.whatsapp.net', '');
        sessionInfo.connectedPhone = phone;
      }
      if (!sessionInfo.connectedAt) {
        sessionInfo.connectedAt = new Date().toISOString();
      }

      logger.info({ realmId, phone: sessionInfo.connectedPhone }, 'Connection established');
    }

    if (connection === 'close') {
      const statusCode = (lastDisconnect?.error as Boom)?.output?.statusCode;
      handleConnectionClose(sessionInfo, statusCode);
    }
  });

  // Persist credentials on update. A transient PG pool timeout must not crash
  // the process — Baileys emits creds.update from inside socket event handlers,
  // so an unhandled rejection here takes down the pod.
  socket.ev.on('creds.update', async () => {
    try {
      await saveCreds();
    } catch (err) {
      logger.error(
        { realmId, err: (err as Error).message },
        'Failed to persist creds (will retry on next update)',
      );
    }
  });

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

  if (session.reconnectTimer) {
    clearTimeout(session.reconnectTimer);
    session.reconnectTimer = null;
  }
  session.reconnectAttempts = 0;

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
  session.connectedPhone = null;
  session.connectedAt = null;
  logger.info({ realmId }, 'Session stopped');
}

/**
 * Get the current status of a session.
 */
export function getStatus(realmId: string): { status: ConnectionStatus; qrCode: string | null; connectedPhone: string | null; connectedAt: string | null } {
  const session = sessions.get(realmId);
  if (!session) {
    return { status: 'disconnected', qrCode: null, connectedPhone: null, connectedAt: null };
  }
  return { status: session.status, qrCode: session.qrCode, connectedPhone: session.connectedPhone, connectedAt: session.connectedAt };
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
