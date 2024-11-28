const { main: cosmwasmDeploy } = require('../../cosmwasm/deploy-contract');
const { deployAmplifierGateway } = require('../deploy-amplifier-gateway');
const { mainProcessor, printInfo, printError, printLog } = require('../utils');

const deployCosmWasmContract = async ({ contractName, chainName, salt, mnemonic, env, yes, codeId }) => {
    try {
        console.log(`Starting deployment for ${contractName} on ${chainName.name}`);
        await cosmwasmDeploy({
            contractName,
            chainName: chainName.axelarId,
            salt,
            mnemonic,
            env,
            yes,
            codeId,
        });
        printInfo(`Deployment successful for ${contractName} on ${chainName.name}`);
    } catch (error) {
        printError(`Error deploying ${contractName} on ${chainName.name}:`, error);
        throw error;
    }
};

const deployEvmContract = async (config, chainName, { salt, env, yes, privateKey }, predict) => {
    try {
        printLog(`Starting deployment for Ext. Gateway on ${chainName.name}`);
        const gateway = await deployAmplifierGateway(config, chainName, {
            salt,
            env,
            yes,
            privateKey,
            deployMethod: 'create3',
            previousSignersRetention: 15,
            minimumRotationDelay: 86400,
            predictOnly: predict,
            deployMethod: 'create3',
        });
        printInfo(`Deployment successful for Gateway on ${chainName.name}`);
        return gateway;
    } catch (error) {
        printError(`Error deploying Gateway on ${chainName.name}:`, error);
        throw error;
    }
};

module.exports = {
    deployCosmWasmContract,
    deployEvmContract,
};
