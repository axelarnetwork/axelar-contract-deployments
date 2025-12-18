'use strict';

require('../common/cli-utils');

const { createHash } = require('crypto');
const { instantiate2Address } = require('@cosmjs/cosmwasm-stargate');
const { AccessType } = require('cosmjs-types/cosmwasm/wasm/v1/types');

const {
    CONTRACTS,
    fromHex,
    toArray,
    getSalt,
    getAmplifierContractConfig,
    getCodeId,
    getCodeDetails,
    decodeProposalAttributes,
    encodeStoreCode,
    encodeStoreInstantiate,
    encodeInstantiate,
    encodeExecuteContract,
    encodeParameterChangeProposal,
    encodeMigrate,
    isLegacySDK,
    encodeUpdateInstantiateConfigProposal,
    submitProposal,
} = require('./utils');
const { printInfo, prompt, getChainConfig, readContractCode } = require('../common');
const {
    StoreCodeProposal,
    StoreAndInstantiateContractProposal,
    InstantiateContractProposal,
    InstantiateContract2Proposal,
    ExecuteContractProposal,
    MigrateContractProposal,
    UpdateInstantiateConfigProposal,
} = require('cosmjs-types/cosmwasm/wasm/v1/proposal_legacy');
const { ParameterChangeProposal } = require('cosmjs-types/cosmos/params/v1beta1/params');
const {
    MsgExecuteContract,
    MsgInstantiateContract,
    MsgInstantiateContract2,
    MsgMigrateContract,
    MsgStoreCode,
    MsgStoreAndInstantiateContract,
    MsgUpdateInstantiateConfig,
} = require('cosmjs-types/cosmwasm/wasm/v1/tx');

const { Command, Option } = require('commander');
const { addAmplifierOptions } = require('./cli-utils');
const { mainProcessor } = require('./processor');
const { CoordinatorManager } = require('./coordinator');

const predictAddress = async (client, contractConfig, options) => {
    const { contractName, salt, chainName, runAs } = options;

    const { checksum } = await client.getCodeDetails(contractConfig.codeId);
    const contractAddress = instantiate2Address(fromHex(checksum), runAs, getSalt(salt, contractName, chainName), 'axelar');

    printInfo(`Predicted address for ${chainName ? chainName.concat(' ') : ''}${contractName}. Address`, contractAddress);

    return contractAddress;
};

const printProposal = (proposalData, proposalType = null) => {
    if (proposalType) {
        // Legacy: single proposal with decoder
        printInfo(
            `Encoded ${proposalData.typeUrl}`,
            JSON.stringify(decodeProposalAttributes(proposalType.toJSON(proposalType.decode(proposalData.value))), null, 2),
        );
    } else {
        // v0.50: array of messages
        proposalData.forEach((message) => {
            const typeMap = {
                '/cosmwasm.wasm.v1.MsgExecuteContract': MsgExecuteContract,
                '/cosmwasm.wasm.v1.MsgStoreCode': MsgStoreCode,
                '/cosmwasm.wasm.v1.MsgInstantiateContract': MsgInstantiateContract,
                '/cosmwasm.wasm.v1.MsgInstantiateContract2': MsgInstantiateContract2,
                '/cosmwasm.wasm.v1.MsgMigrateContract': MsgMigrateContract,
                '/cosmwasm.wasm.v1.MsgStoreAndInstantiateContract': MsgStoreAndInstantiateContract,
                '/cosmwasm.wasm.v1.MsgUpdateInstantiateConfig': MsgUpdateInstantiateConfig,
            };
            const MessageType = typeMap[message.typeUrl];
            if (MessageType) {
                const decoded = MessageType.decode(message.value);
                if (decoded.codeId) {
                    decoded.codeId = decoded.codeId.toString();
                }
                if (
                    (message.typeUrl === '/cosmwasm.wasm.v1.MsgExecuteContract' ||
                        message.typeUrl === '/cosmwasm.wasm.v1.MsgInstantiateContract' ||
                        message.typeUrl === '/cosmwasm.wasm.v1.MsgInstantiateContract2' ||
                        message.typeUrl === '/cosmwasm.wasm.v1.MsgMigrateContract' ||
                        message.typeUrl === '/cosmwasm.wasm.v1.MsgStoreAndInstantiateContract') &&
                    decoded.msg
                ) {
                    decoded.msg = JSON.parse(Buffer.from(decoded.msg).toString());
                }
                if (decoded.wasmByteCode) {
                    decoded.wasmByteCode = `<${decoded.wasmByteCode.length} bytes>`;
                }
                printInfo(`Encoded ${message.typeUrl}`, JSON.stringify(decoded, null, 2));
            } else {
                printInfo(`Unknown message type: ${message.typeUrl}`, '<Unable to decode>');
            }
        });
    }
};

