'use strict';

require('dotenv').config();
const { isNil } = require('lodash');

const { instantiate2Address } = require('@cosmjs/cosmwasm-stargate');

const { printInfo, loadConfig, saveConfig, prompt } = require('../common');
const {
    prepareWallet,
    prepareClient,
    fromHex,
    getSalt,
    initContractConfig,
    getAmplifierContractConfig,
    uploadContract,
    instantiateContract,
    makeInstantiateMsg,
} = require('./utils');

const { Command, Option } = require('commander');
const { addAmplifierOptions } = require('./cli-utils');

const upload = async (client, wallet, config, options) => {
    const { reuseCodeId, contractName, fetchCodeId, instantiate2, salt, chainName } = options;

    const { contractBaseConfig, contractConfig } = getAmplifierContractConfig(config, options);

    if (!fetchCodeId && (!reuseCodeId || isNil(contractBaseConfig.lastUploadedCodeId))) {
        printInfo('Uploading contract binary');

        const { checksum, codeId } = await uploadContract(client, wallet, config, options);

        printInfo('Uploaded contract binary');
        contractBaseConfig.lastUploadedCodeId = codeId;

        if (instantiate2) {
            const [account] = await wallet.getAccounts();
            const address = instantiate2Address(fromHex(checksum), account.address, getSalt(salt, contractName, chainName), 'axelar');

            contractConfig.address = address;

            printInfo('Expected contract address', address);
        }
    } else {
        printInfo('Skipping upload. Reusing previously uploaded bytecode');
    }
};

const instantiate = async (client, wallet, config, options) => {
    const { contractName, chainName } = options;

    const { contractConfig } = getAmplifierContractConfig(config, options);

    await updateCodeId(config, options);

    const initMsg = makeInstantiateMsg(contractName, chainName, config);
    const contractAddress = await instantiateContract(client, wallet, initMsg, config, options);

    contractConfig.address = contractAddress;

    printInfo(`Instantiated ${chainName ? chainName.concat(' ') : ''}${contractName}. Address`, contractAddress);
};

const main = async (options) => {
    const { env, uploadOnly, yes } = options;
    const config = loadConfig(env);

    initContractConfig(config, options);

    const wallet = await prepareWallet(options);
    const client = await prepareClient(config, wallet);

    await upload(client, wallet, config, options);

    if (!(uploadOnly || prompt(`Proceed with deployment on axelar?`, yes))) {
        await instantiate(client, wallet, config, options);
    }

    saveConfig(config, env);
};

const programHandler = () => {
    const program = new Command();

    program.name('upload-contract').description('Upload CosmWasm contracts');

    addAmplifierOptions(program, {
        contractOptions: true,
        storeOptions: true,
        instantiateOptions: true,
        instantiate2Options: true,
        codeId: true,
        fetchCodeId: true,
    });

    program.addOption(new Option('-r, --reuseCodeId', 'reuse code Id'));
    program.addOption(
        new Option(
            '-u, --uploadOnly',
            'upload the contract without instantiating. prints expected contract address if --instantiate2 is passed',
        ),
    );

    program.action((options) => {
        main(options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
