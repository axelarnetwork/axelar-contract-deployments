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
    initContractConfig,
    getAmplifierBaseContractConfig,
    getAmplifierContractConfig,
    updateCodeId,
    decodeProposalAttributes,
    encodeStoreCodeProposal,
    encodeStoreInstantiateProposal,
    encodeInstantiateProposal,
    encodeInstantiate2Proposal,
    encodeExecuteContractProposal,
    encodeParameterChangeProposal,
    encodeMigrateContractProposal,
    submitProposal,
    makeInstantiateMsg,
} = require('./utils');
const { saveConfig, loadConfig, printInfo, prompt } = require('../common');
const {
    StoreCodeProposal,
    StoreAndInstantiateContractProposal,
    InstantiateContractProposal,
    InstantiateContract2Proposal,
    ExecuteContractProposal,
    MigrateContractProposal,
} = require('cosmjs-types/cosmwasm/wasm/v1/proposal');
const { ParameterChangeProposal } = require('cosmjs-types/cosmos/params/v1beta1/params');

const { Command } = require('commander');
const { addAmplifierOptions } = require('./cli-utils');

const predictAndUpdateAddress = async (client, contractConfig, options) => {
    const { contractName, salt, chainName, runAs } = options;

    const { checksum } = await client.getCodeDetails(contractConfig.codeId);
    const contractAddress = instantiate2Address(fromHex(checksum), runAs, getSalt(salt, contractName, chainName), 'axelar');

    contractConfig.address = contractAddress;

    printInfo(`Predicted address for ${chainName ? chainName.concat(' ') : ''}${contractName}. Address`, contractAddress);
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
    const contractBaseConfig = getAmplifierBaseContractConfig(config, contractName);

    const proposal = encodeStoreCodeProposal(options);

    if (!confirmProposalSubmission(options, proposal, StoreCodeProposal)) {
        return;
    }

    const proposalId = await callSubmitProposal(client, wallet, config, options, proposal);

    contractBaseConfig.storeCodeProposalId = proposalId;
    contractBaseConfig.storeCodeProposalCodeHash = createHash('sha256').update(readWasmFile(options)).digest().toString('hex');
};

const storeInstantiate = async (client, wallet, config, options) => {
    const { contractName, instantiate2, chainName } = options;
    const { contractConfig, contractBaseConfig } = getAmplifierContractConfig(config, options);

    if (instantiate2) {
        throw new Error('instantiate2 not supported for storeInstantiate');
    }

    const initMsg = makeInstantiateMsg(contractName, chainName, config);
    const proposal = encodeStoreInstantiateProposal(config, options, initMsg);

    if (!confirmProposalSubmission(options, proposal, StoreAndInstantiateContractProposal)) {
        return;
    }

    const proposalId = await callSubmitProposal(client, wallet, config, options, proposal);

    contractConfig.storeInstantiateProposalId = proposalId;
    contractBaseConfig.storeCodeProposalCodeHash = createHash('sha256').update(readWasmFile(options)).digest().toString('hex');
};

const instantiate = async (client, wallet, config, options) => {
    const { contractName, instantiate2, predictOnly, chainName } = options;
    const { contractConfig } = getAmplifierContractConfig(config, options);

    await updateCodeId(client, config, options);

    if (predictOnly) {
        return predictAndUpdateAddress(client, contractConfig, options);
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

    contractConfig.instantiateProposalId = proposalId;

    if (instantiate2) {
        return predictAndUpdateAddress(client, contractConfig, options);
    }
};

const execute = async (client, wallet, config, options) => {
    const { chainName } = options;

    const proposal = encodeExecuteContractProposal(config, options, chainName);

    if (!confirmProposalSubmission(options, proposal, ExecuteContractProposal)) {
        return;
    }

    await callSubmitProposal(client, wallet, config, options, proposal);
};

const paramChange = async (client, wallet, config, options) => {
    const proposal = encodeParameterChangeProposal(options);

    if (!confirmProposalSubmission(options, proposal, ParameterChangeProposal)) {
        return;
    }

    await callSubmitProposal(client, wallet, config, options, proposal);
};

const migrate = async (client, wallet, config, options) => {
    await updateCodeId(client, config, options);

    const proposal = encodeMigrateContractProposal(config, options);

    if (!confirmProposalSubmission(options, proposal, MigrateContractProposal)) {
        return;
    }

    await callSubmitProposal(client, wallet, config, options, proposal);
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

    program.name('submit-proposal').description('Submit governance proposals');

    const storeCmd = program
        .command('store')
        .description('Submit a wasm binary proposal')
        .action((options) => {
            mainProcessor(storeCode, options);
        });
    addAmplifierOptions(storeCmd, {
        contractOptions: true,
        storeOptions: true,
        storeProposalOptions: true,
        proposalOptions: true,
        runAs: true,
    });

    const storeInstantiateCmd = program
        .command('storeInstantiate')
        .description('Submit and instantiate a wasm contract proposal')
        .action((options) => {
            mainProcessor(storeInstantiate, options);
        });
    addAmplifierOptions(storeInstantiateCmd, {
        contractOptions: true,
        storeOptions: true,
        storeProposalOptions: true,
        proposalOptions: true,
        instantiateOptions: true,
        runAs: true,
    });

    const instantiateCmd = program
        .command('instantiate')
        .description('Submit an instantiate wasm contract proposal')
        .action((options) => {
            mainProcessor(instantiate, options);
        });
    addAmplifierOptions(instantiateCmd, {
        contractOptions: true,
        instantiateOptions: true,
        instantiate2Options: true,
        instantiateProposalOptions: true,
        proposalOptions: true,
        codeId: true,
        fetchCodeId: true,
        runAs: true,
    });

    const executeCmd = program
        .command('execute')
        .description('Submit a execute wasm contract proposal')
        .action((options) => {
            mainProcessor(execute, options);
        });
    addAmplifierOptions(executeCmd, { contractOptions: true, executeProposalOptions: true, proposalOptions: true, runAs: true });

    const paramChangeCmd = program
        .command('paramChange')
        .description('Submit a parameter change proposal')
        .action((options) => {
            mainProcessor(paramChange, options);
        });
    addAmplifierOptions(paramChangeCmd, { paramChangeProposalOptions: true, proposalOptions: true });

    const migrateCmd = program
        .command('migrate')
        .description('Submit a migrate contract proposal')
        .action((options) => {
            mainProcessor(migrate, options);
        });
    addAmplifierOptions(migrateCmd, {
        contractOptions: true,
        migrateOptions: true,
        proposalOptions: true,
        codeId: true,
        fetchCodeId: true,
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