const confirmProposalSubmission = (options, proposalData, proposalType = null) => {
    printProposal(proposalData, proposalType);
    if (prompt(`Proceed with proposal submission?`, options.yes)) {
        return false;
    }
    return true;
};

const callSubmitProposal = async (client, config, options, proposal, fee) => {
    const proposalId = await submitProposal(client, config, options, proposal, fee);
    printInfo('Proposal submitted', proposalId);
    return proposalId;
};

const saveStoreCodeProposalInfo = (config, contractName, contractCodePath, proposalId) => {
    const contractBaseConfig = config.getContractConfig(contractName);
    contractBaseConfig.storeCodeProposalId = proposalId;

    const contractOptions = { contractName, contractCodePath };
    contractBaseConfig.storeCodeProposalCodeHash = createHash('sha256').update(readContractCode(contractOptions)).digest().toString('hex');
};

const storeCode = async (client, config, options, _args, fee) => {
    const isLegacy = isLegacySDK(config);
    let contractName = options.contractName;
    const { contractCodePath, contractCodePaths } = options;

    if (!Array.isArray(contractName)) {
        contractName = [contractName];
    }

    if (isLegacy) {
        if (contractName.length > 1) {
            throw new Error('Legacy SDK only supports storing one contract at a time. Please provide a single contract name.');
        }
        const singleContractName = contractName[0];
        const legacyOptions = { ...options, contractName: singleContractName };
        const proposal = encodeStoreCode(config, legacyOptions);

        if (!confirmProposalSubmission(options, proposal, StoreCodeProposal)) {
            return;
        }
        const proposalId = await callSubmitProposal(client, config, options, proposal, fee);
        saveStoreCodeProposalInfo(config, singleContractName, contractCodePath, proposalId);
        return proposalId;
    } else {
        const contractNames = contractName;
        const proposal = contractNames.map((name) => {
            const contractOptions = {
                ...options,
                contractName: name,
                contractCodePath: contractCodePaths ? contractCodePaths[name] : contractCodePath,
            };
            return encodeStoreCode(config, contractOptions);
        });

        if (!confirmProposalSubmission(options, proposal)) {
            return;
        }
        const proposalId = await callSubmitProposal(client, config, options, proposal, fee);
        contractNames.forEach((name) => {
            const codePath = contractCodePaths ? contractCodePaths[name] : contractCodePath;
            saveStoreCodeProposalInfo(config, name, codePath, proposalId);
        });
        return proposalId;
    }
};

const storeInstantiate = async (client, config, options, _args, fee) => {
    const isLegacy = isLegacySDK(config);
    let { contractName } = options;
    const { instantiate2 } = options;

    if (Array.isArray(contractName)) {
        if (contractName.length > 1) {
            throw new Error('storeInstantiate only supports a single contract at a time');
        }
        contractName = contractName[0];
    }

    const { contractConfig, contractBaseConfig } = getAmplifierContractConfig(config, { ...options, contractName });

    if (instantiate2) {
        throw new Error('instantiate2 not supported for storeInstantiate');
    }

    const initMsg = CONTRACTS[contractName].makeInstantiateMsg(config, { ...options, contractName }, contractConfig);
    const proposal = encodeStoreInstantiate(config, { ...options, contractName }, initMsg);

    if (isLegacy) {
        if (!confirmProposalSubmission(options, proposal, StoreAndInstantiateContractProposal)) {
            return;
        }
        const proposalId = await callSubmitProposal(client, config, options, proposal, fee);

        contractConfig.storeInstantiateProposalId = proposalId;
        contractBaseConfig.storeCodeProposalCodeHash = createHash('sha256')
            .update(readContractCode({ ...options, contractName }))
            .digest()
            .toString('hex');
    } else {
        if (!confirmProposalSubmission(options, [proposal])) {
            return;
        }
        const proposalId = await callSubmitProposal(client, config, options, [proposal], fee);

        contractConfig.storeInstantiateProposalId = proposalId;
        contractBaseConfig.storeCodeProposalCodeHash = createHash('sha256')
            .update(readContractCode({ ...options, contractName }))
            .digest()
            .toString('hex');
    }
};

