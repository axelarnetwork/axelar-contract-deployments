'use strict';

require('dotenv').config();
const { isNil } = require('lodash');

const { isNumber, printInfo, loadConfig, saveConfig, prompt, getChainConfig } = require('../common');
const {
    prepareWallet,
    prepareClient,
    getChains,
    fetchCodeIdFromCodeHash,
    uploadContract,
    instantiateContract,
    makeInstantiateMsg,
} = require('./utils');

const { Command, Option } = require('commander');
const { addAmplifierOptions } = require('./cli-utils');

const upload = (client, wallet, chainName, config, options) => {
    const { reuseCodeId, contractName, fetchCodeId } = options;
    const {
        axelar: {
            contracts: { [contractName]: contractConfig },
        },
    } = config;
    const chainConfig = chainName === 'none' ? undefined : getChainConfig(config, chainName);

    if (!fetchCodeId && (!reuseCodeId || isNil(contractConfig.codeId))) {
        printInfo('Uploading contract binary');

        return uploadContract(client, wallet, config, options)
            .then(({ address, codeId }) => {
                printInfo('Uploaded contract binary');
                contractConfig.codeId = codeId;

                if (!address) {
                    return;
                }

                if (chainConfig) {
                    contractConfig[chainConfig.axelarId] = {
                        ...contractConfig[chainConfig.axelarId],
                        address,
                    };
                } else {
                    contractConfig.address = address;
                }

                printInfo('Expected contract address', address);
            })
            .then(() => ({ wallet, client }));
    }

    printInfo('Skipping upload. Reusing previously uploaded bytecode');
    return Promise.resolve({ wallet, client });
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
    return instantiateContract(client, wallet, initMsg, config, options).then((contractAddress) => {
        if (chainConfig) {
            contractConfig[chainConfig.axelarId] = {
                ...contractConfig[chainConfig.axelarId],
                address: contractAddress,
            };
        } else {
            contractConfig.address = contractAddress;
        }

        printInfo(`Instantiated ${chainName === 'none' ? '' : chainName.concat(' ')}${contractName}. Address`, contractAddress);
    });
};

const main = async (options) => {
    const { env, uploadOnly, yes } = options;
    const config = loadConfig(env);

    const chains = getChains(config, options);

    await prepareWallet(options)
        .then((wallet) => prepareClient(config, wallet))
        .then(({ wallet, client }) => upload(client, wallet, chains[0], config, options))
        .then(({ wallet, client }) => {
            if (uploadOnly || prompt(`Proceed with deployment on axelar?`, yes)) {
                return;
            }

            return chains.reduce((promise, chain) => {
                return promise.then(() => instantiate(client, wallet, chain.toLowerCase(), config, options));
            }, Promise.resolve());
        })
        .then(() => saveConfig(config, env));
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
