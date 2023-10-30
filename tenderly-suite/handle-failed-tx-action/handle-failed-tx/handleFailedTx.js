const axios = require('axios').default;
const { ethers } = require('ethers');

const WARNING_THRESHOLD = 10; // TODO: discuss for production
const CRITICAL_THRESHOLD = 20;
const TIME_SPLIT = 5 * 3600 * 1000; //  5 hours in milliseconds
const PAGER_DUTY_ALERT_URL = 'https://events.pagerduty.com/v2/enqueue';

const handleFailedTxFn = async (context, event) => {
    const chainName = context.metadata.getNetwork();
    const provider = new ethers.providers.JsonRpcProvider(await context.secrets.get(`RPC_${chainName.toUpperCase()}`));
    const tx = await provider.getTransaction(event.hash);

    const response = await provider.call(tx, tx.blockNumber);
    const errorHash = response.slice(0, 10);
    console.log('errorHash: ', errorHash);
    let warningOptions = [];

    switch (errorHash) {
        case '0xdf359a15':
            warningOptions = ['FlowLimitExceeded', 'TokenManager'];
            break;
        case '0xbb6c1639':
            warningOptions = ['MissingRole', '-'];
            break;
        case '0x90a6e7d6':
            warningOptions = ['MissingAllRoles', '-'];
            break;
        case '0xb94d593e':
            warningOptions = ['MissingAnyOfRoles', '-'];
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
        await sendWarning(event, context, chainName, ...warningOptions, 'info');
    }

    const failedTxStartTime = await context.storage.getNumber('FailedTxStartTimestamp');
    let failedTxCount = await context.storage.getNumber('FailedTxCount');

    const timeNow = Date.now();

    if (timeNow - failedTxStartTime > TIME_SPLIT) {
        console.log('Updating Time stamp');
        failedTxCount = 1;
        await context.storage.putNumber('FailedTxStartTimestamp', timeNow);
    } else {
        failedTxCount++;

        if (failedTxCount >= CRITICAL_THRESHOLD) {
            await sendWarning(
                event,
                context,
                chainName,
                `Threshold crossed for failed transactions: ${failedTxCount}`,
                'ITS_PROJECT',
                'critical',
            );
        } else if (failedTxCount >= WARNING_THRESHOLD) {
            await sendWarning(
                event,
                context,
                chainName,
                `Threshold crossed for failed transactions: ${failedTxCount}`,
                'ITS_PROJECT',
                'warning',
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
