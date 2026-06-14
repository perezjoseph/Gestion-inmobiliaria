import { Registry, collectDefaultMetrics, Gauge, Counter } from 'prom-client';

export const registry = new Registry();

collectDefaultMetrics({ register: registry });

const connectionUp = new Gauge({
    name: 'whatsapp_connection_up',
    help: 'WhatsApp connection state per realm (1 connected, 0 otherwise)',
    labelNames: ['realm'],
    registers: [registry],
});

const messagesTotal = new Counter({
    name: 'whatsapp_messages_total',
    help: 'WhatsApp messages processed',
    labelNames: ['realm', 'direction', 'status'],
    registers: [registry],
});

const reconnectsTotal = new Counter({
    name: 'whatsapp_reconnects_total',
    help: 'WhatsApp reconnect attempts scheduled',
    labelNames: ['realm'],
    registers: [registry],
});

const sessionRestoreTotal = new Counter({
    name: 'whatsapp_session_restore_total',
    help: 'WhatsApp session restore outcomes on startup',
    labelNames: ['result'],
    registers: [registry],
});

export function setConnectionUp(realm: string, up: boolean): void {
    connectionUp.labels(realm).set(up ? 1 : 0);
}

export type MessageDirection = 'inbound' | 'outbound';
export type MessageStatus = 'ok' | 'error';

export function incMessages(realm: string, direction: MessageDirection, status: MessageStatus): void {
    messagesTotal.labels(realm, direction, status).inc();
}

export function incReconnect(realm: string): void {
    reconnectsTotal.labels(realm).inc();
}

export type RestoreResult = 'success' | 'failure';

export function incSessionRestore(result: RestoreResult): void {
    sessionRestoreTotal.labels(result).inc();
}
