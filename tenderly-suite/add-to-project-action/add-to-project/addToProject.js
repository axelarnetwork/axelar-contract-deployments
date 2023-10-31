const axios = require('axios').default;

const URL = 'https://api.tenderly.co/api/v2/accounts/axelarEng/projects/ITS/contracts';
const TOKEN_MANAGER_DEPLOYED_TOPIC0 = '0x5284c2478b9c1a55e973429331078be39b5fb3eeb9d87d10b34d65a4c89ee4eb';

const addToProjectFn = async (context, event) => {
    const logs = event.logs;
    const contracts = [];

    for (let index = 0; index < logs.length; ++index) {
        if (logs[index].topics[0] === TOKEN_MANAGER_DEPLOYED_TOPIC0) {
            const deployedAddress = '0x' + logs[index].data.substring(26, 66);
            const name = `TokenManager-${context.metadata.getNetwork()}-${deployedAddress}`; // TokenManager + network + address

            contracts.push({
                address: deployedAddress,
                display_name: name,
                network_id: event.network,
            });
        }
    }

    if (contracts.length === 0) throw Error('NO_DEPLOYED_CONTRACT_FOUND');

    try {
        await axios.post(
            URL,
            { contracts },
            {
                headers: {
                    'X-Access-Key': await context.secrets.get('API_TOKEN'),
                },
            },
        );
    } catch (error) {
        console.log(error.response.status);
        console.error(error.response.data);
        throw Error('CONTRACT_ADDITION_FAILED');
    }
};

module.exports = { addToProjectFn };
