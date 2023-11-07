const axios = require('axios').default;
const { ethers } = require('ethers');

const PAGER_DUTY_ALERT_URL = 'https://events.pagerduty.com/v2/enqueue';
const Severity = {
    INFO: 'info',
    CRITICAL: 'critical',
    WARNING: 'warning',
};

const handleFailedTxFn = async (context, event) => {
    if (!event || !context || !context.metadata) {
        throw new Error('INVALID_INPUT_FOR_ACTION');
    }

    const chainName = context.metadata.getNetwork();
    const provider = new ethers.providers.JsonRpcProvider(await context.secrets.get(`RPC_${chainName.toUpperCase()}`));
    const tx = await provider.getTransaction(event.hash);
    const blockNumberLatest = await provider.getBlockNumber();
    console.log('latestBlockNumber: ', blockNumberLatest);

    const warningThreshold = await context.storage.getStr('WarningThreshold');
    const criticalThreshold = await context.storage.getStr('CriticalThreshold');
    const timeSplit = await context.storage.getStr('TimeSplit');

    if (!tx.to || !tx.from || !tx.blockNumber || !tx.data) {
        throw new Error('INVALID_TX_FORMAT');
    }

    const { to, from, nonce, gasLimit, gasPrice, data, value, chainId, type, accessList, blockNumber } = tx;

    const response = await provider.call(
        {
            to,
            from,
            nonce,
            gasLimit,
            gasPrice,
            data,
            value,
            chainId,
            type: type ?? undefined,
            accessList,
        },
        blockNumber,
    );

    if (response.length < 10) {
        throw new Error('INVALID_RESPONSE_LENGTH');
    }

    const { flowLimitExceeded, missingRole, missingAllRoles, missingAnyOfRoles, reEntrancy, notService } = await context.storage.getJson(
        'ErrorsABI',
    );
    const flowLimitExceededHash = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(flowLimitExceeded)).slice(0, 10);
    const missingRoleHash = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(missingRole)).slice(0, 10);
    const missingAllRolesHash = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(missingAllRoles)).slice(0, 10);
    const missingAnyOfRolesHash = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(missingAnyOfRoles)).slice(0, 10);
    const reEntrancyHash = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(reEntrancy)).slice(0, 10);
    const notServiceHash = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(notService)).slice(0, 10);

    const errorHash = response.slice(0, 10);
    console.log('errorHash: ', errorHash);
    let warningOptions = [];

    switch (errorHash) {
        case flowLimitExceededHash:
            warningOptions = ['FlowLimitExceeded', 'TokenManager'];
            break;
        case missingRoleHash:
            warningOptions = ['MissingRole', event.to];
            break;
        case missingAllRolesHash:
            warningOptions = ['MissingAllRoles', event.to];
            break;
        case missingAnyOfRolesHash:
            warningOptions = ['MissingAnyOfRoles', event.to];
            break;
        case reEntrancyHash:
            warningOptions = ['ReEntrancy', event.to];
            break;
        case notServiceHash:
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
