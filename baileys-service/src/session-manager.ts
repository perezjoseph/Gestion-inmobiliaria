import makeWASocket, {
  DisconnectReason,
  areJidsSameUser,
  isPnUser,
  jidDecode,
  makeCacheableSignalKeyStore,
  WASocket,
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
  /** All JIDs that identify the connected device (PN and/or LID forms). Used to detect self-chat messages. */
  ownJids: string[];
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

// Track message IDs sent by the bot to avoid re-processing echoed self-messages
const sentMessageIds: Set<string> = new Set();
const SENT_IDS_MAX_SIZE = 500;

/**
 * Convert a Baileys PN-format JID into a `+E.164` phone string.
 * Returns `null` for non-PN JIDs (e.g. `@lid`).
 */
function jidToPhone(jid: string | undefined | null): string | null {
  if (!jid) return null;
  const decoded = jidDecode(jid);
  if (!decoded || decoded.server !== 's.whatsapp.net') return null;
  return '+' + decoded.user;
}

/**
 * Collect every JID that identifies the connected device. In Baileys v7 this can include
 * both the phone-number JID (`@s.whatsapp.net`) and the LID (`@lid`); either may appear in
 * an incoming message's `key.remoteJid` for a self-chat.
 */
function collectOwnJids(me: { id?: string; lid?: string; phoneNumber?: string } | undefined): string[] {
  if (!me) return [];
  const jids = [me.id, me.lid, me.phoneNumber].filter(
    (v): v is string => typeof v === 'string' && v.length > 0,
  );
  return Array.from(new Set(jids));
}

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
    sessionInfo.ownJids = [];
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
 * Handle a connection.update event: QR generation, connection open, connection close.
 */
function handleConnectionUpdate(
  update: { connection?: string; lastDisconnect?: any; qr?: string },
  sessionInfo: SessionInfo,
  state: { creds: { me?: { id: string; lid?: string; phoneNumber?: string } } },
): void {
  const { connection, lastDisconnect, qr } = update;
  const { realmId } = sessionInfo;

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

    const me = state.creds.me;
    if (me) {
      sessionInfo.ownJids = collectOwnJids(me);
      // Prefer the phone-number JID for display; fall back to whichever PN-form JID we have.
      const phoneJid = me.phoneNumber || (isPnUser(me.id) ? me.id : undefined);
      const phone = jidToPhone(phoneJid);
      if (phone) {
        sessionInfo.connectedPhone = phone;
      }
    }
    if (!sessionInfo.connectedAt) {
      sessionInfo.connectedAt = new Date().toISOString();
    }

    logger.info(
      { realmId, phone: sessionInfo.connectedPhone, ownJids: sessionInfo.ownJids },
      'Connection established',
    );
  }

  if (connection === 'close') {
    const statusCode = (lastDisconnect?.error as Boom)?.output?.statusCode;
    handleConnectionClose(sessionInfo, statusCode);
  }
}

/**
 * Determine whether an incoming message should be forwarded to the backend.
 * Filters out bot-sent echoes and non-self fromMe messages.
 */
function shouldForwardMessage(msg: any, sessionInfo: SessionInfo): boolean {
  // Skip messages we sent programmatically (bot replies echoing back)
  if (msg.key?.id && sentMessageIds.has(msg.key.id)) {
    sentMessageIds.delete(msg.key.id);
    return false;
  }

  // For self-messages (messaging your own number), Baileys delivers
  // with fromMe=true. Allow these through so the bot can respond.
  //
  // In Baileys v7 the remote JID can be either the phone-number form
  // (`@s.whatsapp.net`) or the LID form (`@lid`). Compare against every JID
  // that identifies the connected device, plus the alternate JID Baileys
  // attaches to the message key (`remoteJidAlt`).
  if (msg.key.fromMe) {
    const remoteCandidates: (string | undefined)[] = [msg.key?.remoteJid, msg.key?.remoteJidAlt];
    const isSelfChat = remoteCandidates.some((candidate) =>
      sessionInfo.ownJids.some((own) => areJidsSameUser(candidate, own)),
    );
    return isSelfChat;
  }

  return true;
}

/**
 * Forward an incoming WhatsApp message to the backend webhook.
 */
async function forwardToBackend(realmId: string, message: any): Promise<void> {
  if (!INTERNAL_TOKEN) {
    logger.warn({ realmId }, 'BAILEYS_INTERNAL_TOKEN not set, skipping webhook forward');
    return;
  }

  const remoteJid: string | undefined = message.key?.remoteJid;
  const remoteJidAlt: string | undefined = message.key?.remoteJidAlt;
  if (!remoteJid || remoteJid.endsWith('@g.us') || remoteJid === 'status@broadcast') {
    // Skip group messages and status broadcasts
    return;
  }

  // Resolve the sender's phone number. The backend identifies tenants by phone, so we
  // need a PN-form JID. Try `remoteJid` first; if it's a `@lid`, fall back to
  // `remoteJidAlt` (Baileys v7 attaches the PN as the alternate JID for LID chats).
  const senderPhone = jidToPhone(remoteJid) ?? jidToPhone(remoteJidAlt);
  if (!senderPhone) {
    logger.warn(
      { realmId, remoteJid, remoteJidAlt },
      'Skipping message: could not resolve sender phone (no PN-form JID available)',
    );
    return;
  }

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
    ownJids: [],
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

  // If creds already have a paired identity, pre-populate identifiers so a self-message
  // arriving during the brief reconnect window is still recognised.
  if (state.creds.me) {
    sessionInfo.ownJids = collectOwnJids(state.creds.me);
    const phoneJid =
      state.creds.me.phoneNumber || (isPnUser(state.creds.me.id) ? state.creds.me.id : undefined);
    const phone = jidToPhone(phoneJid);
    if (phone) {
      sessionInfo.connectedPhone = phone;
    }
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

  // Use batch event processing (recommended by Baileys for production)
  socket.ev.process(async (events) => {
    if (events['connection.update']) {
      handleConnectionUpdate(events['connection.update'], sessionInfo, state);
    }

    if (events['creds.update']) {
      try {
        await saveCreds();
      } catch (err) {
        logger.error(
          { realmId, err: (err as Error).message },
          'Failed to persist creds (will retry on next update)',
        );
      }
    }

    if (events['messages.upsert']) {
      const { messages: incomingMessages, type } = events['messages.upsert'];
      if (type === 'notify') {
        for (const msg of incomingMessages) {
          if (shouldForwardMessage(msg, sessionInfo)) {
            await forwardToBackend(realmId, msg);
          }
        }
      }
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
  session.ownJids = [];
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
  const sent = await session.socket.sendMessage(jid, { text: content });

  // Track the sent message ID so we don't re-process it when it echoes back
  if (sent?.key?.id) {
    sentMessageIds.add(sent.key.id);
    // Evict oldest entries if the set grows too large
    if (sentMessageIds.size > SENT_IDS_MAX_SIZE) {
      const first = sentMessageIds.values().next().value;
      if (first) sentMessageIds.delete(first);
    }
  }
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
