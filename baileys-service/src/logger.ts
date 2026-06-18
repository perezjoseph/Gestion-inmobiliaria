import pino from 'pino';

const logger = pino({
    formatters: {
        level(label) {
            return { level: label };
        },
    },
    timestamp: pino.stdTimeFunctions.isoTime,
});

export function childLogger(name: string) {
    return logger.child({ service: 'baileys-service', component: name });
}
