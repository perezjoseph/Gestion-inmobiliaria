import express, { Request, Response } from 'express';
import pino from 'pino';
import QRCode, { QRCodeToDataURLOptions } from 'qrcode';
import { getConnectionCounts, startSession, stopSession, getStatus, sendMessage, restoreSessions } from './session-manager';

const logger = pino({ name: 'baileys-service' });

const PORT = Number.parseInt(process.env.PORT || '3100', 10);

// Render QR at 2x the displayed 256px size for crisp scanning on high-DPI screens,
// with a proper quiet zone margin and medium error correction (WhatsApp's QR payload
// is large enough that 'H' would overflow).
const QR_OPTIONS: QRCodeToDataURLOptions = {
  width: 512,
  margin: 4,
  errorCorrectionLevel: 'M',
  color: { dark: '#000000', light: '#FFFFFF' },
};

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
      qr = await QRCode.toDataURL(session.qrCode, QR_OPTIONS);
    }
    res.json({
      realmId: session.realmId,
      status: session.status,
      qr,
      connectedPhone: session.connectedPhone,
      connectedAt: session.connectedAt,
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
    qr = await QRCode.toDataURL(sessionStatus.qrCode, QR_OPTIONS);
  }
  res.json({
    realmId,
    status: sessionStatus.status,
    qr,
    connectedPhone: sessionStatus.connectedPhone,
    connectedAt: sessionStatus.connectedAt,
  });
});

app.listen(PORT, () => {
  logger.info({ port: PORT }, 'Baileys service started');
  restoreSessions().catch((err) => {
    logger.error({ err: err.message }, 'Failed to restore sessions on startup');
  });
});

export { app };
