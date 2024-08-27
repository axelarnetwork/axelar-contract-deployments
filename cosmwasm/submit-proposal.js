'use strict';

require('dotenv').config();

const { createHash } = require('crypto');

const {
    prepareWallet,
    prepareClient,
    readWasmFile,
    getChains,
    fetchCodeIdFromCodeHash,
    decodeProposalAttributes,
    encodeStoreCodeProposal,
    encodeStoreInstantiateProposal,
    encodeInstantiateProposal,
    encodeInstantiate2Proposal,
    encodeExecuteContractProposal,
    submitProposal,
    makeInstantiateMsg,
    instantiate2AddressForProposal,
    governanceAddress,
} = require('./utils');
const { isNumber, saveConfig, loadConfig, printInfo, prompt } = require('../evm/utils');
const { addEnvOption } = require('../common');
const {
    StoreCodeProposal,
    StoreAndInstantiateContractProposal,
    InstantiateContractProposal,
    InstantiateContract2Proposal,
    ExecuteContractProposal,
} = require('cosmjs-types/cosmwasm/wasm/v1/proposal');

const { Command, Option } = require('commander');
const { addCommonAmplifierOptions } = require('./cli-utils');

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

        return contractAddress;
    });
};

const printProposal = (proposal, proposalType) => {
    printInfo(
        `Encoded ${proposal.typeUrl}`,
        JSON.stringify(decodeProposalAttributes(proposalType.toJSON(proposalType.decode(proposal.value))), null, 2),
    );
};

const storeCode = (client, wallet, config, options) => {
    const { contractName } = options;
    const {
        axelar: {
            contracts: { [contractName]: contractConfig },
        },
    } = config;

    const proposal = encodeStoreCodeProposal(options);

    printProposal(proposal, StoreCodeProposal);

    if (prompt(`Proceed with proposal submission?`, options.yes)) {
        return Promise.resolve();
    }

    return submitProposal(client, wallet, config, options, proposal).then((proposalId) => {
        printInfo('Proposal submitted', proposalId);

        contractConfig.storeCodeProposalId = proposalId;
        contractConfig.storeCodeProposalCodeHash = createHash('sha256').update(readWasmFile(options)).digest().toString('hex');
    });
};

const storeInstantiate = (client, wallet, config, options, chainName) => {
    const { contractName, instantiate2 } = options;
    const {
        axelar: {
            contracts: { [contractName]: contractConfig },
        },
        chains: { [chainName]: chainConfig },
    } = config;

    if (instantiate2) {
        throw new Error('instantiate2 not supported for storeInstantiate');
    }

    const initMsg = makeInstantiateMsg(contractName, chainName, config);

    const proposal = encodeStoreInstantiateProposal(config, options, initMsg);
    printProposal(proposal, StoreAndInstantiateContractProposal);

    if (prompt(`Proceed with proposal submission?`, options.yes)) {
        return Promise.resolve();
    }

    return submitProposal(client, wallet, config, options, proposal).then((proposalId) => {
        printInfo('Proposal submitted', proposalId);

        updateContractConfig(contractConfig, chainConfig, 'storeInstantiateProposalId', proposalId);
        contractConfig.storeCodeProposalCodeHash = createHash('sha256').update(readWasmFile(options)).digest().toString('hex');
    });
};

