const devnet = require('../axelar-chains-config/info/devnet-amplifier.json');

function extractAddresses(data) {
    const result = {};

    for (const chain in data.chains) {
        const chainData = data.chains[chain];
        const contracts = chainData.contracts;

        const filteredContracts = {};

        for (const contract in contracts) {
            if (contracts[contract].address) {
                filteredContracts[contract] = { address: contracts[contract].address };
            }
        }

        result[chain] = {
            name: chainData.name,
            id: chainData.id,
            axelarId: chainData.axelarId,
            chainId: chainData.chainId,
            rpc: chainData.rpc,
            tokenSymbol: chainData.tokenSymbol,
            confirmations: chainData.confirmations,
            chainType: chainData.chainType,
            contracts: filteredContracts,
        };
    }

    const amplifierContracts = data.axelar.contracts;
    const { MultisigProver } = amplifierContracts;

    Object.keys(MultisigProver).forEach((axelarChainId) => {
        if (result[axelarChainId]) {
            const chainContracts = {};
            chainContracts.MultisigProver = { address: amplifierContracts.MultisigProver[axelarChainId].address };
            chainContracts.VotingVerifier = { address: amplifierContracts.VotingVerifier[axelarChainId].address };
            chainContracts.Gateway = { address: amplifierContracts.Gateway[axelarChainId].address };
            result[axelarChainId].amplifierContracts = chainContracts;
        }
    });

    return result;
}

const result = extractAddresses(devnet);

console.log(JSON.stringify(result, null, 2));
