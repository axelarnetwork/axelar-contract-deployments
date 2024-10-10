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
    isString,
    isStringArray,
    isStringLowercase,
    isKeccak256Hash,
    isNumber,
    toBigNumberString,
    getChainConfig,
    getSaltFromKey,
    calculateDomainSeparator,
} = require('../common');
const { normalizeBech32 } = require('@cosmjs/encoding');

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

const getAmplifierContractConfig = (config, contractName) => {
    const contractConfig = config.axelar.contracts[contractName];

    if (!contractConfig) {
        throw new Error(`Contract ${contractName} not found in config`);
    }

    return contractConfig;
};

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

const uploadContract = async (client, wallet, config, options) => {
    const {
        axelar: { gasPrice, gasLimit },
    } = config;

    const [account] = await wallet.getAccounts();
    const wasm = readWasmFile(options);

    const uploadFee = gasLimit === 'auto' ? 'auto' : calculateFee(gasLimit, GasPrice.fromString(gasPrice));

    return await client.upload(account.address, wasm, uploadFee);
};

const instantiateContract = async (client, wallet, initMsg, config, options) => {
    const { contractName, salt, instantiate2, chainName, admin } = options;

    const [account] = await wallet.getAccounts();

    const contractConfig = config.axelar.contracts[contractName];

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

const makeCoordinatorInstantiateMsg = ({ governanceAddress }, { ServiceRegistry: { address: registryAddress } }) => {
    if (!validateAddress(governanceAddress)) {
        throw new Error('Missing or invalid Coordinator.governanceAddress in axelar info');
    }

    if (!validateAddress(registryAddress)) {
        throw new Error('Missing or invalid ServiceRegistry.address in axelar info');
    }

    return { governance_address: governanceAddress, service_registry: registryAddress };
};

const makeServiceRegistryInstantiateMsg = ({ governanceAccount }) => {
    if (!validateAddress(governanceAccount)) {
        throw new Error('Missing or invalid ServiceRegistry.governanceAccount in axelar info');
    }

    return { governance_account: governanceAccount };
};

const makeMultisigInstantiateMsg = ({ adminAddress, governanceAddress, blockExpiry }, { Rewards: { address: rewardsAddress } }) => {
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

const makeRewardsInstantiateMsg = ({ governanceAddress, rewardsDenom }) => {
    if (!validateAddress(governanceAddress)) {
        throw new Error('Missing or invalid Rewards.governanceAddress in axelar info');
    }

    if (!isString(rewardsDenom)) {
        throw new Error('Missing or invalid Rewards.rewardsDenom in axelar info');
    }

    return { governance_address: governanceAddress, rewards_denom: rewardsDenom };
};

const makeRouterInstantiateMsg = ({ adminAddress, governanceAddress }, { NexusGateway: { address: nexusGateway } }) => {
    if (!validateAddress(adminAddress)) {
        throw new Error('Missing or invalid Router.adminAddress in axelar info');
    }

    if (!validateAddress(governanceAddress)) {
        throw new Error('Missing or invalid Router.governanceAddress in axelar info');
    }

    if (!validateAddress(nexusGateway)) {
        throw new Error('Missing or invalid NexusGateway.address in axelar info');
    }

    return { admin_address: adminAddress, governance_address: governanceAddress, nexus_gateway: nexusGateway };
};

const makeNexusGatewayInstantiateMsg = ({ nexus }, { Router: { address: router }, AxelarnetGateway: { address: axelarnetGateway } }) => {
    if (!validateAddress(nexus)) {
        throw new Error('Missing or invalid NexusGateway.nexus in axelar info');
    }

    if (!validateAddress(axelarnetGateway)) {
        throw new Error('Missing or invalid AxelarnetGateway.address in axelar info');
    }

    if (!validateAddress(router)) {
        throw new Error('Missing or invalid Router.address in axelar info');
    }

    return { nexus, axelarnet_gateway: axelarnetGateway, router };
};

const makeVotingVerifierInstantiateMsg = (
    contractConfig,
    { ServiceRegistry: { address: serviceRegistryAddress }, Rewards: { address: rewardsAddress } },
    { axelarId },
) => {
    const {
        [axelarId]: {
            governanceAddress,
            serviceName,
            sourceGatewayAddress,
            votingThreshold,
            blockExpiry,
            confirmationHeight,
            msgIdFormat,
            addressFormat,
        },
    } = contractConfig;

    if (!isStringLowercase(axelarId)) {
        throw new Error('Missing or invalid axelar ID');
    }

    if (!validateAddress(serviceRegistryAddress)) {
        throw new Error('Missing or invalid ServiceRegistry.address in axelar info');
    }

    if (!validateAddress(rewardsAddress)) {
        throw new Error('Missing or invalid Rewards.address in axelar info');
    }

    if (!validateAddress(governanceAddress)) {
        throw new Error(`Missing or invalid VotingVerifier[${axelarId}].governanceAddress in axelar info`);
    }

    if (!isString(serviceName)) {
        throw new Error(`Missing or invalid VotingVerifier[${axelarId}].serviceName in axelar info`);
    }

    if (!isString(sourceGatewayAddress)) {
        throw new Error(`Missing or invalid VotingVerifier[${axelarId}].sourceGatewayAddress in axelar info`);
    }

    if (!isStringArray(votingThreshold)) {
        throw new Error(`Missing or invalid VotingVerifier[${axelarId}].votingThreshold in axelar info`);
    }

    if (!isNumber(blockExpiry)) {
        throw new Error(`Missing or invalid VotingVerifier[${axelarId}].blockExpiry in axelar info`);
    }

    if (!isNumber(confirmationHeight)) {
        throw new Error(`Missing or invalid VotingVerifier[${axelarId}].confirmationHeight in axelar info`);
    }

    if (!isString(msgIdFormat)) {
        throw new Error(`Missing or invalid VotingVerifier[${axelarId}].msgIdFormat in axelar info`);
    }

    if (!isString(addressFormat)) {
        throw new Error(`Missing or invalid VotingVerifier[${axelarId}].addressFormat in axelar info`);
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
        source_chain: axelarId,
        msg_id_format: msgIdFormat,
        address_format: addressFormat,
    };
};

const makeGatewayInstantiateMsg = ({ Router: { address: routerAddress }, VotingVerifier }, { axelarId: chainId }) => {
    const {
        [chainId]: { address: verifierAddress },
    } = VotingVerifier;

    if (!validateAddress(routerAddress)) {
        throw new Error('Missing or invalid Router.address in axelar info');
    }

    if (!validateAddress(verifierAddress)) {
        throw new Error(`Missing or invalid VotingVerifier[${chainId}].address in axelar info`);
    }

    return { router_address: routerAddress, verifier_address: verifierAddress };
};

const makeMultisigProverInstantiateMsg = (config, chainName) => {
    const {
        axelar: { contracts, chainId: axelarChainId },
    } = config;
    const chainConfig = getChainConfig(config, chainName);

    const { axelarId } = chainConfig;

    const {
        Router: { address: routerAddress },
        Coordinator: { address: coordinatorAddress },
        Multisig: { address: multisigAddress },
        ServiceRegistry: { address: serviceRegistryAddress },
        VotingVerifier: {
            [axelarId]: { address: verifierAddress },
        },
        Gateway: {
            [axelarId]: { address: gatewayAddress },
        },
        MultisigProver: contractConfig,
    } = contracts;
    const {
        [axelarId]: {
            adminAddress,
            governanceAddress,
            domainSeparator,
            signingThreshold,
            serviceName,
            verifierSetDiffThreshold,
            encoder,
            keyType,
        },
    } = contractConfig;

    if (!isStringLowercase(axelarId)) {
        throw new Error(`Missing or invalid axelar ID for chain ${chainName}`);
    }

    if (!validateAddress(routerAddress)) {
        throw new Error('Missing or invalid Router.address in axelar info');
    }

    if (!isString(axelarChainId)) {
        throw new Error(`Missing or invalid chain ID`);
    }

    const separator = domainSeparator || calculateDomainSeparator(axelarId, routerAddress, axelarChainId);
    contractConfig[axelarId].domainSeparator = separator;

    if (!validateAddress(adminAddress)) {
        throw new Error(`Missing or invalid MultisigProver[${axelarId}].adminAddress in axelar info`);
    }

    if (!validateAddress(governanceAddress)) {
        throw new Error(`Missing or invalid MultisigProver[${axelarId}].governanceAddress in axelar info`);
    }

    if (!validateAddress(gatewayAddress)) {
        throw new Error(`Missing or invalid Gateway[${axelarId}].address in axelar info`);
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
        throw new Error(`Missing or invalid VotingVerifier[${axelarId}].address in axelar info`);
    }

    if (!isKeccak256Hash(separator)) {
        throw new Error(`Invalid MultisigProver[${axelarId}].domainSeparator in axelar info`);
    }

    if (!isStringArray(signingThreshold)) {
        throw new Error(`Missing or invalid MultisigProver[${axelarId}].signingThreshold in axelar info`);
    }

    if (!isString(serviceName)) {
        throw new Error(`Missing or invalid MultisigProver[${axelarId}].serviceName in axelar info`);
    }

    if (!isNumber(verifierSetDiffThreshold)) {
        throw new Error(`Missing or invalid MultisigProver[${axelarId}].verifierSetDiffThreshold in axelar info`);
    }

    if (!isString(encoder)) {
        throw new Error(`Missing or invalid MultisigProver[${axelarId}].encoder in axelar info`);
    }

    if (!isString(keyType)) {
        throw new Error(`Missing or invalid MultisigProver[${axelarId}].keyType in axelar info`);
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
        chain_name: axelarId,
        verifier_set_diff_threshold: verifierSetDiffThreshold,
        encoder,
        key_type: keyType,
    };
};

const makeAxelarnetGatewayInstantiateMsg = ({ nexus }, config, chainName) => {
    const {
        axelar: { contracts },
    } = config;
    const chainConfig = getChainConfig(config, chainName);

    const { axelarId } = chainConfig;

    const {
        Router: { address: routerAddress },
        NexusGateway: { address: nexusAddress },
    } = contracts;

    if (!isString(axelarId)) {
        throw new Error(`Missing or invalid axelar ID for chain ${chainName}`);
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

const makeInstantiateMsg = (contractName, chainName, config) => {
    const {
        axelar: { contracts },
    } = config;
    const chainConfig = getChainConfig(config, chainName);

    const { [contractName]: contractConfig } = contracts;

    switch (contractName) {
        case 'Coordinator': {
            if (chainConfig) {
                throw new Error('Coordinator does not support chainName option');
            }

            return makeCoordinatorInstantiateMsg(contractConfig, contracts);
        }

        case 'ServiceRegistry': {
            if (chainConfig) {
                throw new Error('ServiceRegistry does not support chainName option');
            }

            return makeServiceRegistryInstantiateMsg(contractConfig);
        }

        case 'Multisig': {
            if (chainConfig) {
                throw new Error('Multisig does not support chainName option');
            }

            return makeMultisigInstantiateMsg(contractConfig, contracts);
        }

        case 'Rewards': {
            if (chainConfig) {
                throw new Error('Rewards does not support chainName option');
            }

            return makeRewardsInstantiateMsg(contractConfig);
        }

        case 'Router': {
            if (chainConfig) {
                throw new Error('Router does not support chainName option');
            }

            return makeRouterInstantiateMsg(contractConfig, contracts);
        }

        case 'NexusGateway': {
            if (chainConfig) {
                throw new Error('NexusGateway does not support chainName option');
            }

            return makeNexusGatewayInstantiateMsg(contractConfig, contracts);
        }

        case 'VotingVerifier': {
            if (!chainConfig) {
                throw new Error('VotingVerifier requires chainName option');
            }

            return makeVotingVerifierInstantiateMsg(contractConfig, contracts, chainConfig);
        }

        case 'Gateway': {
            if (!chainConfig) {
                throw new Error('Gateway requires chainName option');
            }

            return makeGatewayInstantiateMsg(contracts, chainConfig);
        }

        case 'MultisigProver': {
            if (!chainConfig) {
                throw new Error('MultisigProver requires chainName option');
            }

            return makeMultisigProverInstantiateMsg(config, chainName);
        }

        case 'AxelarnetGateway': {
            if (!chainConfig) {
                throw new Error('AxelarnetGateway requires chainName option');
            }

            return makeAxelarnetGatewayInstantiateMsg(contractConfig, config, chainName);
        }
    }

    throw new Error(`${contractName} is not supported.`);
};

const fetchCodeIdFromCodeHash = async (client, contractConfig) => {
    if (!contractConfig.storeCodeProposalCodeHash) {
        throw new Error('storeCodeProposalCodeHash not found in contract config');
    }

    const codes = await client.getCodes(); // TODO: create custom function to retrieve codes more efficiently and with pagination
    let codeId;

    // most likely to be near the end, so we iterate backwards. We also get the latest if there are multiple
    for (let i = codes.length - 1; i >= 0; i--) {
        if (codes[i].checksum.toUpperCase() === contractConfig.storeCodeProposalCodeHash.toUpperCase()) {
            codeId = codes[i].id;
            break;
        }
    }

    if (!codeId) {
        throw new Error('codeId not found on network for the given codeHash');
    }

    return codeId;
};

const getInstantiatePermission = (accessType, addresses) => {
    return {
        permission: accessType,
        addresses: addresses.split(',').map((address) => address.trim()),
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

    const instantiatePermission = instantiateAddresses
        ? getInstantiatePermission(AccessType.ACCESS_TYPE_ANY_OF_ADDRESSES, instantiateAddresses)
        : getInstantiatePermission(AccessType.ACCESS_TYPE_NOBODY, '');

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
    const { contractName, admin } = options;

    const contractConfig = config.axelar.contracts[contractName];

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

const getMigrateContractParams = (config, options, chainName) => {
    const { contractName, msg } = options;

    const contractConfig = getAmplifierContractConfig(config, contractName);
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

const encodeMigrateContractProposal = (config, options, chainName) => {
    const proposal = MigrateContractProposal.fromPartial(getMigrateContractParams(config, options, chainName));

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

module.exports = {
    governanceAddress,
    prepareWallet,
    prepareClient,
    fromHex,
    getSalt,
    calculateDomainSeparator,
    readWasmFile,
    getAmplifierContractConfig,
    updateContractConfig,
    uploadContract,
    instantiateContract,
    makeInstantiateMsg,
    fetchCodeIdFromCodeHash,
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
