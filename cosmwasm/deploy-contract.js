'use strict';

require('dotenv').config();
const { isNil } = require('lodash');

const { CHAIN_ENVIRONMENTS } = require('../common');
const { isNumber, printInfo, loadConfig, saveConfig, prompt } = require('../evm/utils');
const {
    prepareWallet,
    prepareClient,
    getChains,
    fetchCodeIdFromCodeHash,
    uploadContract,
    instantiateContract,
    makeInstantiateMsg,
    governanceAddress,
} = require('./utils');

const { Command, Option } = require('commander');

const upload = (client, wallet, chainName, config, options) => {
    const { reuseCodeId, contractName, fetchCodeId } = options;
    const {
        axelar: {
            contracts: { [contractName]: contractConfig },
        },
        chains: { [chainName]: chainConfig },
    } = config;

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

    printInfo('Skipping upload. Reusing previously uploaded binary');
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

    program.addOption(
        new Option('-e, --env <env>', 'environment').choices(CHAIN_ENVIRONMENTS).default('testnet').makeOptionMandatory(true).env('ENV'),
    );
    program.addOption(new Option('-m, --mnemonic <mnemonic>', 'mnemonic').makeOptionMandatory(true).env('MNEMONIC'));
    program.addOption(new Option('-a, --artifactPath <artifactPath>', 'artifact path').makeOptionMandatory(true).env('ARTIFACT_PATH'));
    program.addOption(new Option('-c, --contractName <contractName>', 'contract name').makeOptionMandatory(true));
    program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').default('none'));
    program.addOption(new Option('-r, --reuseCodeId', 'reuse code Id'));
    program.addOption(new Option('-s, --salt <salt>', 'salt for instantiate2. defaults to contract name').env('SALT'));
    program.addOption(
        new Option('--admin <address>', 'when instantiating contract, set an admin address. Defaults to governance module account').default(
            governanceAddress,
        ),
    );
    program.addOption(
        new Option(
            '-u, --uploadOnly',
            'upload the contract without instantiating. prints expected contract address if --instantiate2 is passed',
        ),
    );
    program.addOption(new Option('--instantiate2', 'use instantiate2 for constant address deployment'));
    program.addOption(new Option('-l, --label <label>', 'contract instance label'));
    program.addOption(new Option('--aarch64', 'aarch64').env('AARCH64').default(false));
    program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));

    program.addOption(new Option('--fetchCodeId', 'fetch code id from the chain by comparing to the uploaded code hash'));

    program.action((options) => {
        main(options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
