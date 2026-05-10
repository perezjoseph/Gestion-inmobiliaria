"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.getSession = getSession;
exports.getConnectionCounts = getConnectionCounts;
exports.getActiveConnectionCount = getActiveConnectionCount;
exports.startSession = startSession;
exports.stopSession = stopSession;
exports.getStatus = getStatus;
exports.sendMessage = sendMessage;
exports.restoreSessions = restoreSessions;
const baileys_1 = __importStar(require("@whiskeysockets/baileys"));
const pino_1 = __importDefault(require("pino"));
const pg_auth_state_1 = require("./pg-auth-state");
const reconnect_1 = require("./reconnect");
const RECONNECT_DELAY_MS = 1000;
const logger = (0, pino_1.default)({ name: 'session-manager' });
// --- Session Manager ---
const MAX_CONNECTIONS = Number.parseInt(process.env.MAX_CONNECTIONS || '100', 10);
const BACKEND_WEBHOOK_URL = process.env.BACKEND_WEBHOOK_URL || 'http://backend:8080';
const INTERNAL_TOKEN = process.env.BAILEYS_INTERNAL_TOKEN || '';
const sessions = new Map();
/**
 * Forward an incoming WhatsApp message to the backend webhook.
 */
async function forwardToBackend(realmId, message) {
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
    let messageType = 'text';
    let content = '';
    let caption;
    const msg = message.message;
    if (!msg)
        return;
    if (msg.conversation) {
        content = msg.conversation;
    }
    else if (msg.extendedTextMessage?.text) {
        content = msg.extendedTextMessage.text;
    }
    else if (msg.imageMessage) {
        messageType = 'image';
        caption = msg.imageMessage.caption || undefined;
        // For images, content remains empty — the backend can fetch media if needed
    }
    else {
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
            logger.error({ realmId, status: response.status, senderPhone }, 'Backend webhook returned non-OK status');
        }
    }
    catch (err) {
        logger.error({ realmId, err: err.message, senderPhone }, 'Failed to forward message to backend');
    }
}
function getSession(realmId) {
    return sessions.get(realmId);
}
function getConnectionCounts() {
    const counts = {
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
function getActiveConnectionCount() {
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
async function startSession(realmId) {
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
    const sessionInfo = {
        realmId,
        status: 'disconnected',
        qrCode: null,
        socket: null,
    };
    sessions.set(realmId, sessionInfo);
    // Load auth state from PostgreSQL
    const { state, saveCreds } = await (0, pg_auth_state_1.usePostgresAuthState)(realmId);
    // Wrap keys with in-memory cache to minimize DB round-trips
    const cachedKeys = (0, baileys_1.makeCacheableSignalKeyStore)(state.keys, (0, pino_1.default)({ level: 'silent' }));
    // Create WASocket
    const socket = (0, baileys_1.default)({
        auth: { creds: state.creds, keys: cachedKeys },
        logger: (0, pino_1.default)({ level: 'silent' }),
        printQRInTerminal: false,
    });
    sessionInfo.socket = socket;
    // Handle connection updates
    socket.ev.on('connection.update', (update) => {
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
            const statusCode = lastDisconnect?.error?.output?.statusCode;
            if (statusCode === baileys_1.DisconnectReason.loggedOut) {
                sessionInfo.status = 'logged_out';
                sessionInfo.socket = null;
                sessionInfo.qrCode = null;
                logger.info({ realmId }, 'Session logged out remotely');
                // Clean up auth state from database on logout
                (0, pg_auth_state_1.deleteAuthState)(realmId).catch((err) => logger.error({ realmId, err }, 'Failed to delete auth state on logout'));
            }
            else if (typeof statusCode === 'number' && (0, reconnect_1.isRecoverableDisconnect)(statusCode)) {
                // Recoverable disconnects include 515 (restartRequired) emitted right after QR scan
                // success, plus transient codes 408/428/440/411. Reconnect using the creds persisted
                // via `creds.update` so pairing can complete.
                sessionInfo.status = 'disconnected';
                sessionInfo.socket = null;
                sessionInfo.qrCode = null;
                logger.info({ realmId, statusCode }, 'Recoverable disconnect, reconnecting');
                setTimeout(() => {
                    startSession(realmId).catch((err) => logger.error({ realmId, err: err.message }, 'Failed to reconnect session'));
                }, RECONNECT_DELAY_MS);
            }
            else {
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
            if (msg.key.fromMe)
                continue;
            await forwardToBackend(realmId, msg);
        }
    });
    return sessionInfo;
}
/**
 * Stop a WhatsApp session for an organization.
 */
async function stopSession(realmId) {
    const session = sessions.get(realmId);
    if (!session)
        return;
    if (session.socket) {
        try {
            await session.socket.logout();
        }
        catch {
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
function getStatus(realmId) {
    const session = sessions.get(realmId);
    if (!session) {
        return { status: 'disconnected', qrCode: null };
    }
    return { status: session.status, qrCode: session.qrCode };
}
/**
 * Send a message through an active session.
 */
async function sendMessage(realmId, recipientPhone, content) {
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
async function restoreSessions() {
    const realms = await (0, pg_auth_state_1.listStoredRealms)();
    logger.info({ count: realms.length }, 'Restoring sessions from database');
    for (const realmId of realms) {
        try {
            await startSession(realmId);
            logger.info({ realmId }, 'Session restored');
        }
        catch (err) {
            logger.error({ realmId, err: err.message }, 'Failed to restore session');
        }
    }
}
//# sourceMappingURL=session-manager.js.map