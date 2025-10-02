'use strict';

require('../common/cli-utils');

const { createHash } = require('crypto');

const { instantiate2Address } = require('@cosmjs/cosmwasm-stargate');

const {
    CONTRACTS,
    fromHex,
    getSalt,
    getAmplifierBaseContractConfig,
    getAmplifierContractConfig,
    getCodeId,
    getChainTruncationParams,
    decodeProposalAttributes,
    encodeStoreCodeProposal,
    encodeStoreInstantiateProposal,
    encodeInstantiateProposal,
    encodeInstantiate2Proposal,
    encodeExecuteContractProposal,
    encodeParameterChangeProposal,
    encodeMigrateContractProposal,
    submitProposal,
    getInstantiateChainContractsMessage,
    validateItsChainChange,
    // V0.50!
    encodeExecuteContractMessageV50,
    submitProposalV50,
    encodeStoreCodeMessageV50,
} = require('./utils');
const { printInfo, prompt, getChainConfig, itsEdgeContract, readContractCode } = require('../common');
const {
    StoreCodeProposal,
    StoreAndInstantiateContractProposal,
    InstantiateContractProposal,
    InstantiateContract2Proposal,
    ExecuteContractProposal,
    MigrateContractProposal,
} = require('cosmjs-types/cosmwasm/wasm/v1/proposal');
const { ParameterChangeProposal } = require('cosmjs-types/cosmos/params/v1beta1/params');
// V0.50! message decoding
const { MsgExecuteContract, MsgStoreCode } = require('cosmjs-types/cosmwasm/wasm/v1/tx');

const { Command, Option } = require('commander');
const { addAmplifierOptions } = require('./cli-utils');
const { mainProcessor } = require('./processor');

const predictAddress = async (client, contractConfig, options) => {
    const { contractName, salt, chainName, runAs } = options;

    const { checksum } = await client.getCodeDetails(contractConfig.codeId);
    const contractAddress = instantiate2Address(fromHex(checksum), runAs, getSalt(salt, contractName, chainName), 'axelar');

    printInfo(`Predicted address for ${chainName ? chainName.concat(' ') : ''}${contractName}. Address`, contractAddress);

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

// V0.50!
// Updated printProposalV50 function
const printProposalV50 = (messages) => {
    messages.forEach((message) => {
        const typeMap = {
            '/cosmwasm.wasm.v1.MsgExecuteContract': MsgExecuteContract,
            '/cosmwasm.wasm.v1.MsgStoreCode': MsgStoreCode,
        };

        const MessageType = typeMap[message.typeUrl];
        if (MessageType) {
            const decoded = MessageType.decode(message.value);

            // Special handling for MsgExecuteContract - decode the msg field
            if (message.typeUrl === '/cosmwasm.wasm.v1.MsgExecuteContract' && decoded.msg) {
                decoded.msg = JSON.parse(Buffer.from(decoded.msg).toString());
            }

            // Special handling for large fields
            if (decoded.wasmByteCode) {
                decoded.wasmByteCode = `<${decoded.wasmByteCode.length} bytes>`;
            }

            printInfo(`Encoded ${message.typeUrl}`, JSON.stringify(decoded, null, 2));
        } else {
            printInfo(`Encoded ${message.typeUrl}`, '<Unable to decode>');
        }
    });
};

// V0.50!
const confirmProposalSubmissionV50 = (options, messages) => {
    printProposalV50(messages);

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

// const storeCode = async (client, config, options, _args, fee) => {
//     const { contractName } = options;
//     const contractBaseConfig = getAmplifierBaseContractConfig(config, contractName);

//     const proposal = encodeStoreCodeProposal(options);

//     if (!confirmProposalSubmission(options, proposal, StoreCodeProposal)) {
//         return;
//     }

//     const proposalId = await callSubmitProposal(client, config, options, proposal, fee);

//     contractBaseConfig.storeCodeProposalId = proposalId;
//     contractBaseConfig.storeCodeProposalCodeHash = createHash('sha256').update(readContractCode(options)).digest().toString('hex');
// };

// V0.50!
const storeCode = async (client, config, options, _args, fee) => {
    const { contractName } = options;
    const contractBaseConfig = getAmplifierBaseContractConfig(config, contractName);

    const storeMsg = encodeStoreCodeMessageV50(options);
    const messages = [storeMsg];

    if (!confirmProposalSubmissionV50(options, messages)) {
        return;
    }

    const proposalId = await submitProposalV50(client, config, options, messages, fee);
    printInfo('Proposal submitted', proposalId);

    contractBaseConfig.storeCodeProposalId = proposalId;
    contractBaseConfig.storeCodeProposalCodeHash = createHash('sha256').update(readContractCode(options)).digest().toString('hex');

    return proposalId;
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

// const execute = async (client, config, options, _args, fee) => {
//     const { chainName } = options;

//     const proposal = encodeExecuteContractProposal(config, options, chainName);

//     if (!confirmProposalSubmission(options, proposal, ExecuteContractProposal)) {
//         return;
//     }

//     return callSubmitProposal(client, config, options, proposal, fee);
// };

// V0.50!
const execute = async (client, config, options, _args, fee) => {
    const { chainName } = options;

    const executeMsg = encodeExecuteContractMessageV50(config, options, chainName);
    const messages = [executeMsg];

    if (!confirmProposalSubmissionV50(options, messages)) {
        return;
    }

    return submitProposalV50(client, config, options, messages, fee);
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
    const { chainName } = options;

    const coordinatorAddress = config.axelar?.contracts?.Coordinator?.address;
    if (!coordinatorAddress) {
        throw new Error('Coordinator contract address not found in config');
    }

    const message = await getInstantiateChainContractsMessage(client, config, options);

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
        salt: options.salt,
        proposalId,
    };
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

    program.parse();
};

if (require.main === module) {
    programHandler();
}
