'use strict';

require('dotenv').config();

const { instantiate2Address } = require('@cosmjs/cosmwasm-stargate');

const { printInfo, loadConfig, saveConfig, prompt } = require('../common');
const {
    CONTRACTS,
    prepareWallet,
    prepareClient,
    fromHex,
    getSalt,
    initContractConfig,
    getAmplifierContractConfig,
    getCodeId,
    uploadContract,
    instantiateContract,
} = require('./utils');

const { Command } = require('commander');
const { addAmplifierOptions } = require('./cli-utils');

const upload = async (client, wallet, config, options) => {
    const { contractName, instantiate2, salt, chainName } = options;
    const { contractBaseConfig, contractConfig } = getAmplifierContractConfig(config, options);

    printInfo('Uploading contract binary');

    const { checksum, codeId } = await uploadContract(client, wallet, config, options);

    printInfo('Uploaded contract binary with codeId', codeId);
    contractBaseConfig.lastUploadedCodeId = codeId;

    if (instantiate2) {
        const [account] = await wallet.getAccounts();
        const address = instantiate2Address(fromHex(checksum), account.address, getSalt(salt, contractName, chainName), 'axelar');

        contractConfig.address = address;

        printInfo('Expected contract address', address);
    }
};

const instantiate = async (client, wallet, config, options) => {
    const { contractName, chainName, yes } = options;

    const { contractConfig } = getAmplifierContractConfig(config, options);

    const codeId = await getCodeId(client, config, options);
    printInfo('Using code id', codeId);

    if (prompt(`Proceed with instantiation on axelar?`, yes)) {
        return;
    }

    contractConfig.codeId = codeId;

    const initMsg = CONTRACTS[contractName].makeInstantiateMsg(config, options, contractConfig);
    const contractAddress = await instantiateContract(client, wallet, initMsg, config, options);

    contractConfig.address = contractAddress;

    printInfo(`Instantiated ${chainName ? chainName.concat(' ') : ''}${contractName}. Address`, contractAddress);
};

const uploadInstantiate = async (client, wallet, config, options) => {
    await upload(client, wallet, config, options);
    await instantiate(client, wallet, config, options);
};

const mainProcessor = async (processor, options) => {
    const { env } = options;
    const config = loadConfig(env);

    initContractConfig(config, options);

    const wallet = await prepareWallet(options);
    const client = await prepareClient(config, wallet);

    await processor(client, wallet, config, options);

    saveConfig(config, env);
};

const programHandler = () => {
    const program = new Command();

    program.name('deploy-contract').description('Deploy CosmWasm contracts');

    const uploadCmd = program
        .command('upload')
        .description('Upload wasm binary')
        .action((options) => {
            mainProcessor(upload, options);
        });
    addAmplifierOptions(uploadCmd, {
        contractOptions: true,
        storeOptions: true,
        instantiate2Options: true,
    });

    const instantiateCmd = program
        .command('instantiate')
        .description('Instantiate contract')
        .action((options) => {
            mainProcessor(instantiate, options);
        });
    addAmplifierOptions(instantiateCmd, {
        contractOptions: true,
        instantiateOptions: true,
        instantiate2Options: true,
        codeId: true,
        fetchCodeId: true,
    });

    const uploadInstantiateCmd = program
        .command('upload-instantiate')
        .description('Upload wasm binary and instantiate contract')
        .action((options) => {
            mainProcessor(uploadInstantiate, options);
        });
    addAmplifierOptions(uploadInstantiateCmd, {
        contractOptions: true,
        storeOptions: true,
        instantiateOptions: true,
        instantiate2Options: true,
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
