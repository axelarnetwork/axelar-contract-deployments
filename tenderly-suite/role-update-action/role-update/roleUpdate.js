const axios = require('axios').default;

const TOPIC_0_ROLES_ADDED = '0x3df2f62906643352cfb366ada865850a1f2127a98c97ac962921d5caf75561c3';
const TOPIC_0_ROLES_REMOVED = '0x17e90d13bc6dcdbe950d3d022f0774c9dfa3308b96720b8779075dd83236061f';

const PAGER_DUTY_ALERT_URL = 'https://events.pagerduty.com/v2/enqueue';

const TRUSTED_ADDRESSES = [
    '0x4a6eea0999b000a941926e298f7a49373c153fbc', // TODO: Update contracts list upon deployment
    '0xf1ea5615086a0936f82656f88263365831978f71',
    '0xb8cd93c83a974649d76b1c19f311f639e62272bc',
];

const handleRoleUpdate = async (context, event) => {
    const chainName = context.metadata.getNetwork();

    const roleAddedAccounts = [];
    const addedRoles = [];
    const roleRemovedAccounts = [];
    const removedRoles = [];
    const summary = 'Roles updated';
    let severity = 0;

    for (const log of event.logs) {
        if (log.topics[0] === TOPIC_0_ROLES_ADDED || log.topics[0] === TOPIC_0_ROLES_REMOVED) {
            console.log('log Found');
            const length = parseInt(log.data.substring(128, 130), 16);
            const roles = [];

            for (let index = 0; index < length; index++) {
                const subIndex = 64 * (3 + index) + 2;
                roles.push(getRole(parseInt(log.data.substring(subIndex - 2, subIndex), 16)));
            }

            const account = `0x${log.topics[1].substring(26, 26 + 40)}`;
            const tempSeverity = TRUSTED_ADDRESSES.includes(account.toLowerCase()) ? 1 : 2;

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

    return '-';
}

module.exports = { handleRoleUpdate };
