const Sentry = require('@sentry/node');

if (process.env.SENTRY_DSN) {
    Sentry.init({
        dsn: process.env.SENTRY_DSN,
    });

    process.on('unhandledRejection', (reason) => {
        Sentry.captureException(reason);
        Sentry.flush(2000).then(() => {
            console.error('Unhandled promise rejection:', reason);
            process.exit(1);
        });
    });
}
