'use strict';

const zlib = require('zlib');
const { createHash } = require('crypto');

const { readFileSync } = require('fs');
const { calculateFee, GasPrice } = require('@cosmjs/stargate');
const { SigningCosmWasmClient } = require('@cosmjs/cosmwasm-stargate');
const { DirectSecp256k1HdWallet } = require('@cosmjs/proto-signing');
const { MsgSubmitProposal } = require('cosmjs-types/cosmos/gov/v1beta1/tx');
const {
    StoreCodeProposal,
    StoreAndInstantiateContractProposal,
    InstantiateContractProposal,
    InstantiateContract2Proposal,
    ExecuteContractProposal,
    MigrateContractProposal,
} = require('cosmjs-types/cosmwasm/wasm/v1/proposal');
const { ParameterChangeProposal } = require('cosmjs-types/cosmos/params/v1beta1/params');
const { AccessType } = require('cosmjs-types/cosmwasm/wasm/v1/types');
const {
    printInfo,
    printWarn,
    isString,
    isStringArray,
    isKeccak256Hash,
    isNumber,
    toBigNumberString,
    getChainConfig,
    getSaltFromKey,
    calculateDomainSeparator,
    validateParameters,
} = require('../common');
const { normalizeBech32 } = require('@cosmjs/encoding');

const DEFAULT_MAX_UINT_BITS_EVM = 256;
const DEFAULT_MAX_DECIMALS_WHEN_TRUNCATING_EVM = 255;

const CONTRACT_SCOPE_GLOBAL = 'global';
const CONTRACT_SCOPE_CHAIN = 'chain';

const governanceAddress = 'axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj';

const prepareWallet = async ({ mnemonic }) => await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, { prefix: 'axelar' });

const prepareClient = async ({ axelar: { rpc, gasPrice } }, wallet) =>
    await SigningCosmWasmClient.connectWithSigner(rpc, wallet, { gasPrice });

const pascalToSnake = (str) => str.replace(/([A-Z])/g, (group) => `_${group.toLowerCase()}`).replace(/^_/, '');

const isValidCosmosAddress = (str) => {
    try {
        normalizeBech32(str);

        return true;
    } catch (error) {
        return false;
    }
};

const fromHex = (str) => new Uint8Array(Buffer.from(str.replace('0x', ''), 'hex'));

const getSalt = (salt, contractName, chainName) => fromHex(getSaltFromKey(salt || contractName.concat(chainName)));

const getLabel = ({ contractName, label }) => label || contractName;

const readWasmFile = ({ artifactPath, contractName }) => readFileSync(`${artifactPath}/${pascalToSnake(contractName)}.wasm`);

const initContractConfig = (config, { contractName, chainName }) => {
    if (!contractName) {
        return;
    }

    config.axelar = config.axelar || {};
    config.axelar.contracts = config.axelar.contracts || {};
    config.axelar.contracts[contractName] = config.axelar.contracts[contractName] || {};

    if (chainName) {
        config.axelar.contracts[contractName][chainName] = config.axelar.contracts[contractName][chainName] || {};
    }
};

const getAmplifierBaseContractConfig = (config, contractName) => {
    const contractBaseConfig = config.axelar.contracts[contractName];

    if (!contractBaseConfig) {
        throw new Error(`Contract ${contractName} not found in config`);
    }

    return contractBaseConfig;
};

const getAmplifierContractConfig = (config, { contractName, chainName }) => {
    const contractBaseConfig = getAmplifierBaseContractConfig(config, contractName);

    if (!chainName) {
        return { contractBaseConfig, contractConfig: contractBaseConfig }; // contractConfig is the same for non-chain specific contracts
    }

    const contractConfig = contractBaseConfig[chainName];

    if (!contractConfig) {
        throw new Error(`Contract ${contractName} (${chainName}) not found in config`);
    }

    return { contractBaseConfig, contractConfig };
};

const getCodeId = async (client, config, options) => {
    const { fetchCodeId, codeId, contractName } = options;

    const contractBaseConfig = getAmplifierBaseContractConfig(config, contractName);

    if (codeId) {
        return codeId;
    }

    if (fetchCodeId) {
        return fetchCodeIdFromCodeHash(client, contractBaseConfig);
    }

    if (contractBaseConfig.lastUploadedCodeId) {
        return contractBaseConfig.lastUploadedCodeId;
    }

    throw new Error('Code Id is not defined');
};

