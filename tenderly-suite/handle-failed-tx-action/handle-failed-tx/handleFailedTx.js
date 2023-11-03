const axios = require('axios').default;
const { ethers } = require('ethers');

const PAGER_DUTY_ALERT_URL = 'https://events.pagerduty.com/v2/enqueue';
const Severity = {
    INFO: 'info',
    CRITICAL: 'critical',
    WARNING: 'warning',
};

const handleFailedTxFn = async (context, event) => {
    if (!event || !event.logs || !context || !context.metadata) {
        throw new Error('INVALID_INPUT_FOR_ACTION');
    }

    const chainName = context.metadata.getNetwork();
    const provider = new ethers.providers.JsonRpcProvider(await context.secrets.get(`RPC_${chainName.toUpperCase()}`));
    const tx = await provider.getTransaction(event.hash);

    const warningThreshold = await context.storage.getStr('WarningThreshold');
    const criticalThreshold = await context.storage.getStr('CriticalThreshold');
    const timeSplit = await context.storage.getStr('TimeSplit');

    if (!tx.to || !tx.from || !tx.blockNumber || !tx.data) {
        throw new Error('INVALID_TX_FORMAT');
    }

    const response = await provider.call(tx, tx.blockNumber);

    if (response.length < 10) {
        throw new Error('INVALID_RESPONSE_LENGTH');
    }

    const errorHash = response.slice(0, 10);
    console.log('errorHash: ', errorHash);
    let warningOptions = [];

    switch (errorHash) {
        case '0xdf359a15':
            warningOptions = ['FlowLimitExceeded', 'TokenManager'];
            break;
        case '0xbb6c1639':
            warningOptions = ['MissingRole', event.to];
            break;
        case '0x7fa6fbb4':
            warningOptions = ['MissingAllRoles', event.to];
            break;
        case '0x218de251':
            warningOptions = ['MissingAnyOfRoles', event.to];
            break;
        case '0xb078d99c':
            warningOptions = ['ReEntrancy', 'TokenManager'];
            break;
        case '0x0d6c7be9':
            warningOptions = ['NotService', 'TokenManager'];
            break;
        default:
            console.log('No Error match found');
    }

    if (warningOptions.length !== 0) {
        await sendWarning(event, context, chainName, ...warningOptions, Severity.INFO);
    }

    const failedTxStartTime = await context.storage.getNumber('FailedTxStartTimestamp');
    let failedTxCount = await context.storage.getNumber('FailedTxCount');

    const timeNow = Date.now();

    if (timeNow - failedTxStartTime > timeSplit) {
        console.log('Updating Time stamp');
        failedTxCount = 1;
        await context.storage.putNumber('FailedTxStartTimestamp', timeNow);
    } else {
        failedTxCount++;

        if (failedTxCount >= criticalThreshold) {
            await sendWarning(
                event,
                context,
                chainName,
                `Threshold crossed for failed transactions: ${failedTxCount}`,
                'ITS_PROJECT',
                Severity.CRITICAL,
            );
        } else if (failedTxCount >= warningThreshold) {
            await sendWarning(
                event,
                context,
                chainName,
                `Threshold crossed for failed transactions: ${failedTxCount}`,
                'ITS_PROJECT',
                Severity.WARNING,
            );
        }
    }

    await context.storage.putNumber('FailedTxCount', failedTxCount);
};

async function sendWarning(event, context, chainName, summary, source, severity) {
    try {
        await axios.post(
            PAGER_DUTY_ALERT_URL,
            {
                routing_key: await context.secrets.get('PD_ROUTING_KEY'),
                event_action: 'trigger',
                payload: {
                    summary,
                    source,
                    severity,
                    custom_details: {
                        timestamp: Date.now(),
                        chain_name: chainName,
                        trigger_event: event,
                    },
                },
            },
            {
                'Content-Type': 'application/json',
            },
        );
    } catch (error) {
        console.log('PD error status: ', error.response.status);
        console.log('PD error data: ', error.response.data);
        throw Error('SENDING_ALERTS_FAILED');
    }
}

module.exports = { handleFailedTxFn };
