'use strict';

require('../common/cli-utils');

const { createHash } = require('crypto');
const { instantiate2Address } = require('@cosmjs/cosmwasm-stargate');
const { AccessType } = require('cosmjs-types/cosmwasm/wasm/v1/types');

const {
    CONTRACTS,
    fromHex,
    getSalt,
    getAmplifierContractConfig,
    getCodeId,
    getCodeDetails,
    getChainTruncationParams,
    decodeProposalAttributes,
    encodeStoreCode,
    encodeStoreInstantiateProposal,
    encodeInstantiateProposal,
    encodeInstantiate2Proposal,
    encodeExecuteContract,
    encodeParameterChangeProposal,
    encodeMigrateContractProposal,
    isLegacySDK,
    encodeUpdateInstantiateConfigProposal,
    submitProposal,
    validateItsChainChange,
    VERIFIER_CONTRACT_NAME,
    GATEWAY_CONTRACT_NAME,
    MULTISIG_PROVER_CONTRACT_NAME,
} = require('./utils');
const { printInfo, prompt, getChainConfig, itsEdgeContract, readContractCode } = require('../common');
const {
    StoreCodeProposal,
    StoreAndInstantiateContractProposal,
    InstantiateContractProposal,
    InstantiateContract2Proposal,
    ExecuteContractProposal,
    MigrateContractProposal,
    UpdateInstantiateConfigProposal,
} = require('cosmjs-types/cosmwasm/wasm/v1/proposal');
const { ParameterChangeProposal } = require('cosmjs-types/cosmos/params/v1beta1/params');
const { MsgExecuteContract, MsgStoreCode } = require('cosmjs-types/cosmwasm/wasm/v1/tx');

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
            };
            const MessageType = typeMap[message.typeUrl];
            if (MessageType) {
                const decoded = MessageType.decode(message.value);
                if (message.typeUrl === '/cosmwasm.wasm.v1.MsgExecuteContract' && decoded.msg) {
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
    const { contractName, instantiate2 } = options;
    const { contractConfig, contractBaseConfig } = getAmplifierContractConfig(config, options);

    if (instantiate2) {
        throw new Error('instantiate2 not supported for storeInstantiate');
    }

    const initMsg = CONTRACTS[contractName].makeInstantiateMsg(config, options, contractConfig);
    const proposal = encodeStoreInstantiateProposal(config, options, initMsg);

    if (!confirmProposalSubmission(options, proposal, StoreAndInstantiateContractProposal)) {
        return;
    }

    const proposalId = await callSubmitProposal(client, config, options, proposal, fee);

    contractConfig.storeInstantiateProposalId = proposalId;
    contractBaseConfig.storeCodeProposalCodeHash = createHash('sha256').update(readContractCode(options)).digest().toString('hex');
};

const instantiate = async (client, config, options, _args, fee) => {
    const { contractName, instantiate2, predictOnly } = options;
    const { contractConfig } = getAmplifierContractConfig(config, options);

    contractConfig.codeId = await getCodeId(client, config, options);

    let contractAddress;

    if (predictOnly) {
        contractAddress = await predictAddress(client, contractConfig, options);
        contractConfig.address = contractAddress;

        return;
    }

    const initMsg = CONTRACTS[contractName].makeInstantiateMsg(config, options, contractConfig);

    let proposal;
    let proposalType;

    if (instantiate2) {
        proposal = encodeInstantiate2Proposal(config, options, initMsg);
        proposalType = InstantiateContract2Proposal;

        contractAddress = await predictAddress(client, contractConfig, options);
    } else {
        proposal = encodeInstantiateProposal(config, options, initMsg);
        proposalType = InstantiateContractProposal;

        printInfo('Contract address cannot be predicted without using `--instantiate2` flag, address will not be saved in the config');
    }

    if (!confirmProposalSubmission(options, proposal, proposalType)) {
        return;
    }

    const proposalId = await callSubmitProposal(client, config, options, proposal, fee);

    contractConfig.instantiateProposalId = proposalId;
    if (instantiate2) contractConfig.address = contractAddress;
};

const execute = async (client, config, options, _args, fee) => {
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
        const msgs = Array.isArray(options.msg) ? options.msg : [options.msg];
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
        const msgs = Array.isArray(msg) ? msg : [msg];

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

const registerItsChain = async (client, config, options, _args, fee) => {
    if (options.itsEdgeContract && options.chains.length > 1) {
        throw new Error('Cannot use --its-edge-contract option with multiple chains.');
    }

    const itsMsgTranslator = options.itsMsgTranslator || config.axelar?.contracts?.ItsAbiTranslator?.address;

    if (!itsMsgTranslator) {
        throw new Error('ItsMsgTranslator address is required for registerItsChain');
    }

    const chains = options.chains.map((chain) => {
        const chainConfig = getChainConfig(config.chains, chain);
        const { maxUintBits, maxDecimalsWhenTruncating } = getChainTruncationParams(config, chainConfig);
        const itsEdgeContractAddress = options.itsEdgeContract || itsEdgeContract(chainConfig);

        return {
            chain: chainConfig.axelarId,
            its_edge_contract: itsEdgeContractAddress,
            msg_translator: itsMsgTranslator,
            truncation: {
                max_uint_bits: maxUintBits,
                max_decimals_when_truncating: maxDecimalsWhenTruncating,
            },
        };
    });

    if (options.update) {
        for (let i = 0; i < options.chains.length; i++) {
            const chain = options.chains[i];
            await validateItsChainChange(client, config, chain, chains[i]);
        }
    }

    const operation = options.update ? 'update' : 'register';

    return execute(
        client,
        config,
        {
            ...options,
            contractName: 'InterchainTokenService',
            msg: `{ "${operation}_chains": { "chains": ${JSON.stringify(chains)} } }`,
        },
        undefined,
        fee,
    );
};

const registerProtocol = async (client, config, options, _args, fee) => {
    const serviceRegistry = config.axelar?.contracts?.ServiceRegistry?.address;
    const router = config.axelar?.contracts?.Router?.address;
    const multisig = config.axelar?.contracts?.Multisig?.address;

    return execute(
        client,
        config,
        {
            ...options,
            contractName: 'Coordinator',
            msg: JSON.stringify({
                register_protocol: {
                    service_registry_address: serviceRegistry,
                    router_address: router,
                    multisig_address: multisig,
                },
            }),
        },
        undefined,
        fee,
    );
};

const paramChange = async (client, config, options, _args, fee) => {
    const proposal = encodeParameterChangeProposal(options);

    if (!confirmProposalSubmission(options, proposal, ParameterChangeProposal)) {
        return;
    }

    return callSubmitProposal(client, config, options, proposal, fee);
};

const migrate = async (client, config, options, _args, fee) => {
    const { contractConfig } = getAmplifierContractConfig(config, options);
    contractConfig.codeId = await getCodeId(client, config, options);

    const proposal = encodeMigrateContractProposal(config, options);

    if (!confirmProposalSubmission(options, proposal, MigrateContractProposal)) {
        return;
    }

    return callSubmitProposal(client, config, options, proposal, fee);
};

const instantiateChainContracts = async (client, config, options, _args, fee) => {
    const { chainName, salt, gatewayCodeId, verifierCodeId, proverCodeId, admin } = options;

    const coordinatorAddress = config.axelar?.contracts?.Coordinator?.address;
    if (!coordinatorAddress) {
        throw new Error('Coordinator contract address not found in config');
    }

    if (!admin) {
        throw new Error('Admin address is required when instantiating chain contracts');
    }

    if (!salt) {
        throw new Error('Salt is required when instantiating chain contracts');
    }

    // validate that the contract configs exist
    let gatewayConfig = config.getContractConfigByChain(GATEWAY_CONTRACT_NAME, chainName);
    let verifierConfig = config.getContractConfigByChain(VERIFIER_CONTRACT_NAME, chainName);
    let proverConfig = config.getContractConfigByChain(MULTISIG_PROVER_CONTRACT_NAME, chainName);

    if (options.fetchCodeId) {
        const gatewayCode = gatewayCodeId || (await getCodeId(client, config, { ...options, contractName: GATEWAY_CONTRACT_NAME }));
        const verifierCode = verifierCodeId || (await getCodeId(client, config, { ...options, contractName: VERIFIER_CONTRACT_NAME }));
        const proverCode = proverCodeId || (await getCodeId(client, config, { ...options, contractName: MULTISIG_PROVER_CONTRACT_NAME }));
        gatewayConfig.codeId = gatewayCode;
        verifierConfig.codeId = verifierCode;
        proverConfig.codeId = proverCode;
    } else {
        if (!gatewayConfig.codeId && !gatewayCodeId) {
            throw new Error(
                'Gateway code ID is required when --fetchCodeId is not used. Please provide it with --gatewayCodeId or in the config',
            );
        }
        if (!verifierConfig.codeId && !verifierCodeId) {
            throw new Error(
                'VotingVerifier code ID is required when --fetchCodeId is not used. Please provide it with --verifierCodeId or in the config',
            );
        }
        if (!proverConfig.codeId && !proverCodeId) {
            throw new Error(
                'MultisigProver code ID is required when --fetchCodeId is not used. Please provide it with --proverCodeId or in the config',
            );
        }

        gatewayConfig.codeId = gatewayCodeId || gatewayConfig.codeId;
        verifierConfig.codeId = verifierCodeId || verifierConfig.codeId;
        proverConfig.codeId = proverCodeId || proverConfig.codeId;
    }

    const coordinator = new CoordinatorManager(config);
    const message = coordinator.constructExecuteMessage(chainName, salt, admin);

    const proposalId = await execute(
        client,
        config,
        {
            ...options,
            contractName: 'Coordinator',
            msg: JSON.stringify(message),
        },
        undefined,
        fee,
    );

    if (!config.axelar.contracts.Coordinator.deployments) {
        config.axelar.contracts.Coordinator.deployments = {};
    }
    config.axelar.contracts.Coordinator.deployments[chainName] = {
        deploymentName: message.instantiate_chain_contracts.deployment_name,
        salt: salt,
        proposalId,
    };
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

    if (!confirmProposalSubmission(options, proposal, UpdateInstantiateConfigProposal)) {
        return;
    }

    try {
        await submitProposal(client, config, updateOptions, proposal, fee);
        printInfo('Instantiate params proposal successfully submitted');
    } catch (e) {
        printError(`Error: ${e}`);
    }
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
const registerDeployment = async (client, config, options, _args, fee) => {
    const { chainName } = options;
    const coordinator = new CoordinatorManager(config);
    const message = coordinator.constructRegisterDeploymentMessage(chainName);
    const proposalId = await execute(
        client,
        config,
        { ...options, contractName: 'Coordinator', msg: JSON.stringify(message) },
        undefined,
        fee,
    );
    return proposalId;
};

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

    const executeCmd = program
        .command('execute')
        .description('Submit an execute wasm contract proposal')
        .action((options) => mainProcessor(execute, options));
    addAmplifierOptions(executeCmd, {
        contractOptions: true,
        executeProposalOptions: true,
        proposalOptions: true,
        runAs: true,
    });

    const registerItsChainCmd = program
        .command('its-hub-register-chains')
        .description('Submit an execute wasm contract proposal to register or update an InterchainTokenService chain')
        .argument('<chains...>', 'list of chains to register or update on InterchainTokenService hub')
        .addOption(
            new Option(
                '--its-msg-translator <itsMsgTranslator>',
                'address for the message translation contract associated with the chain being registered or updated on ITS Hub',
            ),
        )
        .addOption(
            new Option(
                '--its-edge-contract <itsEdgeContract>',
                'address for the ITS edge contract associated with the chain being registered or updated on ITS Hub',
            ),
        )
        .addOption(new Option('--update', 'update existing chain registration instead of registering new chain'))
        .action((chains, options) => {
            options.chains = chains;
            return mainProcessor(registerItsChain, options);
        });
    addAmplifierOptions(registerItsChainCmd, { proposalOptions: true, runAs: true });

    const registerProtocolCmd = program
        .command('register-protocol-contracts')
        .description('Submit an execute wasm contract proposal to register the main protocol contracts (e.g. Router)')
        .action((options) => mainProcessor(registerProtocol, options));
    addAmplifierOptions(registerProtocolCmd, { proposalOptions: true, runAs: true });

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
    });

    const instantiateChainContractsCmd = program
        .command('instantiate-chain-contracts')
        .description(
            'Submit an execute wasm contract proposal to instantiate Gateway, VotingVerifier and MultisigProver contracts via Coordinator',
        )
        .requiredOption('-n, --chainName <chainName>', 'chain name')
        .requiredOption('-s, --salt <salt>', 'salt for instantiate2')
        .option('--gatewayCodeId <gatewayCodeId>', 'code ID for Gateway contract')
        .option('--verifierCodeId <verifierCodeId>', 'code ID for VotingVerifier contract')
        .option('--proverCodeId <proverCodeId>', 'code ID for MultisigProver contract')
        .action((options) => mainProcessor(instantiateChainContracts, options));
    addAmplifierOptions(instantiateChainContractsCmd, {
        proposalOptions: true,
        runAs: true,
        fetchCodeId: true,
        instantiateOptions: true,
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
            proposalOptions: true,
        },
    );

    const registerDeploymentCmd = program
        .command('register-deployment')
        .description('Submit an execute wasm contract proposal to register a deployment')
        .requiredOption('-n, --chainName <chainName>', 'chain name')
        .action((options) => mainProcessor(registerDeployment, options));
    addAmplifierOptions(registerDeploymentCmd, {
        proposalOptions: true,
        runAs: true,
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
