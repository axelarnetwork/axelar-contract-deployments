'use strict';

require('dotenv').config();

const { createHash } = require('crypto');

const { instantiate2Address } = require('@cosmjs/cosmwasm-stargate');

const {
    prepareWallet,
    prepareClient,
    fromHex,
    getSalt,
    readWasmFile,
    getChains,
    updateContractConfig,
    fetchCodeIdFromCodeHash,
    decodeProposalAttributes,
    encodeStoreCodeProposal,
    encodeStoreInstantiateProposal,
    encodeInstantiateProposal,
    encodeInstantiate2Proposal,
    encodeExecuteContractProposal,
    encodeParameterChangeProposal,
    submitProposal,
    makeInstantiateMsg,
    governanceAddress,
} = require('./utils');
const { isNumber, saveConfig, loadConfig, printInfo, prompt } = require('../common');
const {
    StoreCodeProposal,
    StoreAndInstantiateContractProposal,
    InstantiateContractProposal,
    InstantiateContract2Proposal,
    ExecuteContractProposal,
} = require('cosmjs-types/cosmwasm/wasm/v1/proposal');
const { ParameterChangeProposal } = require('cosmjs-types/cosmos/params/v1beta1/params');

const { Command, Option } = require('commander');
const { addAmplifierOptions } = require('./cli-utils');

const predictAndUpdateAddress = async (client, contractConfig, chainConfig, options) => {
    const { contractName, salt, chainNames, runAs } = options;

    const { checksum } = await client.getCodeDetails(contractConfig.codeId);
    const contractAddress = instantiate2Address(fromHex(checksum), runAs, getSalt(salt, contractName, chainNames), 'axelar');

    updateContractConfig(contractConfig, chainConfig, 'address', contractAddress);

    return contractAddress;
};

const printProposal = (proposal, proposalType) => {
    printInfo(
        `Encoded ${proposal.typeUrl}`,
        JSON.stringify(decodeProposalAttributes(proposalType.toJSON(proposalType.decode(proposal.value))), null, 2),
    );
};

const confirmProposalSubmission = (options, proposal, proposalType) => {
    printProposal(proposal, proposalType);

    if (prompt(`Proceed with proposal submission?`, options.yes)) {
        return false;
    }

    return true;
};

const callSubmitProposal = async (client, wallet, config, options, proposal) => {
    const proposalId = await submitProposal(client, wallet, config, options, proposal);
    printInfo('Proposal submitted', proposalId);

    return proposalId;
};

const storeCode = async (client, wallet, config, options) => {
    const { contractName } = options;
    const {
        axelar: {
            contracts: { [contractName]: contractConfig },
        },
    } = config;

    const proposal = encodeStoreCodeProposal(options);

    if (!confirmProposalSubmission(options, proposal, StoreCodeProposal)) {
        return;
    }

    const proposalId = await callSubmitProposal(client, wallet, config, options, proposal);

    contractConfig.storeCodeProposalId = proposalId;
    contractConfig.storeCodeProposalCodeHash = createHash('sha256').update(readWasmFile(options)).digest().toString('hex');
};

const storeInstantiate = async (client, wallet, config, options, chainName) => {
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

    if (!confirmProposalSubmission(options, proposal, StoreAndInstantiateContractProposal)) {
        return;
    }

    const proposalId = await callSubmitProposal(client, wallet, config, options, proposal);

    updateContractConfig(contractConfig, chainConfig, 'storeInstantiateProposalId', proposalId);
    contractConfig.storeCodeProposalCodeHash = createHash('sha256').update(readWasmFile(options)).digest().toString('hex');
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
        return predictAndUpdateAddress(client, contractConfig, chainConfig, options);
    }

    const initMsg = makeInstantiateMsg(contractName, chainName, config);

    let proposal;
    let proposalType;

    if (instantiate2) {
        proposal = encodeInstantiate2Proposal(config, options, initMsg);
        proposalType = InstantiateContract2Proposal;
    } else {
        proposal = encodeInstantiateProposal(config, options, initMsg);
        proposalType = InstantiateContractProposal;
    }

    if (!confirmProposalSubmission(options, proposal, proposalType)) {
        return;
    }

    const proposalId = await callSubmitProposal(client, wallet, config, options, proposal);

    updateContractConfig(contractConfig, chainConfig, 'instantiateProposalId', proposalId);

    if (instantiate2) {
        return predictAndUpdateAddress(client, contractConfig, chainConfig, options);
    }
};

const execute = async (client, wallet, config, options, chainName) => {
    const { contractName } = options;
    const {
        axelar: {
            contracts: { [contractName]: contractConfig },
        },
        chains: { [chainName]: chainConfig },
    } = config;

    const proposal = encodeExecuteContractProposal(config, options, chainName);

    if (!confirmProposalSubmission(options, proposal, ExecuteContractProposal)) {
        return;
    }

    const proposalId = await callSubmitProposal(client, wallet, config, options, proposal);

    updateContractConfig(contractConfig, chainConfig, 'executeProposalId', proposalId);
};

const paramChange = async (client, wallet, config, options) => {
    const proposal = encodeParameterChangeProposal(options);

    if (!confirmProposalSubmission(options, proposal, ParameterChangeProposal)) {
        return;
    }

    await callSubmitProposal(client, wallet, config, options, proposal);
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

    const wallet = await prepareWallet(options);
    const client = await prepareClient(config, wallet);

    switch (proposalType) {
        case 'store':
            await storeCode(client, wallet, config, options);
            break;

        case 'storeInstantiate': {
            const chains = getChains(config, options);

            for (const chain of chains) {
                await storeInstantiate(client, wallet, config, options, chain.toLowerCase());
            }

            break;
        }

        case 'instantiate': {
            const chains = getChains(config, options);

            for (const chain of chains) {
                const contractAddress = await instantiate(client, wallet, config, options, chain.toLowerCase());

                if (contractAddress) {
                    printInfo(
                        `Predicted address for ${
                            chain.toLowerCase() === 'none' ? '' : chain.toLowerCase().concat(' ')
                        }${contractName}. Address`,
                        contractAddress,
                    );
                }
            }

            break;
        }

        case 'execute': {
            const chains = getChains(config, options);

            for (const chain of chains) {
                await execute(client, wallet, config, options, chain.toLowerCase());
            }

            break;
        }

        case 'paramChange': {
            await paramChange(client, wallet, config, options);
            break;
        }

        default:
            throw new Error('Invalid proposal type');
    }

    saveConfig(config, env);
};

const programHandler = () => {
    const program = new Command();

    program.name('submit-proposal').description('Submit governance proposals');

    addAmplifierOptions(program);

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
            .choices(['store', 'storeInstantiate', 'instantiate', 'execute', 'paramChange'])
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

    program.addOption(new Option('--changes <changes>', 'parameter changes'));

    program.addOption(new Option('--predictOnly', 'output the predicted changes only').env('PREDICT_ONLY'));

    program.action((options) => {
        main(options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