const uploadContract = async (client, wallet, config, options) => {
    const {
        axelar: { gasPrice, gasLimit },
    } = config;

    const [account] = await wallet.getAccounts();
    const wasm = readWasmFile(options);

    const uploadFee = gasLimit === 'auto' ? 'auto' : calculateFee(gasLimit, GasPrice.fromString(gasPrice));

    // uploading through stargate doesn't support defining instantiate permissions
    return await client.upload(account.address, wasm, uploadFee);
};

const instantiateContract = async (client, wallet, initMsg, config, options) => {
    const { contractName, salt, instantiate2, chainName, admin } = options;

    const [account] = await wallet.getAccounts();

    const { contractConfig } = getAmplifierContractConfig(config, options);

    const {
        axelar: { gasPrice, gasLimit },
    } = config;
    const initFee = gasLimit === 'auto' ? 'auto' : calculateFee(gasLimit, GasPrice.fromString(gasPrice));

    const contractLabel = getLabel(options);

    const { contractAddress } = instantiate2
        ? await client.instantiate2(
              account.address,
              contractConfig.codeId,
              getSalt(salt, contractName, chainName),
              initMsg,
              contractLabel,
              initFee,
              { admin },
          )
        : await client.instantiate(account.address, contractConfig.codeId, initMsg, contractLabel, initFee, {
              admin,
          });

    return contractAddress;
};

const validateAddress = (address) => {
    return isString(address) && isValidCosmosAddress(address);
};

const makeCoordinatorInstantiateMsg = (config, _options, contractConfig) => {
    const {
        axelar: { contracts },
    } = config;
    const {
        ServiceRegistry: { address: registryAddress },
    } = contracts;
    const { governanceAddress } = contractConfig;

    if (!validateAddress(governanceAddress)) {
        throw new Error('Missing or invalid Coordinator.governanceAddress in axelar info');
    }

    if (!validateAddress(registryAddress)) {
        throw new Error('Missing or invalid ServiceRegistry.address in axelar info');
    }

    return { governance_address: governanceAddress, service_registry: registryAddress };
};

const makeServiceRegistryInstantiateMsg = (_config, _options, contractConfig) => {
    const { governanceAccount } = contractConfig;

    if (!validateAddress(governanceAccount)) {
        throw new Error('Missing or invalid ServiceRegistry.governanceAccount in axelar info');
    }

    return { governance_account: governanceAccount };
};

const makeMultisigInstantiateMsg = (config, _options, contractConfig) => {
    const {
        axelar: { contracts },
    } = config;
    const {
        Rewards: { address: rewardsAddress },
    } = contracts;
    const { adminAddress, governanceAddress, blockExpiry } = contractConfig;

    if (!validateAddress(adminAddress)) {
        throw new Error('Missing or invalid Multisig.adminAddress in axelar info');
    }

    if (!validateAddress(governanceAddress)) {
        throw new Error('Missing or invalid Multisig.governanceAddress in axelar info');
    }

    if (!validateAddress(rewardsAddress)) {
        throw new Error('Missing or invalid Rewards.address in axelar info');
    }

    if (!isNumber(blockExpiry)) {
        throw new Error(`Missing or invalid Multisig.blockExpiry in axelar info`);
    }

    return {
        admin_address: adminAddress,
        governance_address: governanceAddress,
        rewards_address: rewardsAddress,
        block_expiry: toBigNumberString(blockExpiry),
    };
};

const makeRewardsInstantiateMsg = (_config, _options, contractConfig) => {
    const { governanceAddress, rewardsDenom } = contractConfig;

    if (!validateAddress(governanceAddress)) {
        throw new Error('Missing or invalid Rewards.governanceAddress in axelar info');
    }

    if (!isString(rewardsDenom)) {
        throw new Error('Missing or invalid Rewards.rewardsDenom in axelar info');
    }

    return { governance_address: governanceAddress, rewards_denom: rewardsDenom };
};