const instantiate = async (client, config, options, _args, fee) => {
    let contractName = options.contractName;

    if (!Array.isArray(contractName)) {
        contractName = [contractName];
    }

    const singleContractName = contractName[0];
    if (contractName.length > 1) {
        throw new Error('Instantiate command only supports one contract at a time.');
    }

    const isLegacy = isLegacySDK(config);
    const { instantiate2, predictOnly } = options;

    const instantiateOptions = { ...options, contractName: singleContractName };
    const { contractConfig } = getAmplifierContractConfig(config, instantiateOptions);

    contractConfig.codeId = await getCodeId(client, config, instantiateOptions);

    let contractAddress;

    if (predictOnly) {
        contractAddress = await predictAddress(client, contractConfig, instantiateOptions);
        contractConfig.address = contractAddress;
        return;
    }

    const initMsg = CONTRACTS[singleContractName].makeInstantiateMsg(config, instantiateOptions, contractConfig);

    const proposal = encodeInstantiate(config, instantiateOptions, initMsg);

    if (instantiate2) {
        contractAddress = await predictAddress(client, contractConfig, instantiateOptions);
    } else {
        printInfo('Contract address cannot be predicted without using `--instantiate2` flag, address will not be saved in the config');
    }

    if (isLegacy) {
        const proposalType = instantiate2 ? InstantiateContract2Proposal : InstantiateContractProposal;
        if (!confirmProposalSubmission(options, proposal, proposalType)) {
            return;
        }
        const proposalId = await callSubmitProposal(client, config, options, proposal, fee);
        contractConfig.instantiateProposalId = proposalId;
        if (instantiate2) {
            contractConfig.address = contractAddress;
        }
    } else {
        if (!confirmProposalSubmission(options, [proposal])) {
            return;
        }
        const proposalId = await callSubmitProposal(client, config, options, [proposal], fee);
        contractConfig.instantiateProposalId = proposalId;
        if (instantiate2) {
            contractConfig.address = contractAddress;
        }
    }
};

const executeByGovernance = async (client, config, options, _args, fee) => {
    const { chainName } = options;
    let contractName = options.contractName;

    if (!Array.isArray(contractName)) {
        contractName = [contractName];
    }

    const singleContractName = contractName[0];
    if (contractName.length > 1) {
        throw new Error(
            'Execute command only supports one contract at a time. Use multiple --msg flags for multiple messages to the same contract.',
        );
    }

    const isLegacy = isLegacySDK(config);

    if (isLegacy) {
        const msgs = toArray(options.msg);
        if (msgs.length > 1) {
            throw new Error('Legacy SDK only supports one message per proposal. Please provide a single --msg flag.');
        }
        const singleMsg = msgs[0];
        const legacyOptions = { ...options, contractName: singleContractName, msg: singleMsg };
        const proposal = encodeExecuteContract(config, legacyOptions, chainName);

        if (!confirmProposalSubmission(options, proposal, ExecuteContractProposal)) {
            return;
        }
        return callSubmitProposal(client, config, options, proposal, fee);
    } else {
        const { msg } = options;
        const msgs = toArray(msg);

        const messages = msgs.map((msgJson) => {
            const msgOptions = { ...options, contractName: singleContractName, msg: msgJson };
            return encodeExecuteContract(config, msgOptions, chainName);
        });

        if (!confirmProposalSubmission(options, messages)) {
            return;
        }

        return callSubmitProposal(client, config, options, messages, fee);
    }
};

const paramChange = async (client, config, options, _args, fee) => {
    const isLegacy = isLegacySDK(config);

    if (!isLegacy) {
        throw new Error('Parameter change proposals are not yet supported on SDK v0.50+.');
    }

    const proposal = encodeParameterChangeProposal(options);

    if (!confirmProposalSubmission(options, proposal, ParameterChangeProposal)) {
        return;
    }

    return callSubmitProposal(client, config, options, proposal, fee);
};

const migrate = async (client, config, options, _args, fee) => {
    let { contractName } = options;

    if (Array.isArray(contractName)) {
        if (contractName.length > 1) {
            throw new Error('migrate only supports a single contract at a time');
        }
        contractName = contractName[0];
    }

    const isLegacy = isLegacySDK(config);
    const { contractConfig } = getAmplifierContractConfig(config, { ...options, contractName });
    contractConfig.codeId = await getCodeId(client, config, { ...options, contractName });

    const proposal = encodeMigrate(config, { ...options, contractName });

    if (isLegacy) {
        if (!confirmProposalSubmission(options, proposal, MigrateContractProposal)) {
            return;
        }
        return callSubmitProposal(client, config, options, proposal, fee);
    } else {
        if (!confirmProposalSubmission(options, [proposal])) {
            return;
        }
        return callSubmitProposal(client, config, options, [proposal], fee);
    }
};

