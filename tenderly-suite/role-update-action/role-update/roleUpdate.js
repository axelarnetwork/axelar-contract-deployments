const axios = require('axios').default;

const TOPIC_0_ROLES_ADDED = '0x34e73c57659d4b6809b53db4feee9b007b892e978114eda420d2991aba150143';
const TOPIC_0_ROLES_REMOVED = '0xccf920c8facee98a9c2a6c6124f2857b87b17e9f3a819bfcc6945196ee77366b';

const PAGER_DUTY_ALERT_URL = 'https://events.pagerduty.com/v2/enqueue';

const handleRoleUpdate = async (context, event) => {
    if (!event || !event.logs || !context || !context.metadata) {
        throw new Error('INVALID_INPUT_FOR_ACTION');
    }

    const chainName = context.metadata.getNetwork();

    const trustedAddresses = await context.storage.getJson('TrustedAddresses');

    const roleAddedAccounts = [];
    const addedRoles = [];
    const roleRemovedAccounts = [];
    const removedRoles = [];
    const summary = 'Roles updated';
    let severity = 0;

    for (const log of event.logs) {
        if (log.topics[0] === TOPIC_0_ROLES_ADDED || log.topics[0] === TOPIC_0_ROLES_REMOVED) {
            if (log.data.length <= 2) {
                throw new Error('EMPTY_LOG_DATA');
            }

            const roles = toRoleArray(parseInt(log.data, 16));

            if (log.topics.length === 0) {
                throw new Error('INVALID_LOGS_LENGTH');
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

function toRoleArray(accountRoles) {
    const roles = [];
    let bitIndex = 0;

    //  calculate uint8 array from uint256 value by bit shifting operation
    while (accountRoles > 0) {
        if (accountRoles & 1) {
            roles.push(getRole(bitIndex));
        }

        accountRoles >>= 1;
        bitIndex++;
    }

    return roles;
}

module.exports = { handleRoleUpdate };