const makeRouterInstantiateMsg = (config, _options, contractConfig) => {
    const {
        axelar: { contracts },
    } = config;
    const {
        AxelarnetGateway: { address: axelarnetGateway },
    } = contracts;
    const { adminAddress, governanceAddress } = contractConfig;

    if (!validateAddress(adminAddress)) {
        throw new Error('Missing or invalid Router.adminAddress in axelar info');
    }

    if (!validateAddress(governanceAddress)) {
        throw new Error('Missing or invalid Router.governanceAddress in axelar info');
    }

    if (!validateAddress(axelarnetGateway)) {
        throw new Error('Missing or invalid AxelarnetGateway.address in axelar info');
    }

    return { admin_address: adminAddress, governance_address: governanceAddress, axelarnet_gateway: axelarnetGateway };
};

const makeVotingVerifierInstantiateMsg = (config, options, contractConfig) => {
    const { chainName } = options;
    const {
        axelar: { contracts },
    } = config;
    const {
        ServiceRegistry: { address: serviceRegistryAddress },
        Rewards: { address: rewardsAddress },
    } = contracts;
    const {
        governanceAddress,
        serviceName,
        sourceGatewayAddress,
        votingThreshold,
        blockExpiry,
        confirmationHeight,
        msgIdFormat,
        addressFormat,
    } = contractConfig;

    if (!validateAddress(serviceRegistryAddress)) {
        throw new Error('Missing or invalid ServiceRegistry.address in axelar info');
    }

    if (!validateAddress(rewardsAddress)) {
        throw new Error('Missing or invalid Rewards.address in axelar info');
    }

    if (!validateAddress(governanceAddress)) {
        throw new Error(`Missing or invalid VotingVerifier[${chainName}].governanceAddress in axelar info`);
    }

    if (!isString(serviceName)) {
        throw new Error(`Missing or invalid VotingVerifier[${chainName}].serviceName in axelar info`);
    }

    if (!isString(sourceGatewayAddress)) {
        throw new Error(`Missing or invalid VotingVerifier[${chainName}].sourceGatewayAddress in axelar info`);
    }

    if (!isStringArray(votingThreshold)) {
        throw new Error(`Missing or invalid VotingVerifier[${chainName}].votingThreshold in axelar info`);
    }

    if (!isNumber(blockExpiry)) {
        throw new Error(`Missing or invalid VotingVerifier[${chainName}].blockExpiry in axelar info`);
    }

    if (!isNumber(confirmationHeight)) {
        throw new Error(`Missing or invalid VotingVerifier[${chainName}].confirmationHeight in axelar info`);
    }

    if (!isString(msgIdFormat)) {
        throw new Error(`Missing or invalid VotingVerifier[${chainName}].msgIdFormat in axelar info`);
    }

    if (!isString(addressFormat)) {
        throw new Error(`Missing or invalid VotingVerifier[${chainName}].addressFormat in axelar info`);
    }

    return {
        service_registry_address: serviceRegistryAddress,
        rewards_address: rewardsAddress,
        governance_address: governanceAddress,
        service_name: serviceName,
        source_gateway_address: sourceGatewayAddress,
        voting_threshold: votingThreshold,
        block_expiry: toBigNumberString(blockExpiry),
        confirmation_height: confirmationHeight,
        source_chain: chainName,
        msg_id_format: msgIdFormat,
        address_format: addressFormat,
    };
};

const makeGatewayInstantiateMsg = (config, options, _contractConfig) => {
    const { chainName } = options;
    const {
        axelar: {
            contracts: {
                Router: { address: routerAddress },
                VotingVerifier: {
                    [chainName]: { address: verifierAddress },
                },
            },
        },
    } = config;

    if (!validateAddress(routerAddress)) {
        throw new Error('Missing or invalid Router.address in axelar info');
    }

    if (!validateAddress(verifierAddress)) {
        throw new Error(`Missing or invalid VotingVerifier[${chainName}].address in axelar info`);
    }

    return { router_address: routerAddress, verifier_address: verifierAddress };
};

