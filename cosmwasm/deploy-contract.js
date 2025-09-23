'use strict';

require('../common/cli-utils');

const { instantiate2Address } = require('@cosmjs/cosmwasm-stargate');

const { printInfo, prompt } = require('../common');

const {
    CONTRACTS,
    fromHex,
    getSalt,
    getAmplifierContractConfig,
    getCodeId,
    uploadContract,
    instantiateContract,
    migrateContract,
} = require('./utils');

const { mainProcessor } = require('./processor');

const { Command } = require('commander');
const { addAmplifierOptions } = require('./cli-utils');

const upload = async (client, config, options) => {
    const { contractName, instantiate2, salt, chainName } = options;
    const { contractBaseConfig, contractConfig } = getAmplifierContractConfig(config, options);

    printInfo('Uploading contract binary');
    const { checksum, codeId } = await uploadContract(client, config, options);

    printInfo('Uploaded contract binary with codeId', codeId);
    contractBaseConfig.lastUploadedCodeId = codeId;

    if (instantiate2) {
        const [account] = client.accounts;
        const address = instantiate2Address(fromHex(checksum), account.address, getSalt(salt, contractName, chainName), 'axelar');

        contractConfig.address = address;

        printInfo('Expected contract address', address);
    }
};

const instantiate = async (client, config, options) => {
    const { contractName, chainName, yes } = options;

    const { contractConfig } = getAmplifierContractConfig(config, options);

    const codeId = await getCodeId(client, config, options);
    printInfo('Using code id', codeId);

    if (prompt(`Proceed with instantiation on axelar?`, yes)) {
        return;
    }

    contractConfig.codeId = codeId;

    const initMsg = await CONTRACTS[contractName].makeInstantiateMsg(config, options, contractConfig);
    const contractAddress = await instantiateContract(client, initMsg, config, options);

    contractConfig.address = contractAddress;

    printInfo(`Instantiated ${chainName ? chainName.concat(' ') : ''}${contractName}. Address`, contractAddress);
};

const uploadInstantiate = async (client, config, options) => {
    await upload(client, config, options);
    await instantiate(client, config, options);
};

const migrate = async (client, config, options) => {
    const { yes } = options;
    const { contractConfig } = getAmplifierContractConfig(config, options);

    const codeId = await getCodeId(client, config, options);
    printInfo('Using code id', codeId);

    if (prompt(`Proceed with contract migration on axelar?`, yes)) {
        return;
    }

    contractConfig.codeId = codeId;

    const { transactionHash } = await migrateContract(client, config, options);
    printInfo('Migration completed. Transaction hash', transactionHash);
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

    const migrateCmd = program
        .command('migrate')
        .description('Migrate contract')
        .action((options) => {
            mainProcessor(migrate, options);
        });
    addAmplifierOptions(migrateCmd, {
        contractOptions: true,
        migrateOptions: true,
        codeId: true,
        fetchCodeId: true,
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
