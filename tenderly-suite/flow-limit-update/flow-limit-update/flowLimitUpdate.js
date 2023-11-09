const axios = require('axios').default;
const { ethers } = require('ethers');

const PAGER_DUTY_ALERT_URL = 'https://events.pagerduty.com/v2/enqueue';
const Severity = {
    INFO: 'info',
    CRITICAL: 'critical',
    WARNING: 'warning',
    1: 'info',
    2: 'warning',
};

const flowLimitUpdateFn = async (context, event) => {
    if (!event || !event.logs || !context || !context.metadata) {
        throw new Error('INVALID_INPUT_FOR_ACTION');
    }

    const { flowLimitSet } = await context.storage.getJson('EventsABI');
    const flowLimitSetHash = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(flowLimitSet));

    const itsAddresses = await context.storage.getJson('ITSAddresses');

    let severity = 0;
    const tokenIDs = [];
    const tokenManagers = [];
    const operators = [];

    for (const log of event.logs) {
        if (log.topics[0] === flowLimitSetHash) {
            console.log(`event emitted: ${flowLimitSet}`);

            if (log.topics.length < 2) {
                throw new Error('INVALID_LOG_TOPICS_LENGTH');
            }

            if (log.data.length < 66) {
                throw new Error('INVALID_LOG_DATA_LENGTH');
            }

            //  log data contains address in first 32 bytes i.e. first 64 chars, here data string is also prefixed with 0x.
            const operatorAddress = '0x' + log.data.substring(26, 66);
            const tempSeverity = itsAddresses.includes(operatorAddress.toLowerCase()) ? 1 : 2;

            if (tempSeverity > severity) {
                severity = tempSeverity;
            }

            tokenIDs.push(log.topics[1]);
            tokenManagers.push(log.address);
            operators.push(operatorAddress);
        }
    }

    if (severity) {
        try {
            await axios.post(
                PAGER_DUTY_ALERT_URL,
                {
                    routing_key: await context.secrets.get('PD_ROUTING_KEY'),
                    event_action: 'trigger',
                    payload: {
                        summary: 'Flow limit updated',
                        source: 'ITS',
                        severity: Severity[severity],
                        custom_details: {
                            timestamp: Date.now(),
                            chain_name: context.metadata.getNetwork(),
                            trigger_event: event,
                            token_ids: tokenIDs,
                            token_managers: tokenManagers,
                            operators,
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
    } else {
        throw new Error('NO_FLOW_LIMIT_UPDATES_DETECTED');
    }
};

module.exports = { flowLimitUpdateFn };