const makeMultisigProverInstantiateMsg = (config, options, contractConfig) => {
    const { chainName } = options;
    const {
        axelar: { contracts, chainId: axelarChainId },
    } = config;
    const {
        Router: { address: routerAddress },
        Coordinator: { address: coordinatorAddress },
        Multisig: { address: multisigAddress },
        ServiceRegistry: { address: serviceRegistryAddress },
        VotingVerifier: {
            [chainName]: { address: verifierAddress },
        },
        Gateway: {
            [chainName]: { address: gatewayAddress },
        },
    } = contracts;
    const { adminAddress, governanceAddress, domainSeparator, signingThreshold, serviceName, verifierSetDiffThreshold, encoder, keyType } =
        contractConfig;

    if (!validateAddress(routerAddress)) {
        throw new Error('Missing or invalid Router.address in axelar info');
    }

    if (!isString(axelarChainId)) {
        throw new Error(`Missing or invalid chain ID`);
    }

    const separator = domainSeparator || calculateDomainSeparator(chainName, routerAddress, axelarChainId);
    contractConfig.domainSeparator = separator;

    if (!validateAddress(adminAddress)) {
        throw new Error(`Missing or invalid MultisigProver[${chainName}].adminAddress in axelar info`);
    }

    if (!validateAddress(governanceAddress)) {
        throw new Error(`Missing or invalid MultisigProver[${chainName}].governanceAddress in axelar info`);
    }

    if (!validateAddress(gatewayAddress)) {
        throw new Error(`Missing or invalid Gateway[${chainName}].address in axelar info`);
    }

    if (!validateAddress(coordinatorAddress)) {
        throw new Error('Missing or invalid Coordinator.address in axelar info');
    }

    if (!validateAddress(multisigAddress)) {
        throw new Error('Missing or invalid Multisig.address in axelar info');
    }

    if (!validateAddress(serviceRegistryAddress)) {
        throw new Error('Missing or invalid ServiceRegistry.address in axelar info');
    }

    if (!validateAddress(verifierAddress)) {
        throw new Error(`Missing or invalid VotingVerifier[${chainName}].address in axelar info`);
    }

    if (!isKeccak256Hash(separator)) {
        throw new Error(`Invalid MultisigProver[${chainName}].domainSeparator in axelar info`);
    }

    if (!isStringArray(signingThreshold)) {
        throw new Error(`Missing or invalid MultisigProver[${chainName}].signingThreshold in axelar info`);
    }

    if (!isString(serviceName)) {
        throw new Error(`Missing or invalid MultisigProver[${chainName}].serviceName in axelar info`);
    }

    if (!isNumber(verifierSetDiffThreshold)) {
        throw new Error(`Missing or invalid MultisigProver[${chainName}].verifierSetDiffThreshold in axelar info`);
    }

    if (!isString(encoder)) {
        throw new Error(`Missing or invalid MultisigProver[${chainName}].encoder in axelar info`);
    }

    if (!isString(keyType)) {
        throw new Error(`Missing or invalid MultisigProver[${chainName}].keyType in axelar info`);
    }

    return {
        admin_address: adminAddress,
        governance_address: governanceAddress,
        gateway_address: gatewayAddress,
        coordinator_address: coordinatorAddress,
        multisig_address: multisigAddress,
        service_registry_address: serviceRegistryAddress,
        voting_verifier_address: verifierAddress,
        domain_separator: separator.replace('0x', ''),
        signing_threshold: signingThreshold,
        service_name: serviceName,
        chain_name: chainName,
        verifier_set_diff_threshold: verifierSetDiffThreshold,
        encoder,
        key_type: keyType,
    };
};

const makeAxelarnetGatewayInstantiateMsg = (config, _options, contractConfig) => {
    const { nexus } = contractConfig;
    const {
        axelar: { contracts, axelarId },
    } = config;
    const {
        Router: { address: routerAddress },
    } = contracts;

    if (!isString(axelarId)) {
        throw new Error(`Missing or invalid axelar ID for Axelar`);
    }

    if (!validateAddress(routerAddress)) {
        throw new Error('Missing or invalid Router.address in axelar info');
    }

    return {
        nexus,
        router_address: routerAddress,
        chain_name: axelarId.toLowerCase(),
    };
};

