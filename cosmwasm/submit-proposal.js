'use strict';

require('dotenv').config();

const {
    prepareWallet,
    prepareClient,
    getChains,
    encodeStoreCodeProposal,
    encodeInstantiateProposal,
    encodeInstantiate2Proposal,
    submitProposal,
    makeInstantiateMsg,
    instantiate2AddressForProposal,
    governanceAddress,
} = require('./utils');
const { saveConfig, loadConfig, printInfo, prompt } = require('../evm/utils');

const { Command, Option } = require('commander');

const updateContractConfig = (contractConfig, chainConfig, key, value) => {
    if (chainConfig) {
        contractConfig[chainConfig.axelarId] = {
            ...contractConfig[chainConfig.axelarId],
            [key]: value,
        };
    } else {
        contractConfig[key] = value;
    }
};

const predictAndUpdateAddress = (client, contractConfig, chainConfig, options, contractName, chainName) => {
    return instantiate2AddressForProposal(client, contractConfig, options).then((contractAddress) => {
        updateContractConfig(contractConfig, chainConfig, 'address', contractAddress);

        printInfo(`Predicted address for ${chainName === 'none' ? '' : chainName.concat(' ')}${contractName}. Address`, contractAddress);
        return contractAddress;
    });
};

const storeCode = (client, wallet, config, options) => {
    const { contractName } = options;
    const {
        axelar: {
            contracts: { [contractName]: contractConfig },
        },
    } = config;

    const proposal = encodeStoreCodeProposal(options);

    if (prompt(`Proceed with proposal submission?`, options.yes)) {
        return Promise.resolve();
    }

    return submitProposal(client, wallet, config, options, proposal).then((proposalId) => {
        printInfo('Proposal submitted', proposalId);

        contractConfig.storeCodeProposalId = proposalId;
    });
};

const instantiate = (client, wallet, config, options, chainName) => {
    const { contractName, instantiate2, predictOnly } = options;
    const {
        axelar: {
            contracts: { [contractName]: contractConfig },
        },
        chains: { [chainName]: chainConfig },
    } = config;

    if (predictOnly) {
        return predictAndUpdateAddress(client, contractConfig, chainConfig, options, contractName, chainName);
    }

    const initMsg = makeInstantiateMsg(contractName, chainName, config);
    const proposal = instantiate2
        ? encodeInstantiate2Proposal(config, options, initMsg)
        : encodeInstantiateProposal(config, options, initMsg);

    if (prompt(`Proceed with proposal submission?`, options.yes)) {
        return Promise.resolve();
    }

    return submitProposal(client, wallet, config, options, proposal).then((proposalId) => {
        printInfo('Proposal submitted', proposalId);

        updateContractConfig(contractConfig, chainConfig, 'instantiateProposalId', proposalId);

        if (instantiate2) {
            return predictAndUpdateAddress(client, contractConfig, chainConfig, options, contractName, chainName);
        }
    });
};

const main = async (options) => {
    const { env, proposalType } = options;
    const config = loadConfig(env);

    await prepareWallet(options)
        .then((wallet) => prepareClient(config, wallet))
        .then(({ wallet, client }) => {
            switch (proposalType) {
                case 'store':
                    return storeCode(client, wallet, config, options);

                case 'instantiate': {
                    const chains = getChains(config, options);

                    return chains.reduce((promise, chain) => {
                        return promise.then(() => instantiate(client, wallet, config, options, chain.toLowerCase()));
                    }, Promise.resolve());
                }

                default:
                    throw new Error('Invalid proposal type');
            }
        })
        .then(() => saveConfig(config, env));
};

const programHandler = () => {
    const program = new Command();

    program.name('submit-proposal').description('Submit governance proposals');

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['local', 'devnet', 'devnet-amplifier', 'devnet-verifiers', 'stagenet', 'testnet', 'mainnet'])
            .default('devnet-amplifier')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(new Option('-m, --mnemonic <mnemonic>', 'mnemonic').makeOptionMandatory(true).env('MNEMONIC'));
    program.addOption(new Option('-a, --artifactPath <artifactPath>', 'artifact path').makeOptionMandatory(true).env('ARTIFACT_PATH'));
    program.addOption(new Option('-c, --contractName <contractName>', 'contract name').makeOptionMandatory(true));
    program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').default('none'));

    program.addOption(new Option('-s, --salt <salt>', 'salt for instantiate2. defaults to contract name').env('SALT'));
    program.addOption(
        new Option('--admin <address>', 'when instantiating contract, set an admin address. Defaults to governance module account').default(
            governanceAddress,
        ),
    );
    program.addOption(new Option('--instantiate2', 'use instantiate2 for constant address deployment'));
    program.addOption(new Option('--aarch64', 'aarch64').env('AARCH64').default(false));
    program.addOption(new Option('-y, --yes', 'skip prompt confirmation').env('YES'));

    program.addOption(new Option('-t, --title <title>', 'title of proposal').makeOptionMandatory(true));
    program.addOption(new Option('-d, --description <description>', 'description of proposal').makeOptionMandatory(true));
    program.addOption(new Option('--deposit <deposit>', 'the proposal deposit').makeOptionMandatory(true));
    program.addOption(new Option('-r, --runAs <runAs>', 'the address that will execute the message').makeOptionMandatory(true));
    program.addOption(
        new Option('--proposalType <proposalType>', 'proposal type').choices(['store', 'instantiate']).makeOptionMandatory(true),
    );
    program.addOption(new Option('--predictOnly', 'output the predicted changes only').env('PREDICT_ONLY'));

    program.addOption(new Option('--source <source>', "a valid HTTPS URI to the contract's source code"));
    program.addOption(
        new Option('--builder <builder>', 'a valid docker image name with tag, such as "cosmwasm/workspace-optimizer:0.16.0'),
    );
    program.addOption(
        new Option('-i, --instantiateAddresses <instantiateAddresses>', 'comma separated list of addresses allowed to instantiate'),
    );

    program.action((options) => {
        main(options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
