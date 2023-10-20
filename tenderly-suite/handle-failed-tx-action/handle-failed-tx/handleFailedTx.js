const axios = require('axios').default;
const { ethers } = require('ethers');

const WARNING_THRESHOLD = 10; // TODO: discuss for production
const TIME_SPLIT = 5 * 3600 * 1000; //  5 hours in milliseconds
const PAGER_DUTY_ALERT_URL = 'https://events.pagerduty.com/v2/enqueue';

const handleFailedTxFn = async (context, event) => {
    const chainName = context.metadata.getNetwork();
    const rpc = context.gateways.getGateway(chainName);
    const provider = new ethers.providers.JsonRpcProvider(rpc);
    const tx = await provider.getTransaction(event.hash);

    const response = await provider.call(tx, tx.blockNumber);
    const errorHash = response.slice(0, 10);
    console.log('errorHash: ', errorHash);
    let warningOptions = [];

    switch (errorHash) {
        case '0xdf359a15':
            warningOptions = ['FlowLimitExceeded', 'TokenManager'];
            break;
        // TODO: Cases will change after Role based actions are merged
        case '0x55f97efc':
            warningOptions = ['NotRemoteService', 'InterchainTokenService'];
            break;
        case '0x8e1bdb05':
            warningOptions = ['NotTokenManager', 'InterchainTokenervice'];
            break;
        case '0x7bed068d': // TODO: calculate correct hash
            warningOptions = ['NotCanonicalTokenManager', 'InterchainTokenService'];
            break;
        case '0xb078d99c':
            warningOptions = ['ReEntrancy', 'TokenManager'];
            break;
        case '0x76c6c93a':
            warningOptions = ['NotOperator', 'InterchainTokenService'];
            break;
        case '0x19cc87c1':
            warningOptions = ['NotProposedOperator', 'InterchainTokenService'];
            break;
        case '0x0d6c7be9':
            warningOptions = ['NotService', 'TokenManager'];
            break;
        default:
            console.log('No Error match found');
    }

    await sendWarning(event, context, chainName, ...warningOptions);

    const failedTxStartTime = await context.storage.getNumber('FailedTxStartTimestamp');
    let failedTxCount = await context.storage.getNumber('FailedTxCount');

    const timeNow = Date.now();

    if (timeNow - failedTxStartTime > TIME_SPLIT) {
        console.log('Updating Time stamp');
        failedTxCount = 1;
        await context.storage.putNumber('FailedTxStartTimestamp', timeNow);
    } else {
        failedTxCount++;

        if (failedTxCount % WARNING_THRESHOLD === 0) {
            await sendWarning(event, context, chainName, `Threshold crossed for failed transactions: ${failedTxCount}`, 'ITS_PROJECT');
        }
    }

    await context.storage.putNumber('FailedTxCount', failedTxCount);
};

async function sendWarning(event, context, chainName, summary, source) {
    try {
        const result = await axios.post(
            PAGER_DUTY_ALERT_URL,
            {
                routing_key: await context.secrets.get('PD_ROUTING_KEY'),
                event_action: 'trigger',
                payload: {
                    summary,
                    source,
                    severity: 'warning',
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
        console.log('Execution Successful: ', result.status);
    } catch (error) {
        console.log(error.response.status);
        console.log(error.response.data);
        throw Error('SENDING_ALERTS_FAILED');
    }
}

module.exports = { handleFailedTxFn };