const instantiate = async (client, wallet, config, options, chainName) => {
    const { contractName, instantiate2, predictOnly, fetchCodeId } = options;
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

    if (predictOnly) {
        return predictAndUpdateAddress(client, contractConfig, chainConfig, options, contractName, chainName);
    }

    const initMsg = makeInstantiateMsg(contractName, chainName, config);

    let proposal;

    if (instantiate2) {
        proposal = encodeInstantiate2Proposal(config, options, initMsg);
        printProposal(proposal, InstantiateContract2Proposal);
    } else {
        proposal = encodeInstantiateProposal(config, options, initMsg);
        printProposal(proposal, InstantiateContractProposal);
    }

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

const execute = (client, wallet, config, options, chainName) => {
    const { contractName } = options;
    const {
        axelar: {
            contracts: { [contractName]: contractConfig },
        },
        chains: { [chainName]: chainConfig },
    } = config;

    const proposal = encodeExecuteContractProposal(config, options, chainName);

    printProposal(proposal, ExecuteContractProposal);

    if (prompt(`Proceed with proposal submission?`, options.yes)) {
        return Promise.resolve();
    }

    return submitProposal(client, wallet, config, options, proposal).then((proposalId) => {
        printInfo('Proposal submitted', proposalId);

        updateContractConfig(contractConfig, chainConfig, 'executeProposalId', proposalId);
    });
};

const main = async (options) => {
    const { env, proposalType, contractName } = options;
    const config = loadConfig(env);

    if (config.axelar.contracts === undefined) {
        config.axelar.contracts = {};
    }

    if (config.axelar.contracts[contractName] === undefined) {
        config.axelar.contracts[contractName] = {};
    }

    await prepareWallet(options)
        .then((wallet) => prepareClient(config, wallet))
        .then(({ wallet, client }) => {
            switch (proposalType) {
                case 'store':
                    return storeCode(client, wallet, config, options);

                case 'storeInstantiate': {
                    const chains = getChains(config, options);

                    return chains.reduce((promise, chain) => {
                        return promise.then(() => storeInstantiate(client, wallet, config, options, chain.toLowerCase()));
                    }, Promise.resolve());
                }

                case 'instantiate': {
                    const chains = getChains(config, options);

                    return chains.reduce((promise, chain) => {
                        return promise.then(() =>
                            instantiate(client, wallet, config, options, chain.toLowerCase()).then((contractAddress) => {
                                if (contractAddress) {
                                    printInfo(
                                        `Predicted address for ${
                                            chain.toLowerCase() === 'none' ? '' : chain.toLowerCase().concat(' ')
                                        }${contractName}. Address`,
                                        contractAddress,
                                    );
                                }
                            }),
                        );
                    }, Promise.resolve());
                }

                case 'execute': {
                    const chains = getChains(config, options);

                    return chains.reduce((promise, chain) => {
                        return promise.then(() => execute(client, wallet, config, options, chain.toLowerCase()));
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

    addCommonAmplifierOptions(program, { predictOnly: true });

    // TODO: combine deploy-contract and submit-proposal options to remove duplicates
    program.addOption(new Option('-m, --mnemonic <mnemonic>', 'mnemonic').makeOptionMandatory(true).env('MNEMONIC'));
    program.addOption(new Option('-a, --artifactPath <artifactPath>', 'artifact path').env('ARTIFACT_PATH'));
    program.addOption(new Option('-c, --contractName <contractName>', 'contract name').makeOptionMandatory(true));
    program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').default('none'));

    program.addOption(new Option('-s, --salt <salt>', 'salt for instantiate2. defaults to contract name').env('SALT'));
    program.addOption(
        new Option('--admin <address>', 'when instantiating contract, set an admin address. Defaults to governance module account').default(
            governanceAddress,
        ),
    );
    program.addOption(new Option('--instantiate2', 'use instantiate2 for constant address deployment'));
    program.addOption(new Option('-l, --label <label>', 'contract instance label'));
    program.addOption(new Option('--aarch64', 'aarch64').env('AARCH64').default(false));
    program.addOption(new Option('-y, --yes', 'skip prompt confirmation').env('YES'));

    program.addOption(new Option('-t, --title <title>', 'title of proposal').makeOptionMandatory(true));
    program.addOption(new Option('-d, --description <description>', 'description of proposal').makeOptionMandatory(true));
    program.addOption(new Option('--deposit <deposit>', 'the proposal deposit').makeOptionMandatory(true));
    program.addOption(
        new Option('-r, --runAs <runAs>', 'the address that will execute the message. Defaults to governance address').default(
            governanceAddress,
        ),
    );
    program.addOption(
        new Option('--proposalType <proposalType>', 'proposal type')
            .choices(['store', 'storeInstantiate', 'instantiate', 'execute'])
            .makeOptionMandatory(true),
    );

    program.addOption(new Option('--source <source>', "a valid HTTPS URI to the contract's source code"));
    program.addOption(
        new Option('--builder <builder>', 'a valid docker image name with tag, such as "cosmwasm/workspace-optimizer:0.16.0'),
    );
    program.addOption(
        new Option('-i, --instantiateAddresses <instantiateAddresses>', 'comma separated list of addresses allowed to instantiate'),
    );

    program.addOption(new Option('--msg <msg>', 'json encoded message to submit with an execute contract proposal'));

    program.action((options) => {
        main(options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
