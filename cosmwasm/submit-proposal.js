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
    encodeStoreCode,
    encodeStoreInstantiate,
    encodeInstantiate,
    encodeExecuteContract,
    encodeMigrate,
    encodeUpdateInstantiateConfigProposal,
    getNexusProtoType,
    submitProposal,
    GOVERNANCE_MODULE_ADDRESS,
    encodeChainStatusRequest,
    validateGovernanceMode,
} = require('./utils');
const { printInfo, prompt, getChainConfig, readContractCode } = require('../common');
const { printProposal, confirmProposalSubmission, submitProposalAndPrint } = require('./proposal-utils');
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
    const { contractName, salt, chainName } = options;

    const { checksum } = await client.getCodeDetails(contractConfig.codeId);
    const contractAddress = instantiate2Address(
        fromHex(checksum),
        GOVERNANCE_MODULE_ADDRESS,
        getSalt(salt, contractName, chainName),
        'axelar',
    );

    printInfo(`Predicted address for ${chainName ? chainName.concat(' ') : ''}${contractName}. Address`, contractAddress);

    return contractAddress;
};

const saveStoreCodeProposalInfo = (config, contractName, contractCodePath, proposalId) => {
    const contractBaseConfig = config.getContractConfig(contractName);
    contractBaseConfig.storeCodeProposalId = proposalId;

    const contractOptions = { contractName, contractCodePath };
    contractBaseConfig.storeCodeProposalCodeHash = createHash('sha256').update(readContractCode(contractOptions)).digest().toString('hex');
};

const storeCode = async (client, config, options, _args, fee) => {
    let contractName = options.contractName;
    const { contractCodePath, contractCodePaths } = options;

    if (!Array.isArray(contractName)) {
        contractName = [contractName];
    }

    const contractNames = contractName;
    const proposal = contractNames.map((name) => {
        const contractOptions = {
            ...options,
            contractName: name,
            contractCodePath: contractCodePaths ? contractCodePaths[name] : contractCodePath,
        };
        return encodeStoreCode(contractOptions);
    });

    if (!confirmProposalSubmission(options, proposal)) {
        return;
    }
    const proposalId = await submitProposalAndPrint(client, config, options, proposal, fee);
    contractNames.forEach((name) => {
        const codePath = contractCodePaths ? contractCodePaths[name] : contractCodePath;
        saveStoreCodeProposalInfo(config, name, codePath, proposalId);
    });
    return proposalId;
};

const storeInstantiate = async (client, config, options, _args, fee) => {
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
    const proposal = encodeStoreInstantiate({ ...options, contractName }, initMsg);

    if (!confirmProposalSubmission(options, [proposal])) {
        return;
    }
    const proposalId = await submitProposalAndPrint(client, config, options, [proposal], fee);

    contractConfig.storeInstantiateProposalId = proposalId;
    contractBaseConfig.storeCodeProposalCodeHash = createHash('sha256')
        .update(readContractCode({ ...options, contractName }))
        .digest()
        .toString('hex');
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

    if (!confirmProposalSubmission(options, [proposal])) {
        return;
    }
    const proposalId = await submitProposalAndPrint(client, config, options, [proposal], fee);
    contractConfig.instantiateProposalId = proposalId;
    if (instantiate2) {
        contractConfig.address = contractAddress;
    }
};

const executeByGovernance = async (client, config, options, _args, fee) => {
    const { chainName, dryRun } = options;
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

    const { msg } = options;
    const msgs = toArray(msg);

    const messages = msgs.map((msgJson) => {
        const msgOptions = { ...options, contractName: singleContractName, msg: msgJson };
        return encodeExecuteContract(config, msgOptions, chainName);
    });

    if (dryRun) {
        const contractConfig = config.axelar.contracts[singleContractName];
        const chainConfig = chainName ? getChainConfig(config.chains, chainName) : null;
        const contractAddress = contractConfig[chainConfig?.axelarId]?.address || contractConfig.address;

        const dryRunOutput = messages.map((message, index) => ({
            '@type': '/cosmwasm.wasm.v1.MsgExecuteContract',
            sender: GOVERNANCE_MODULE_ADDRESS,
            contract: contractAddress,
            msg: JSON.parse(msgs[index]),
            funds: [],
        }));

        console.log(JSON.stringify(dryRunOutput, null, 2));
        return;
    }

    if (!confirmProposalSubmission(options, messages)) {
        return;
    }

    return submitProposalAndPrint(client, config, options, messages, fee);
};

const submitAxelarnetGatewayMessagesByGovernance = async (axelarnetGatewayMsgs, options, { title, description }) => {
    const submitOptions = {
        env: options.env,
        mnemonic: options.mnemonic,
        contractName: 'AxelarnetGateway',
        title,
        description,
        standardProposal: options.standardProposal,
        // Avoid a second interactive prompt: caller already confirmed.
        yes: true,
        msg: axelarnetGatewayMsgs.map((m) => JSON.stringify(m)),
    };

    const submitFn = async (client, config, _opts, _args, fee) => {
        // Fail fast if AxelarnetGateway isn't configured to be governed by the gov module account
        validateGovernanceMode(config, 'AxelarnetGateway');
        return executeByGovernance(client, config, submitOptions, [], fee);
    };

    await mainProcessor(submitFn, submitOptions);
};

const migrate = async (client, config, options, _args, fee) => {
    let { contractName } = options;

    if (Array.isArray(contractName)) {
        if (contractName.length > 1) {
            throw new Error('migrate only supports a single contract at a time');
        }
        contractName = contractName[0];
    }

    const { contractConfig } = getAmplifierContractConfig(config, { ...options, contractName });
    contractConfig.codeId = await getCodeId(client, config, { ...options, contractName });

    const proposal = encodeMigrate(config, { ...options, contractName });

    if (!confirmProposalSubmission(options, [proposal])) {
        return;
    }
    return submitProposalAndPrint(client, config, options, [proposal], fee);
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
        deposit: options.deposit,
        standardProposal: options.standardProposal,
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
    });

    const executeByGovernanceCmd = program
        .command('executeByGovernance')
        .description('Submit an execute wasm contract proposal')
        .action((options) => mainProcessor(executeByGovernance, options));
    addAmplifierOptions(executeByGovernanceCmd, {
        contractOptions: true,
        executeProposalOptions: true,
        proposalOptions: true,
    });

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

    program.parse();
};

if (require.main === module) {
    programHandler();
}

module.exports = {
    confirmProposalSubmission,
    executeByGovernance,
    migrate,
    submitAxelarnetGatewayMessagesByGovernance,
};
