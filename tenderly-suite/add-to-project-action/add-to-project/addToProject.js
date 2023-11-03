const axios = require('axios').default;

const TOKEN_MANAGER_DEPLOYED_TOPIC0 = '0x5284c2478b9c1a55e973429331078be39b5fb3eeb9d87d10b34d65a4c89ee4eb';

const addToProjectFn = async (context, event) => {
    if (!event || !event.logs || !context || !context.metadata) {
        throw new Error('INVALID_INPUT_FOR_ACTION');
    }

    const logs = event.logs;
    const contracts = [];

    for (let index = 0; index < logs.length; ++index) {
        if (logs[index].topics[0] === TOKEN_MANAGER_DEPLOYED_TOPIC0) {
            if (logs[index].data.length < 66) {
                throw new Error('INVALID_LOG_DATA_LENGTH');
            }

            // log data contains address in first 32 bytes i.e. first 64 chars, here data string is also prefixed with 0x.
            // data = '0x' + 24 chars (appended 0) + 40 chars (address)
            const deployedAddress = '0x' + logs[index].data.substring(26, 66);
            const name = `TokenManager-${context.metadata.getNetwork()}-${deployedAddress}`;

            contracts.push({
                address: deployedAddress,
                display_name: name,
                network_id: event.network,
            });
        }
    }

    if (contracts.length === 0) throw Error('NO_NEW_TOKEN_MANAGER_DEPLOYED');

    try {
        await axios.post(
            await context.storage.getStr('TenderlyAddContractsURL'),
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
