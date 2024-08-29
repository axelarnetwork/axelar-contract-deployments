'use strict';

require('dotenv').config();
const { isNil } = require('lodash');

const { instantiate2Address } = require('@cosmjs/cosmwasm-stargate');

const { isNumber, printInfo, loadConfig, saveConfig, prompt, getChainConfig } = require('../common');
const {
    prepareWallet,
    prepareClient,
    fromHex,
    getSalt,
    getChains,
    updateContractConfig,
    fetchCodeIdFromCodeHash,
    uploadContract,
    instantiateContract,
    makeInstantiateMsg,
} = require('./utils');

const { Command, Option } = require('commander');
const { addAmplifierOptions } = require('./cli-utils');

const upload = async (client, wallet, chainName, config, options) => {
    const { reuseCodeId, contractName, fetchCodeId, instantiate2, salt, chainNames } = options;
    const {
        axelar: {
            contracts: { [contractName]: contractConfig },
        },
    } = config;
    const chainConfig = chainName === 'none' ? undefined : getChainConfig(config, chainName);

    if (!fetchCodeId && (!reuseCodeId || isNil(contractConfig.codeId))) {
        printInfo('Uploading contract binary');

        const { checksum, codeId } = await uploadContract(client, wallet, config, options);

        printInfo('Uploaded contract binary');
        contractConfig.codeId = codeId;

        if (instantiate2) {
            const [account] = await wallet.getAccounts();
            const address = instantiate2Address(fromHex(checksum), account.address, getSalt(salt, contractName, chainNames), 'axelar');

            updateContractConfig(contractConfig, chainConfig, 'address', address);

            printInfo('Expected contract address', address);
        }
    } else {
        printInfo('Skipping upload. Reusing previously uploaded bytecode');
    }
};

const instantiate = async (client, wallet, chainName, config, options) => {
    const { contractName, fetchCodeId } = options;
    const {
        axelar: {
            contracts: { [contractName]: contractConfig },
        },
        chains: { [chainName]: chainConfig },
    } = config;

    if (fetchCodeId) {
        contractConfig.codeId = await fetchCodeIdFromCodeHash(client, contractConfig);
    } else if (!isNumber(contractConfig.codeId)) {
        throw new Error('Code Id is not defined');
    }

    const initMsg = makeInstantiateMsg(contractName, chainName, config);
    const contractAddress = await instantiateContract(client, wallet, initMsg, config, options);

    updateContractConfig(contractConfig, chainConfig, 'address', contractAddress);

    printInfo(`Instantiated ${chainName === 'none' ? '' : chainName.concat(' ')}${contractName}. Address`, contractAddress);
};

const main = async (options) => {
    const { env, uploadOnly, yes } = options;
    const config = loadConfig(env);

    const chains = getChains(config, options);

    const wallet = await prepareWallet(options);
    const client = await prepareClient(config, wallet);

    await upload(client, wallet, chains[0], config, options);

    if (uploadOnly || prompt(`Proceed with deployment on axelar?`, yes)) {
        return;
    }

    for (const chain of chains) {
        await instantiate(client, wallet, chain.toLowerCase(), config, options);
    }

    saveConfig(config, env);
};

const programHandler = () => {
    const program = new Command();

    program.name('upload-contract').description('Upload CosmWasm contracts');

    addAmplifierOptions(program);

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