const makeInterchainTokenServiceInstantiateMsg = (config, _options, contractConfig) => {
    const { adminAddress, governanceAddress } = contractConfig;
    const {
        axelar: { contracts },
    } = config;

    const {
        AxelarnetGateway: { address: axelarnetGatewayAddress },
    } = contracts;

    if (!validateAddress(axelarnetGatewayAddress)) {
        throw new Error('Missing or invalid AxelarnetGateway.address in axelar info');
    }

    return {
        governance_address: governanceAddress,
        admin_address: adminAddress,
        axelarnet_gateway_address: axelarnetGatewayAddress,
    };
};

const fetchCodeIdFromCodeHash = async (client, contractBaseConfig) => {
    if (!contractBaseConfig.storeCodeProposalCodeHash) {
        throw new Error('storeCodeProposalCodeHash not found in contract config');
    }

    const codes = await client.getCodes(); // TODO: create custom function to retrieve codes more efficiently and with pagination
    let codeId;

    // most likely to be near the end, so we iterate backwards. We also get the latest if there are multiple
    for (let i = codes.length - 1; i >= 0; i--) {
        if (codes[i].checksum.toUpperCase() === contractBaseConfig.storeCodeProposalCodeHash.toUpperCase()) {
            codeId = codes[i].id;
            break;
        }
    }

    if (!codeId) {
        throw new Error('codeId not found on network for the given codeHash');
    }

    contractBaseConfig.lastUploadedCodeId = codeId;

    printInfo(`Fetched code id ${codeId} from the network`);

    return codeId;
};

const addDefaultInstantiateAddresses = async (client, config, options) => {
    const { contractConfig } = getAmplifierContractConfig(config, options);

    if (!contractConfig.address) {
        return;
    }

    const contract = await client.getContract(contractConfig.address);

    let { instantiateAddresses } = options;

    if (!instantiateAddresses) {
        instantiateAddresses = [];
    }

    if (contract.admin && !instantiateAddresses.includes(contract.admin)) {
        instantiateAddresses.push(contract.admin);
        printWarn(
            `Contract ${contractConfig.address} admin address ${contract.admin} was not included in instantiateAddresses list. Adding it by default.`,
        );
    }

    if (contract.creator && !instantiateAddresses.includes(contract.creator)) {
        instantiateAddresses.push(contract.creator);
        printWarn(
            `Contract ${contractConfig.address} creator address ${contract.creator} was not included in instantiateAddresses list. Adding it by default.`,
        );
    }
};

const getChainTruncationParams = (config, chainConfig) => {
    const key = chainConfig.axelarId.toLowerCase();
    const chainTruncationParams = config.axelar.contracts.InterchainTokenService[key];

    let maxUintBits = chainTruncationParams?.maxUintBits;
    let maxDecimalsWhenTruncating = chainTruncationParams?.maxDecimalsWhenTruncating;

    // set EVM default values
    if (chainConfig.chainType === 'evm') {
        maxUintBits = maxUintBits || DEFAULT_MAX_UINT_BITS_EVM;
        maxDecimalsWhenTruncating = maxDecimalsWhenTruncating || DEFAULT_MAX_DECIMALS_WHEN_TRUNCATING_EVM;
    }

    validateParameters({ isValidNumber: { maxUintBits, maxDecimalsWhenTruncating } });

    return { maxUintBits, maxDecimalsWhenTruncating };
};

const getInstantiatePermission = (accessType, addresses) => {
    return {
        permission: accessType,
        addresses,
    };
};

const getSubmitProposalParams = (options) => {
    const { title, description, runAs } = options;

    return {
        title,
        description,
        runAs,
    };
};

const getStoreCodeParams = (options) => {
    const { source, builder, instantiateAddresses } = options;

    const wasm = readWasmFile(options);

    let codeHash;

    // source, builder and codeHash are optional, but mandatory if one is provided
    if (source && builder) {
        codeHash = createHash('sha256').update(wasm).digest();
    }

    const instantiatePermission =
        instantiateAddresses && instantiateAddresses.length > 0
            ? getInstantiatePermission(AccessType.ACCESS_TYPE_ANY_OF_ADDRESSES, instantiateAddresses)
            : getInstantiatePermission(AccessType.ACCESS_TYPE_NOBODY, []);

    return {
        ...getSubmitProposalParams(options),
        wasmByteCode: zlib.gzipSync(wasm),
        source,
        builder,
        codeHash,
        instantiatePermission,
    };
};