async function instantiatePermissions(client, options, config, senderAddress, coordinatorAddress, permittedAddresses, codeId, fee) {
    const addresses = [...permittedAddresses, coordinatorAddress];

    const updateMsg = JSON.stringify([
        {
            codeId: codeId,
            instantiatePermission: {
                permission: AccessType.ACCESS_TYPE_ANY_OF_ADDRESSES,
                addresses: addresses,
            },
        },
    ]);

    const updateOptions = {
        msg: updateMsg,
        title: options.title,
        description: options.description,
        runAs: senderAddress,
        deposit: options.deposit,
    };

    const proposal = encodeUpdateInstantiateConfigProposal(updateOptions);

    if (!confirmProposalSubmission(options, [proposal])) {
        return;
    }

    const proposalId = await submitProposal(client, config, updateOptions, [proposal], fee);
    printInfo('Instantiate params proposal successfully submitted. Proposal ID', proposalId);
    return proposalId;
}

async function coordinatorInstantiatePermissions(client, config, options, _args, fee) {
    const senderAddress = client.accounts[0].address;
    const contractAddress = config.axelar.contracts['Coordinator']?.address;

    if (!contractAddress) {
        throw new Error('cannot find coordinator address in configuration');
    }

    const codeId = await getCodeId(client, config, { ...options, contractName: options.contractName });
    const codeDetails = await getCodeDetails(config, codeId);
    const permissions = codeDetails.instantiatePermission;

    if (
        permissions?.permission === AccessType.ACCESS_TYPE_EVERYBODY ||
        (permissions?.address === contractAddress && permissions?.permission === AccessType.ACCESS_TYPE_ONLY_ADDRESS)
    ) {
        throw new Error(`coordinator is already allowed to instantiate code id ${codeId}`);
    }

    const permittedAddresses = permissions.addresses ?? [];
    if (permittedAddresses.includes(contractAddress) && permissions?.permission === AccessType.ACCESS_TYPE_ANY_OF_ADDRESSES) {
        throw new Error(`coordinator is already allowed to instantiate code id ${codeId}`);
    }

    return instantiatePermissions(client, options, config, senderAddress, contractAddress, permittedAddresses, codeId, fee);
}

const programHandler = () => {
    const program = new Command();

    program.name('submit-proposal').description('Submit governance proposals');

    const storeCmd = program
        .command('store')
        .description('Submit a wasm binary proposal')
        .action((options) => mainProcessor(storeCode, options));
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
        .action((options) => mainProcessor(storeInstantiate, options));
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
        .action((options) => mainProcessor(instantiate, options));
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

    const executeByGovernanceCmd = program
        .command('executeByGovernance')
        .description('Submit an execute wasm contract proposal')
        .action((options) => mainProcessor(executeByGovernance, options));
    addAmplifierOptions(executeByGovernanceCmd, {
        contractOptions: true,
        executeProposalOptions: true,
        proposalOptions: true,
        runAs: true,
    });

    const paramChangeCmd = program
        .command('paramChange')
        .description('Submit a parameter change proposal')
        .action((options) => mainProcessor(paramChange, options));
    addAmplifierOptions(paramChangeCmd, { paramChangeProposalOptions: true, proposalOptions: true });

    const migrateCmd = program
        .command('migrate')
        .description('Submit a migrate contract proposal')
        .action((options) => mainProcessor(migrate, options));
    addAmplifierOptions(migrateCmd, {
        contractOptions: true,
        migrateOptions: true,
        proposalOptions: true,
        codeId: true,
        fetchCodeId: true,
        runAs: true,
    });

    addAmplifierOptions(
        program
            .command('coordinator-instantiate-permissions')
            .addOption(
                new Option('--contractName <contractName>', 'coordinator will have instantiate permissions for this contract')
                    .makeOptionMandatory(true)
                    .choices(['Gateway', 'VotingVerifier', 'MultisigProver']),
            )
            .description('Give coordinator instantiate permissions for the given contract')
            .action((options) => {
                mainProcessor(coordinatorInstantiatePermissions, options, []);
            }),
        {
            contractOptions: true,
            proposalOptions: true,
        },
    );

    program.parse();
};

if (require.main === module) {
    programHandler();
}

module.exports = {
    confirmProposalSubmission,
    executeByGovernance,
    migrate,
};
