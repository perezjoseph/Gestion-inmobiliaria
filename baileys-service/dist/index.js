"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.app = void 0;
const express_1 = __importDefault(require("express"));
const pino_1 = __importDefault(require("pino"));
const session_manager_1 = require("./session-manager");
const logger = (0, pino_1.default)({ name: 'baileys-service' });
const PORT = Number.parseInt(process.env.PORT || '3100', 10);
const app = (0, express_1.default)();
exports.app = app;
app.use(express_1.default.json());
// --- Health endpoint ---
app.get('/health', (_req, res) => {
    const counts = (0, session_manager_1.getConnectionCounts)();
    res.json({
        status: 'ok',
        uptime: process.uptime(),
        connections: counts,
    });
});
// --- Session API endpoints ---
/**
 * POST /sessions/:realmId/start
 * Create a WASocket for the organization, return QR data or current status.
 */
app.post('/sessions/:realmId/start', async (req, res) => {
    const realmId = req.params.realmId;
    try {
        const session = await (0, session_manager_1.startSession)(realmId);
        res.json({
            realmId: session.realmId,
            status: session.status,
            qrCode: session.qrCode,
        });
    }
    catch (err) {
        logger.error({ realmId, err: err.message }, 'Failed to start session');
        const statusCode = err.message?.includes('Maximum concurrent connections') ? 503 : 500;
        res.status(statusCode).json({ error: err.message });
    }
});
/**
 * POST /sessions/:realmId/send
 * Send a message to a recipient phone number through the active session.
 */
app.post('/sessions/:realmId/send', async (req, res) => {
    const realmId = req.params.realmId;
    const { recipientPhone, content } = req.body;
    if (!recipientPhone || !content) {
        res.status(400).json({ error: 'recipientPhone and content are required' });
        return;
    }
    try {
        await (0, session_manager_1.sendMessage)(realmId, recipientPhone, content);
        res.json({ success: true });
    }
    catch (err) {
        logger.error({ realmId, err: err.message }, 'Failed to send message');
        const statusCode = err.message?.includes('No active connection') ? 409 : 500;
        res.status(statusCode).json({ error: err.message });
    }
});
/**
 * POST /sessions/:realmId/stop
 * Disconnect and cleanup the session for the organization.
 */
app.post('/sessions/:realmId/stop', async (req, res) => {
    const realmId = req.params.realmId;
    try {
        await (0, session_manager_1.stopSession)(realmId);
        res.json({ success: true, status: 'disconnected' });
    }
    catch (err) {
        logger.error({ realmId, err: err.message }, 'Failed to stop session');
        res.status(500).json({ error: err.message });
    }
});
/**
 * GET /sessions/:realmId/status
 * Return the current connection status for the organization.
 */
app.get('/sessions/:realmId/status', (req, res) => {
    const realmId = req.params.realmId;
    const sessionStatus = (0, session_manager_1.getStatus)(realmId);
    res.json({
        realmId,
        status: sessionStatus.status,
        qrCode: sessionStatus.qrCode,
    });
});
app.listen(PORT, () => {
    logger.info({ port: PORT }, 'Baileys service started');
});
//# sourceMappingURL=index.js.map