const getStoreInstantiateParams = (config, options, msg) => {
    const { admin } = options;

    return {
        ...getStoreCodeParams(options),
        admin,
        label: getLabel(options),
        msg: Buffer.from(JSON.stringify(msg)),
    };
};

const getInstantiateContractParams = (config, options, msg) => {
    const { admin } = options;

    const { contractConfig } = getAmplifierContractConfig(config, options);

    return {
        ...getSubmitProposalParams(options),
        admin,
        codeId: contractConfig.codeId,
        label: getLabel(options),
        msg: Buffer.from(JSON.stringify(msg)),
    };
};

const getInstantiateContract2Params = (config, options, msg) => {
    const { contractName, salt, chainName } = options;

    return {
        ...getInstantiateContractParams(config, options, msg),
        salt: getSalt(salt, contractName, chainName),
    };
};

const getExecuteContractParams = (config, options, chainName) => {
    const { contractName, msg } = options;
    const {
        axelar: {
            contracts: { [contractName]: contractConfig },
        },
    } = config;
    const chainConfig = getChainConfig(config, chainName);

    return {
        ...getSubmitProposalParams(options),
        contract: contractConfig[chainConfig?.axelarId]?.address || contractConfig.address,
        msg: Buffer.from(msg),
    };
};

const getParameterChangeParams = ({ title, description, changes }) => ({
    title,
    description,
    changes: JSON.parse(changes).map(({ value, ...rest }) => ({
        ...rest,
        value: JSON.stringify(value), // `value` must be JSON encoded: https://github.com/cosmos/cosmos-sdk/blob/9abd946ba0cdc6d0e708bf862b2ca202b13f2d7b/x/params/client/utils/utils.go#L23
    })),
});

const getMigrateContractParams = (config, options) => {
    const { msg, chainName } = options;

    const { contractConfig } = getAmplifierContractConfig(config, options);
    const chainConfig = getChainConfig(config, chainName);

    return {
        ...getSubmitProposalParams(options),
        contract: contractConfig[chainConfig?.axelarId]?.address || contractConfig.address,
        codeId: contractConfig.codeId,
        msg: Buffer.from(msg),
    };
};

const encodeStoreCodeProposal = (options) => {
    const proposal = StoreCodeProposal.fromPartial(getStoreCodeParams(options));

    return {
        typeUrl: '/cosmwasm.wasm.v1.StoreCodeProposal',
        value: Uint8Array.from(StoreCodeProposal.encode(proposal).finish()),
    };
};

const encodeStoreInstantiateProposal = (config, options, msg) => {
    const proposal = StoreAndInstantiateContractProposal.fromPartial(getStoreInstantiateParams(config, options, msg));

    return {
        typeUrl: '/cosmwasm.wasm.v1.StoreAndInstantiateContractProposal',
        value: Uint8Array.from(StoreAndInstantiateContractProposal.encode(proposal).finish()),
    };
};

const decodeProposalAttributes = (proposalJson) => {
    if (proposalJson.msg) {
        proposalJson.msg = JSON.parse(atob(proposalJson.msg));
    }

    return proposalJson;
};

const encodeInstantiateProposal = (config, options, msg) => {
    const proposal = InstantiateContractProposal.fromPartial(getInstantiateContractParams(config, options, msg));

    return {
        typeUrl: '/cosmwasm.wasm.v1.InstantiateContractProposal',
        value: Uint8Array.from(InstantiateContractProposal.encode(proposal).finish()),
    };
};

const encodeInstantiate2Proposal = (config, options, msg) => {
    const proposal = InstantiateContract2Proposal.fromPartial(getInstantiateContract2Params(config, options, msg));

    return {
        typeUrl: '/cosmwasm.wasm.v1.InstantiateContract2Proposal',
        value: Uint8Array.from(InstantiateContract2Proposal.encode(proposal).finish()),
    };
};

const encodeExecuteContractProposal = (config, options, chainName) => {
    const proposal = ExecuteContractProposal.fromPartial(getExecuteContractParams(config, options, chainName));

    return {
        typeUrl: '/cosmwasm.wasm.v1.ExecuteContractProposal',
        value: Uint8Array.from(ExecuteContractProposal.encode(proposal).finish()),
    };
};

