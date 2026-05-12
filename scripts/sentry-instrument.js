const Sentry = require('@sentry/node');

if (process.env.SENTRY_DSN) {
    Sentry.init({
        dsn: process.env.SENTRY_DSN,
    });
}

process.on('unhandledRejection', async (err) => {
    console.error(err);
    Sentry.captureException(err);
    await Sentry.close(2000);
    process.exit(1);
});

process.on('uncaughtException', async (err) => {
    console.error(err);
    Sentry.captureException(err);
    await Sentry.close(2000);
    process.exit(1);
});
