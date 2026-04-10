import express, { Request, Response } from 'express';
import pino from 'pino';
import QRCode from 'qrcode';
import { getConnectionCounts, startSession, stopSession, getStatus, sendMessage } from './session-manager';

const logger = pino({ name: 'baileys-service' });

const PORT = Number.parseInt(process.env.PORT || '3100', 10);

const app = express();
app.use(express.json());

// --- Health endpoint ---

app.get('/health', (_req: Request, res: Response) => {
  const counts = getConnectionCounts();
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
app.post('/sessions/:realmId/start', async (req: Request, res: Response) => {
  const realmId = req.params.realmId as string;
  try {
    const session = await startSession(realmId);
    let qr: string | null = null;
    if (session.qrCode) {
      qr = await QRCode.toDataURL(session.qrCode, { width: 256 });
    }
    res.json({
      realmId: session.realmId,
      status: session.status,
      qr,
    });
  } catch (err: any) {
    logger.error({ realmId, err: err.message }, 'Failed to start session');
    const statusCode = err.message?.includes('Maximum concurrent connections') ? 503 : 500;
    res.status(statusCode).json({ error: err.message });
  }
});

/**
 * POST /sessions/:realmId/send
 * Send a message to a recipient phone number through the active session.
 */
app.post('/sessions/:realmId/send', async (req: Request, res: Response) => {
  const realmId = req.params.realmId as string;
  const { recipientPhone, content } = req.body;

  if (!recipientPhone || !content) {
    res.status(400).json({ error: 'recipientPhone and content are required' });
    return;
  }

  try {
    await sendMessage(realmId, recipientPhone, content);
    res.json({ success: true });
  } catch (err: any) {
    logger.error({ realmId, err: err.message }, 'Failed to send message');
    const statusCode = err.message?.includes('No active connection') ? 409 : 500;
    res.status(statusCode).json({ error: err.message });
  }
});

/**
 * POST /sessions/:realmId/stop
 * Disconnect and cleanup the session for the organization.
 */
app.post('/sessions/:realmId/stop', async (req: Request, res: Response) => {
  const realmId = req.params.realmId as string;
  try {
    await stopSession(realmId);
    res.json({ success: true, status: 'disconnected' });
  } catch (err: any) {
    logger.error({ realmId, err: err.message }, 'Failed to stop session');
    res.status(500).json({ error: err.message });
  }
});

/**
 * GET /sessions/:realmId/status
 * Return the current connection status for the organization.
 */
app.get('/sessions/:realmId/status', async (req: Request, res: Response) => {
  const realmId = req.params.realmId as string;
  const sessionStatus = getStatus(realmId);
  let qr: string | null = null;
  if (sessionStatus.qrCode) {
    qr = await QRCode.toDataURL(sessionStatus.qrCode, { width: 256 });
  }
  res.json({
    realmId,
    status: sessionStatus.status,
    qr,
  });
});

app.listen(PORT, () => {
  logger.info({ port: PORT }, 'Baileys service started');
});

export { app };
