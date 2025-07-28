'use strict';

require('../common/cli-utils');

const { createHash } = require('crypto');

const { instantiate2Address } = require('@cosmjs/cosmwasm-stargate');

const {
    CONTRACTS,
    prepareWallet,
    prepareClient,
    fromHex,
    getSalt,
    initContractConfig,
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
    governanceAddress,
} = require('./utils');
const {
    saveConfig,
    loadConfig,
    printInfo,
    prompt,
    getChainConfig,
    itsEdgeContract,
    readContractCode,
    getProposalConfig,
} = require('../common');
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
    contractBaseConfig.storeCodeProposalCodeHash = createHash('sha256').update(readContractCode(options)).digest().toString('hex');
};

const storeInstantiate = async (client, wallet, config, options) => {
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

    const proposalId = await callSubmitProposal(client, wallet, config, options, proposal);

    contractConfig.storeInstantiateProposalId = proposalId;
    contractBaseConfig.storeCodeProposalCodeHash = createHash('sha256').update(readContractCode(options)).digest().toString('hex');
};

const instantiate = async (client, wallet, config, options) => {
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

    const proposalId = await callSubmitProposal(client, wallet, config, options, proposal);

    contractConfig.instantiateProposalId = proposalId;
    if (instantiate2) contractConfig.address = contractAddress;
};

const execute = async (client, wallet, config, options) => {
    const { chainName } = options;

    const proposal = encodeExecuteContractProposal(config, options, chainName);

    if (!confirmProposalSubmission(options, proposal, ExecuteContractProposal)) {
        return;
    }

    await callSubmitProposal(client, wallet, config, options, proposal);
};

const registerItsChain = async (client, wallet, config, options) => {
    const chains = options.chains.map((chain) => {
        const chainConfig = getChainConfig(config, chain);
        const { maxUintBits, maxDecimalsWhenTruncating } = getChainTruncationParams(config, chainConfig);

        return {
            chain: chainConfig.axelarId,
            its_edge_contract: itsEdgeContract(chainConfig),
            truncation: {
                max_uint_bits: maxUintBits,
                max_decimals_when_truncating: maxDecimalsWhenTruncating,
            },
        };
    });

    await execute(client, wallet, config, {
        ...options,
        contractName: 'InterchainTokenService',
        msg: `{ "register_chains": { "chains": ${JSON.stringify(chains)} } }`,
    });
};

const registerProtocol = async (client, wallet, config, options) => {
    const serviceRegistry = config.axelar?.contracts?.ServiceRegistry?.address;
    const router = config.axelar?.contracts?.Router?.address;
    const multisig = config.axelar?.contracts?.Multisig?.address;

    await execute(client, wallet, config, {
        ...options,
        contractName: 'Coordinator',
        msg: JSON.stringify({
            register_protocol: {
                service_registry_address: serviceRegistry,
                router_address: router,
                multisig_address: multisig,
            },
        }),
    });
};

const paramChange = async (client, wallet, config, options) => {
    const proposal = encodeParameterChangeProposal(options);

    if (!confirmProposalSubmission(options, proposal, ParameterChangeProposal)) {
        return;
    }

    await callSubmitProposal(client, wallet, config, options, proposal);
};

const migrate = async (client, wallet, config, options) => {
    const { contractConfig } = getAmplifierContractConfig(config, options);
    contractConfig.codeId = await getCodeId(client, config, options);

    const proposal = encodeMigrateContractProposal(config, options);

    if (!confirmProposalSubmission(options, proposal, MigrateContractProposal)) {
        return;
    }

    await callSubmitProposal(client, wallet, config, options, proposal);
};

function addGovProposalDefaults(options, config, env) {
    const { runAs, deposit, instantiateAddresses } = options;

    if (!runAs) options.runAs = env == 'devnet-amplifier' ? 'axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9' : governanceAddress;

    if (!deposit) options.deposit = getProposalConfig(config, env, 'govProposalDepositAmount');

    if (!instantiateAddresses) options.instantiateAddresses = getProposalConfig(config, env, 'govProposalInstantiateAddresses');

    return options;
}

const mainProcessor = async (processor, options) => {
    const { env } = options;
    const config = loadConfig(env);
    addGovProposalDefaults(options, config, env);

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
        .description('Submit an execute wasm contract proposal to register an InterchainTokenService chain')
        .argument('<chains...>', 'list of chains to register on InterchainTokenService hub')
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

    program.parse();
};

if (require.main === module) {
    programHandler();
}