const encodeParameterChangeProposal = (options) => {
    const proposal = ParameterChangeProposal.fromPartial(getParameterChangeParams(options));

    return {
        typeUrl: '/cosmos.params.v1beta1.ParameterChangeProposal',
        value: Uint8Array.from(ParameterChangeProposal.encode(proposal).finish()),
    };
};

const encodeMigrateContractProposal = (config, options) => {
    const proposal = MigrateContractProposal.fromPartial(getMigrateContractParams(config, options));

    return {
        typeUrl: '/cosmwasm.wasm.v1.MigrateContractProposal',
        value: Uint8Array.from(MigrateContractProposal.encode(proposal).finish()),
    };
};

const encodeSubmitProposal = (content, config, options, proposer) => {
    const {
        axelar: { tokenSymbol },
    } = config;
    const { deposit } = options;

    return {
        typeUrl: '/cosmos.gov.v1beta1.MsgSubmitProposal',
        value: MsgSubmitProposal.fromPartial({
            content,
            initialDeposit: [{ denom: `u${tokenSymbol.toLowerCase()}`, amount: deposit }],
            proposer,
        }),
    };
};

const submitProposal = async (client, wallet, config, options, content) => {
    const [account] = await wallet.getAccounts();

    const {
        axelar: { gasPrice, gasLimit },
    } = config;

    const submitProposalMsg = encodeSubmitProposal(content, config, options, account.address);

    const fee = gasLimit === 'auto' ? 'auto' : calculateFee(gasLimit, GasPrice.fromString(gasPrice));
    const { events } = await client.signAndBroadcast(account.address, [submitProposalMsg], fee, '');

    return events.find(({ type }) => type === 'submit_proposal').attributes.find(({ key }) => key === 'proposal_id').value;
};

const CONTRACTS = {
    Coordinator: {
        scope: CONTRACT_SCOPE_GLOBAL,
        makeInstantiateMsg: makeCoordinatorInstantiateMsg,
    },
    ServiceRegistry: {
        scope: CONTRACT_SCOPE_GLOBAL,
        makeInstantiateMsg: makeServiceRegistryInstantiateMsg,
    },
    Multisig: {
        scope: CONTRACT_SCOPE_GLOBAL,
        makeInstantiateMsg: makeMultisigInstantiateMsg,
    },
    Rewards: {
        scope: CONTRACT_SCOPE_GLOBAL,
        makeInstantiateMsg: makeRewardsInstantiateMsg,
    },
    Router: {
        scope: CONTRACT_SCOPE_GLOBAL,
        makeInstantiateMsg: makeRouterInstantiateMsg,
    },
    VotingVerifier: {
        scope: CONTRACT_SCOPE_CHAIN,
        makeInstantiateMsg: makeVotingVerifierInstantiateMsg,
    },
    Gateway: {
        scope: CONTRACT_SCOPE_CHAIN,
        makeInstantiateMsg: makeGatewayInstantiateMsg,
    },
    MultisigProver: {
        scope: CONTRACT_SCOPE_CHAIN,
        makeInstantiateMsg: makeMultisigProverInstantiateMsg,
    },
    AxelarnetGateway: {
        scope: CONTRACT_SCOPE_GLOBAL,
        makeInstantiateMsg: makeAxelarnetGatewayInstantiateMsg,
    },
    InterchainTokenService: {
        scope: CONTRACT_SCOPE_GLOBAL,
        makeInstantiateMsg: makeInterchainTokenServiceInstantiateMsg,
    },
};

module.exports = {
    CONTRACT_SCOPE_CHAIN,
    CONTRACT_SCOPE_GLOBAL,
    CONTRACTS,
    governanceAddress,
    prepareWallet,
    prepareClient,
    fromHex,
    getSalt,
    calculateDomainSeparator,
    readWasmFile,
    initContractConfig,
    getAmplifierBaseContractConfig,
    getAmplifierContractConfig,
    getCodeId,
    uploadContract,
    instantiateContract,
    fetchCodeIdFromCodeHash,
    addDefaultInstantiateAddresses,
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
    isValidCosmosAddress,
};
