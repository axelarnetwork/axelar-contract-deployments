const axios = require('axios').default;

const TOPIC_0_ROLES_ADDED = '0x3df2f62906643352cfb366ada865850a1f2127a98c97ac962921d5caf75561c3';
const TOPIC_0_ROLES_REMOVED = '0x17e90d13bc6dcdbe950d3d022f0774c9dfa3308b96720b8779075dd83236061f';

const PAGER_DUTY_ALERT_URL = 'https://events.pagerduty.com/v2/enqueue';

const handleRoleUpdate = async (context, event) => {
    const chainName = context.metadata.getNetwork();

    if (!event || !event.logs || !context || !context.metadata) {
        throw new Error('INVALID_INPUT_FOR_ACTION');
    }

    const trustedAddresses = await context.storage.getJson('TrustedAddresses');

    const roleAddedAccounts = [];
    const addedRoles = [];
    const roleRemovedAccounts = [];
    const removedRoles = [];
    const summary = 'Roles updated';
    let severity = 0;

    for (const log of event.logs) {
        if (log.topics[0] === TOPIC_0_ROLES_ADDED || log.topics[0] === TOPIC_0_ROLES_REMOVED) {
            if (log.data.length < 130) {
                throw new Error('INVALID_LOG_DATA_LENGTH');
            }

            const length = parseInt(log.data.substring(128, 130), 16);
            const roles = [];

            for (let index = 0; index < length; index++) {
                const subIndex = 64 * (3 + index) + 2;
                roles.push(getRole(parseInt(log.data.substring(subIndex - 2, subIndex), 16)));
            }

            if (log.topics.length === 0) {
                throw new Error('INVALID_LOG_TOPICS_LENGTH');
            }

            if (log.topics[1].length < 26 + 40) {
                throw new Error('INVALID_LOG_TOPIC_LENGTH');
            }

            //  account is present in log topic as 32 bytes hex string, with prefixed 0s
            const account = `0x${log.topics[1].substring(26, 26 + 40)}`;
            const tempSeverity = trustedAddresses.includes(account.toLowerCase()) ? 1 : 2;

            if (log.topics[0] === TOPIC_0_ROLES_ADDED) {
                roleAddedAccounts.push(account);
                addedRoles.push(roles);
            } else {
                roleRemovedAccounts.push(account);
                removedRoles.push(roles);
            }

            if (tempSeverity > severity) {
                severity = tempSeverity;
            }
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
                        summary,
                        source: `${chainName}-${event.hash}`,
                        severity: severity === 2 ? 'warning' : 'info',
                        custom_details: {
                            timestamp: Date.now(),
                            chain_name: chainName,
                            trigger_event: event,
                            rolesUpdated: {
                                roleAddedAccounts,
                                addedRoles,
                                roleRemovedAccounts,
                                removedRoles,
                            },
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
        throw new Error('NO_ROLE_UPDATES_DETECTED');
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

    throw new Error('UNKNOWN_ROLE_UPDATED');
}

module.exports = { handleRoleUpdate };
