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
import { childLogger } from './logger';
import { usePostgresAuthState, deleteAuthState, listStoredRealms } from './pg-auth-state';
import { calculateBackoffDelay, isRecoverableDisconnect } from './reconnect';
import { setConnectionUp, incMessages, incReconnect, incSessionRestore } from './metrics';

const RECONNECT_INITIAL_DELAY_MS = Number.parseInt(
  process.env.RECONNECT_INITIAL_DELAY_MS || '2000',
  10,
);
const RECONNECT_MAX_DELAY_MS = Number.parseInt(process.env.RECONNECT_MAX_DELAY_MS || '60000', 10);
const RECONNECT_MAX_ATTEMPTS = Number.parseInt(process.env.RECONNECT_MAX_ATTEMPTS || '5', 10);

const logger = childLogger('session-manager');

export type ConnectionStatus = 'disconnected' | 'qr_pending' | 'connected' | 'logged_out';

export interface SessionInfo {
  realmId: string;
  status: ConnectionStatus;
  qrCode: string | null;
  socket: WASocket | null;
  connectedPhone: string | null;
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

const MAX_CONNECTIONS = Number.parseInt(process.env.MAX_CONNECTIONS || '100', 10);
const BACKEND_WEBHOOK_URL = process.env.BACKEND_WEBHOOK_URL || 'http://backend:8080';
const INTERNAL_TOKEN = process.env.BAILEYS_INTERNAL_TOKEN || '';

const sessions: Map<string, SessionInfo> = new Map();

const sentMessageIds: Set<string> = new Set();
const SENT_IDS_MAX_SIZE = 500;

function jidToPhone(jid: string | undefined | null): string | null {
  if (!jid) return null;
  const decoded = jidDecode(jid);
  if (decoded?.server !== 's.whatsapp.net') return null;
  return '+' + decoded.user;
}

function collectOwnJids(me: { id?: string; lid?: string; phoneNumber?: string } | undefined): string[] {
  if (!me) return [];
  const jids = [me.id, me.lid, me.phoneNumber].filter(
    (v): v is string => typeof v === 'string' && v.length > 0,
  );
  return Array.from(new Set(jids));
}

function scheduleReconnect(sessionInfo: SessionInfo, statusCode: number): void {
  const { realmId } = sessionInfo;

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

  incReconnect(realmId);

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

function handleConnectionClose(sessionInfo: SessionInfo, statusCode: number | undefined): void {
  const { realmId } = sessionInfo;

  setConnectionUp(realmId, false);

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
    setConnectionUp(realmId, true);
    if (sessionInfo.reconnectTimer) {
      clearTimeout(sessionInfo.reconnectTimer);
      sessionInfo.reconnectTimer = null;
    }

    const me = state.creds.me;
    if (me) {
      sessionInfo.ownJids = collectOwnJids(me);
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

function shouldForwardMessage(msg: any, sessionInfo: SessionInfo): boolean {
  if (msg.key?.id && sentMessageIds.has(msg.key.id)) {
    sentMessageIds.delete(msg.key.id);
    return false;
  }

  if (msg.key.fromMe) {
    const remoteCandidates: (string | undefined)[] = [msg.key?.remoteJid, msg.key?.remoteJidAlt];
    const isSelfChat = remoteCandidates.some((candidate) =>
      sessionInfo.ownJids.some((own) => areJidsSameUser(candidate, own)),
    );
    return isSelfChat;
  }

  return true;
}

async function forwardToBackend(realmId: string, message: any): Promise<void> {
  if (!INTERNAL_TOKEN) {
    logger.warn({ realmId }, 'BAILEYS_INTERNAL_TOKEN not set, skipping webhook forward');
    return;
  }

  const remoteJid: string | undefined = message.key?.remoteJid;
  const remoteJidAlt: string | undefined = message.key?.remoteJidAlt;
  if (!remoteJid || remoteJid.endsWith('@g.us') || remoteJid === 'status@broadcast') {
    return;
  }

  const senderPhone = jidToPhone(remoteJid) ?? jidToPhone(remoteJidAlt);
  if (!senderPhone) {
    logger.warn(
      { realmId, remoteJid, remoteJidAlt },
      'Skipping message: could not resolve sender phone (no PN-form JID available)',
    );
    return;
  }

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
  } else {
    return;
  }

  const sessionInfo = sessions.get(realmId);
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
    sessionPhone: sessionInfo?.connectedPhone ?? null,
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

    if (response.ok) {
      incMessages(realmId, 'inbound', 'ok');
    } else {
      logger.error(
        { realmId, status: response.status, senderPhone },
        'Backend webhook returned non-OK status'
      );
      incMessages(realmId, 'inbound', 'error');
    }
  } catch (err: any) {
    logger.error({ realmId, err: err.message, senderPhone }, 'Failed to forward message to backend');
    incMessages(realmId, 'inbound', 'error');
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

export async function startSession(realmId: string): Promise<SessionInfo> {
  const existing = sessions.get(realmId);
  if (existing && (existing.status === 'connected' || existing.status === 'qr_pending')) {
    return existing;
  }

  if (getActiveConnectionCount() >= MAX_CONNECTIONS) {
    throw new Error(`Maximum concurrent connections reached (${MAX_CONNECTIONS})`);
  }

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

  const { state, saveCreds } = await usePostgresAuthState(realmId);

  if (state.creds.me) {
    sessionInfo.ownJids = collectOwnJids(state.creds.me);
    const phoneJid =
      state.creds.me.phoneNumber || (isPnUser(state.creds.me.id) ? state.creds.me.id : undefined);
    const phone = jidToPhone(phoneJid);
    if (phone) {
      sessionInfo.connectedPhone = phone;
    }
  }

  const cachedKeys = makeCacheableSignalKeyStore(state.keys, pino({ level: 'silent' }) as any);

  const socket = makeWASocket({
    auth: { creds: state.creds, keys: cachedKeys },
    logger: pino({ level: 'silent' }) as any,
    printQRInTerminal: false,
  });

  sessionInfo.socket = socket;

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

export function getStatus(realmId: string): { status: ConnectionStatus; qrCode: string | null; connectedPhone: string | null; connectedAt: string | null } {
  const session = sessions.get(realmId);
  if (!session) {
    return { status: 'disconnected', qrCode: null, connectedPhone: null, connectedAt: null };
  }
  return { status: session.status, qrCode: session.qrCode, connectedPhone: session.connectedPhone, connectedAt: session.connectedAt };
}

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
  let sent;
  try {
    sent = await session.socket.sendMessage(jid, { text: content });
  } catch (err) {
    incMessages(realmId, 'outbound', 'error');
    throw err;
  }
  incMessages(realmId, 'outbound', 'ok');

  if (sent?.key?.id) {
    sentMessageIds.add(sent.key.id);
    if (sentMessageIds.size > SENT_IDS_MAX_SIZE) {
      const first = sentMessageIds.values().next().value;
      if (first) sentMessageIds.delete(first);
    }
  }
}

export async function restoreSessions(): Promise<void> {
  const realms = await listStoredRealms();
  logger.info({ count: realms.length }, 'Restoring sessions from database');

  for (const realmId of realms) {
    try {
      await startSession(realmId);
      incSessionRestore('success');
      logger.info({ realmId }, 'Session restored');
    } catch (err: any) {
      incSessionRestore('failure');
      logger.error({ realmId, err: err.message }, 'Failed to restore session');
    }
  }
}
