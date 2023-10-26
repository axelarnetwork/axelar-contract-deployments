const axios = require('axios').default;

const TOPIC_0_ROLES_ADDED = '0x7d84b89a03a5eeb4f4c08928d186adfdee2af1ef0a026ef9b04fe688774e8463';
const TOPIC_0_ROLES_REMOVED = '0xfcacbe9899a5563ab47d70de5417c1cb8e648bc9ccaeea6d8e0f5276087394c3';

const PAGER_DUTY_ALERT_URL = 'https://events.pagerduty.com/v2/enqueue';

const handleRoleUpdate = async (context, event) => {
    const chainName = context.metadata.getNetwork();

    for (const log of event.logs) {
        if (log.topics[0] === TOPIC_0_ROLES_ADDED || log.topics[0] === TOPIC_0_ROLES_REMOVED) {
            const length = parseInt(log.data.substring(128, 130), 16);
            const roles = [];

            for (let index = 0; index < length; index++) {
                const subIndex = 64 * (3 + index) + 2;
                roles.push(getRole(parseInt(log.data.substring(subIndex - 2, subIndex), 16)));
            }

            const summary = log.topics[0] === TOPIC_0_ROLES_ADDED ? 'Roles added' : 'Roles removed';
            const account = `0x${log.topics[1].substring(26, 26 + 40)}`;

            try {
                await axios.post(
                    PAGER_DUTY_ALERT_URL,
                    {
                        routing_key: await context.secrets.get('PD_ROUTING_KEY'),
                        event_action: 'trigger',
                        payload: {
                            summary,
                            source: `${chainName}-${log.address}`,
                            severity: 'warning',
                            custom_details: {
                                timestamp: Date.now(),
                                chain_name: chainName,
                                trigger_event: event,
                                account,
                                roles,
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
    }
};

function getRole(roleId) {
    if (roleId === 0) {
        return 'Distributor';
    } else if (roleId === 1) {
        return 'Operator';
    } else if (roleId === 2) {
        return 'FlowLimiter';
    }

    return '-';
}

module.exports = { handleRoleUpdate };
