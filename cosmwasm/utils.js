'use strict';

const zlib = require('zlib');
const { createHash } = require('crypto');
const path = require('path');
const fs = require('fs');
const protobuf = require('protobufjs');
const { MsgSubmitProposal } = require('cosmjs-types/cosmos/gov/v1beta1/tx');
const { QueryCodeRequest, QueryCodeResponse } = require('cosmjs-types/cosmwasm/wasm/v1/query');
const { AccessType } = require('cosmjs-types/cosmwasm/wasm/v1/types');
const { MsgSubmitProposal: MsgSubmitProposalV1 } = require('cosmjs-types/cosmos/gov/v1/tx');
const {
    MsgExecuteContract,
    MsgInstantiateContract,
    MsgInstantiateContract2,
    MsgMigrateContract,
    MsgStoreCode,
    MsgStoreAndInstantiateContract,
    MsgUpdateInstantiateConfig,
} = require('cosmjs-types/cosmwasm/wasm/v1/tx');
const { Tendermint34Client } = require('@cosmjs/tendermint-rpc');
const {
    printInfo,
    isString,
    isStringArray,
    isKeccak256Hash,
    isNumber,
    toBigNumberString,
    getChainConfig,
    getSaltFromKey,
    calculateDomainSeparator,
    validateParameters,
    tryItsEdgeContract,
    itsEdgeContract,
} = require('../common');
const {
    pascalToSnake,
    pascalToKebab,
    downloadContractCode,
    readContractCode,
    VERSION_REGEX,
    SHORT_COMMIT_HASH_REGEX,
} = require('../common/utils');
const { normalizeBech32 } = require('@cosmjs/encoding');

const { GATEWAY_CONTRACT_NAME, VERIFIER_CONTRACT_NAME } = require('../common/config');
const XRPLClient = require('../xrpl/xrpl-client');

const DEFAULT_MAX_UINT_BITS_EVM = 256;
const DEFAULT_MAX_DECIMALS_WHEN_TRUNCATING_EVM = 255;

const CONTRACT_SCOPE_GLOBAL = 'global';
const CONTRACT_SCOPE_CHAIN = 'chain';

const AXELAR_R2_BASE_URL = 'https://static.axelar.network';

const GOVERNANCE_MODULE_ADDRESS = 'axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj';

const isValidCosmosAddress = (str) => {
    try {
        normalizeBech32(str);

        return true;
    } catch (error) {
        return false;
    }
};

const fromHex = (str) => new Uint8Array(Buffer.from(str.replace('0x', ''), 'hex'));

const toArray = (value) => {
    return Array.isArray(value) ? value : [value];
};

const getSalt = (salt, contractName, chainName) => fromHex(getSaltFromKey(salt || contractName.concat(chainName)));

const getLabel = ({ contractName, label }) => label || contractName;

const getAmplifierContractConfig = (config, { contractName, chainName }) => {
    const contractBaseConfig = config.getContractConfig(contractName);

    if (!chainName) {
        return { contractBaseConfig, contractConfig: contractBaseConfig }; // contractConfig is the same for non-chain specific contracts
    }

    const contractConfig = contractBaseConfig[chainName];

    if (!contractConfig) {
        throw new Error(`Contract ${contractName} (${chainName}) not found in config`);
    }

    return { contractBaseConfig, contractConfig };
};

const getUnitDenom = (config) => {
    const {
        axelar: { unitDenom },
    } = config;
    return unitDenom;
};

const validateGovernanceMode = (config, contractName, chainName) => {
    const governanceAddress = config.axelar.governanceAddress;

    if (governanceAddress !== GOVERNANCE_MODULE_ADDRESS) {
        throw new Error(
            `Contract ${contractName}${chainName ? ` (${chainName})` : ''} governanceAddress is not set to governance module address. ` +
                `Cannot use --governance flag. The proposal will fail at execution.`,
        );
    }
};

const getCodeId = async (client, config, options) => {
    const { fetchCodeId, codeId, contractName } = options;

    const contractBaseConfig = config.getContractConfig(contractName);

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

const executeTransaction = async (client, contractAddress, message, fee) => {
    const [account] = client.accounts;
    const tx = await client.execute(account.address, contractAddress, message, fee, '');
    return tx;
};

const uploadContract = async (client, options, uploadFee) => {
    const [account] = client.accounts;
    const wasm = readContractCode(options);

    // uploading through stargate doesn't support defining instantiate permissions
    return client.upload(account.address, wasm, uploadFee);
};

const instantiateContract = async (client, initMsg, config, options, initFee) => {
    const { contractName, salt, instantiate2, chainName, admin } = options;
    const [account] = client.accounts;
    const { contractConfig } = getAmplifierContractConfig(config, options);
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

const migrateContract = async (client, config, options, migrateFee) => {
    const { msg } = options;
    const [account] = client.accounts;
    const { contractConfig } = getAmplifierContractConfig(config, options);

    return client.migrate(account.address, contractConfig.address, contractConfig.codeId, JSON.parse(msg), migrateFee);
};

const validateAddress = (address) => {
    return isString(address) && isValidCosmosAddress(address);
};

const makeCoordinatorInstantiateMsg = (config, _options, contractConfig) => {
    const governanceAddress = config.axelar.governanceAddress;

    if (!validateAddress(governanceAddress)) {
        throw new Error('Missing or invalid Coordinator.governanceAddress in axelar info');
    }

    return { governance_address: governanceAddress };
};

const makeServiceRegistryInstantiateMsg = (config, _options, _contractConfig) => {
    const governanceAccount = config.axelar.governanceAddress;

    if (!validateAddress(governanceAccount)) {
        throw new Error('Missing or invalid axelar.governanceAddress in axelar info');
    }

    return { governance_account: governanceAccount };
};

const makeMultisigInstantiateMsg = (config, _options, contractConfig) => {
    const {
        axelar: { contracts },
    } = config;
    const {
        Rewards: { address: rewardsAddress },
        Coordinator: { address: coordinatorAddress },
    } = contracts;
    const { blockExpiry } = contractConfig;
    const adminAddress = contractConfig.adminAddress || config.axelar.adminAddress;
    const governanceAddress = config.axelar.governanceAddress;

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

    if (!validateAddress(coordinatorAddress)) {
        throw new Error('Missing or invalid Coordinator.address in axelar info');
    }

    return {
        admin_address: adminAddress,
        governance_address: governanceAddress,
        rewards_address: rewardsAddress,
        block_expiry: toBigNumberString(blockExpiry),
        coordinator_address: coordinatorAddress,
    };
};

const makeRewardsInstantiateMsg = (config, _options, contractConfig) => {
    const { rewardsDenom } = contractConfig;
    const governanceAddress = config.axelar.governanceAddress;

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
        Coordinator: { address: coordinator },
    } = contracts;
    const adminAddress = contractConfig.adminAddress || config.axelar.adminAddress;
    const governanceAddress = config.axelar.governanceAddress;

    if (!validateAddress(adminAddress)) {
        throw new Error('Missing or invalid Router.adminAddress in axelar info');
    }

    if (!validateAddress(governanceAddress)) {
        throw new Error('Missing or invalid Router.governanceAddress in axelar info');
    }

    if (!validateAddress(axelarnetGateway)) {
        throw new Error('Missing or invalid AxelarnetGateway.address in axelar info');
    }

    if (!validateAddress(coordinator)) {
        throw new Error('Missing or invalid Coordinator.address in axelar info');
    }

    return {
        admin_address: adminAddress,
        governance_address: governanceAddress,
        axelarnet_gateway: axelarnetGateway,
        coordinator_address: coordinator,
    };
};

const makeXrplVotingVerifierInstantiateMsg = (config, options, contractConfig) => {
    const { chainName } = options;
    const {
        axelar: { contracts },
        chains: {
            [chainName]: {
                contracts: {
                    AxelarGateway: { address: sourceGatewayAddress },
                },
            },
        },
    } = config;
    const {
        ServiceRegistry: { address: serviceRegistryAddress },
        Rewards: { address: rewardsAddress },
    } = contracts;
    const { serviceName, votingThreshold, blockExpiry, confirmationHeight } = contractConfig;
    const adminAddress = contractConfig.adminAddress || config.axelar.multisigProverAdminAddress;
    const governanceAddress = config.axelar.governanceAddress;

    if (!validateAddress(serviceRegistryAddress)) {
        throw new Error('Missing or invalid ServiceRegistry.address in axelar info');
    }

    if (!validateAddress(rewardsAddress)) {
        throw new Error('Missing or invalid Rewards.address in axelar info');
    }

    if (!validateAddress(adminAddress)) {
        throw new Error(`Missing or invalid XrplVotingVerifier[${chainName}].adminAddress in axelar info`);
    }

    if (!validateAddress(governanceAddress)) {
        throw new Error(`Missing or invalid XrplVotingVerifier[${chainName}].governanceAddress in axelar info`);
    }

    if (!isString(serviceName)) {
        throw new Error(`Missing or invalid XrplVotingVerifier[${chainName}].serviceName in axelar info`);
    }

    if (!isString(sourceGatewayAddress)) {
        throw new Error(`Missing or invalid [${chainName}].contracts.AxelarGateway.address in axelar info`);
    }

    if (!isStringArray(votingThreshold)) {
        throw new Error(`Missing or invalid XrplVotingVerifier[${chainName}].votingThreshold in axelar info`);
    }

    if (!isNumber(blockExpiry)) {
        throw new Error(`Missing or invalid XrplVotingVerifier[${chainName}].blockExpiry in axelar info`);
    }

    if (!isNumber(confirmationHeight)) {
        throw new Error(`Missing or invalid XrplVotingVerifier[${chainName}].confirmationHeight in axelar info`);
    }

    return {
        admin_address: adminAddress,
        service_registry_address: serviceRegistryAddress,
        rewards_address: rewardsAddress,
        governance_address: governanceAddress,
        service_name: serviceName,
        source_gateway_address: sourceGatewayAddress,
        voting_threshold: votingThreshold,
        block_expiry: toBigNumberString(blockExpiry),
        confirmation_height: confirmationHeight,
        source_chain: chainName,
    };
};

const makeEventVerifierInstantiateMsg = (config, _options, contractConfig) => {
    const {
        axelar: { contracts },
    } = config;
    const {
        ServiceRegistry: { address: serviceRegistryAddress },
    } = contracts;
    const { serviceName, votingThreshold, blockExpiry } = contractConfig;
    const adminAddress = contractConfig.adminAddress || config.axelar.adminAddress;
    const governanceAddress = config.axelar.governanceAddress;

    if (!validateAddress(serviceRegistryAddress)) {
        throw new Error('Missing or invalid ServiceRegistry.address in axelar info');
    }

    if (!validateAddress(governanceAddress)) {
        throw new Error('Missing or invalid EventVerifier.governanceAddress in axelar info');
    }

    if (!validateAddress(adminAddress)) {
        throw new Error('Missing or invalid EventVerifier.adminAddress in axelar info');
    }

    if (!isString(serviceName)) {
        throw new Error('Missing or invalid EventVerifier.serviceName in axelar info');
    }

    if (!isStringArray(votingThreshold)) {
        throw new Error('Missing or invalid EventVerifier.votingThreshold in axelar info');
    }

    if (!isNumber(blockExpiry)) {
        throw new Error('Missing or invalid EventVerifier.blockExpiry in axelar info');
    }

    return {
        governance_address: governanceAddress,
        service_registry_address: serviceRegistryAddress,
        service_name: serviceName,
        admin_address: adminAddress,
        voting_threshold: votingThreshold,
        block_expiry: toBigNumberString(blockExpiry),
    };
};

const makeVotingVerifierInstantiateMsg = (config, options, contractConfig) => {
    const { chainName } = options;
    const {
        axelar: { contracts },
        chains: {
            [chainName]: {
                contracts: {
                    [AXELAR_GATEWAY_CONTRACT_NAME]: { address: gatewayAddress },
                },
            },
        },
    } = config;
    const {
        ServiceRegistry: { address: serviceRegistryAddress },
        Rewards: { address: rewardsAddress },
    } = contracts;

    // Get chain codec address
    const chainConfig = config.getChainConfig(chainName);
    const chainCodecAddress = config.getChainCodecAddress(chainConfig.chainType);

    const { serviceName, sourceGatewayAddress, votingThreshold, blockExpiry, confirmationHeight, msgIdFormat, addressFormat } =
        contractConfig;
    const governanceAddress = config.axelar.governanceAddress;

    if (!validateAddress(serviceRegistryAddress)) {
        throw new Error('Missing or invalid ServiceRegistry.address in axelar info');
    }

    if (!validateAddress(rewardsAddress)) {
        throw new Error('Missing or invalid Rewards.address in axelar info');
    }

    if (!validateAddress(chainCodecAddress)) {
        throw new Error(`Missing or invalid ChainCodec address for chain ${chainName} in axelar info`);
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

    if (gatewayAddress !== undefined && gatewayAddress !== sourceGatewayAddress) {
        throw new Error(
            `Address mismatch for [${chainName}] in config:\n` +
                `- [${chainName}].contracts.AxelarGateway.address: ${gatewayAddress}\n` +
                `- axelar.contracts.VotingVerifier[${chainName}].sourceGatewayAddress: ${sourceGatewayAddress}`,
        );
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
        chain_codec_address: chainCodecAddress,
    };
};

const makeChainCodecInstantiateMsg = (_config, _options, contractConfig) => {
    return contractConfig; // we pass on all properties in the codec config
};

const makeXrplGatewayInstantiateMsg = (config, options, contractConfig) => {
    const { chainName } = options;
    const {
        chains: {
            [chainName]: {
                contracts: {
                    AxelarGateway: { address: xrplMultisigAddress },
                },
            },
        },
        axelar: {
            contracts: {
                Router: { address: routerAddress },
                InterchainTokenService: { address: itsHubAddress },
                XrplVotingVerifier: {
                    [chainName]: { address: verifierAddress },
                },
            },
            axelarId: itsHubChainName,
        },
    } = config;
    const adminAddress = contractConfig.adminAddress || config.axelar.multisigProverAdminAddress;
    const governanceAddress = config.axelar.governanceAddress;

    if (!validateAddress(governanceAddress)) {
        throw new Error(`Missing or invalid XrplVotingVerifier[${chainName}].governanceAddress in axelar info`);
    }

    if (!validateAddress(adminAddress)) {
        throw new Error(`Missing or invalid XrplVotingVerifier[${chainName}].adminAddress in axelar info`);
    }

    if (!validateAddress(routerAddress)) {
        throw new Error('Missing or invalid Router.address in axelar info');
    }

    if (!validateAddress(itsHubAddress)) {
        throw new Error('Missing or invalid InterchainTokenService.address in axelar info');
    }

    if (!validateAddress(verifierAddress)) {
        throw new Error(`Missing or invalid XrplVotingVerifier[${chainName}].address in axelar info`);
    }

    if (!isString(xrplMultisigAddress)) {
        throw new Error(`Missing or invalid [${chainName}].contracts.AxelarGateway.address in axelar info`);
    }

    return {
        admin_address: adminAddress,
        governance_address: governanceAddress,
        its_hub_address: itsHubAddress,
        its_hub_chain_name: itsHubChainName,
        router_address: routerAddress,
        verifier_address: verifierAddress,
        chain_name: chainName,
        xrpl_multisig_address: xrplMultisigAddress,
    };
};

const AXELAR_GATEWAY_CONTRACT_NAME = 'AxelarGateway';

const makeGatewayInstantiateMsg = (config, options, _contractConfig) => {
    const { chainName } = options;

    const {
        axelar: {
            contracts: {
                Router: { address: routerAddress },
                [VERIFIER_CONTRACT_NAME]: {
                    [chainName]: { address: verifierAddress },
                },
            },
        },
    } = config;

    if (!validateAddress(routerAddress)) {
        throw new Error('Missing or invalid Router.address in axelar info');
    }

    if (!validateAddress(verifierAddress)) {
        throw new Error(`Missing or invalid ${VERIFIER_CONTRACT_NAME}[${chainName}].address in axelar info`);
    }

    return { router_address: routerAddress, verifier_address: verifierAddress };
};

const makeXrplMultisigProverInstantiateMsg = async (config, options, contractConfig) => {
    const { chainName } = options;
    const {
        axelar: { contracts, chainId: axelarChainId },
        chains: {
            [chainName]: {
                wssRpc,
                contracts: {
                    AxelarGateway: { address: xrplMultisigAddress },
                },
            },
        },
    } = config;
    const {
        Router: { address: routerAddress },
        Coordinator: { address: coordinatorAddress },
        Multisig: { address: multisigAddress },
        ServiceRegistry: { address: serviceRegistryAddress },
        XrplVotingVerifier: {
            [chainName]: { address: verifierAddress },
        },
        XrplGateway: {
            [chainName]: { address: gatewayAddress },
        },
    } = contracts;
    const { signingThreshold, serviceName, verifierSetDiffThreshold, xrplTransactionFee, ticketCountThreshold } = contractConfig;
    const adminAddress = contractConfig.adminAddress || config.axelar.multisigProverAdminAddress;
    const governanceAddress = config.axelar.governanceAddress;

    if (!validateAddress(routerAddress)) {
        throw new Error('Missing or invalid Router.address in axelar info');
    }

    if (!isString(axelarChainId)) {
        throw new Error(`Missing or invalid chain ID`);
    }

    if (!validateAddress(adminAddress)) {
        throw new Error(`Missing or invalid XrplMultisigProver[${chainName}].adminAddress in axelar info`);
    }

    if (!validateAddress(governanceAddress)) {
        throw new Error(`Missing or invalid XrplMultisigProver[${chainName}].governanceAddress in axelar info`);
    }

    if (!validateAddress(gatewayAddress)) {
        throw new Error(`Missing or invalid XrplGateway[${chainName}].address in axelar info`);
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
        throw new Error(`Missing or invalid XrplVotingVerifier[${chainName}].address in axelar info`);
    }

    if (!isStringArray(signingThreshold)) {
        throw new Error(`Missing or invalid XrplMultisigProver[${chainName}].signingThreshold in axelar info`);
    }

    if (!isString(serviceName)) {
        throw new Error(`Missing or invalid XrplMultisigProver[${chainName}].serviceName in axelar info`);
    }

    if (!isNumber(verifierSetDiffThreshold)) {
        throw new Error(`Missing or invalid XrplMultisigProver[${chainName}].verifierSetDiffThreshold in axelar info`);
    }

    if (!isString(xrplMultisigAddress)) {
        throw new Error(`Missing or invalid [${chainName}].contracts.AxelarGateway.address in axelar info`);
    }

    const client = new XRPLClient(wssRpc);
    await client.connect();
    const availableTickets = (await client.tickets(xrplMultisigAddress)).sort();
    const lastAssignedTicketNumber = Math.min(...availableTickets) - 1;
    const accountInfo = await client.accountInfo(xrplMultisigAddress);
    const nextSequenceNumber = accountInfo.sequence + 1; // 1 sequence number reserved for the genesis signer set rotation
    const initialFeeReserve = Number(accountInfo.balance);
    const reserveRequirements = await client.reserveRequirements();
    const baseReserve = reserveRequirements.baseReserve * 1e6;
    const ownerReserve = reserveRequirements.ownerReserve * 1e6;
    await client.disconnect();

    return {
        admin_address: adminAddress,
        governance_address: governanceAddress,
        gateway_address: gatewayAddress,
        coordinator_address: coordinatorAddress,
        multisig_address: multisigAddress,
        service_registry_address: serviceRegistryAddress,
        voting_verifier_address: verifierAddress,
        signing_threshold: signingThreshold,
        service_name: serviceName,
        chain_name: chainName,
        verifier_set_diff_threshold: verifierSetDiffThreshold,
        xrpl_multisig_address: xrplMultisigAddress,
        xrpl_transaction_fee: xrplTransactionFee,
        xrpl_base_reserve: baseReserve,
        xrpl_owner_reserve: ownerReserve,
        initial_fee_reserve: initialFeeReserve,
        ticket_count_threshold: ticketCountThreshold,
        available_tickets: availableTickets,
        next_sequence_number: nextSequenceNumber,
        last_assigned_ticket_number: lastAssignedTicketNumber,
    };
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
        [VERIFIER_CONTRACT_NAME]: {
            [chainName]: { address: verifierAddress },
        },
        [GATEWAY_CONTRACT_NAME]: {
            [chainName]: { address: gatewayAddress },
        },
    } = contracts;

    // Get chain codec address
    const chainConfig = config.getChainConfig(chainName);
    const chainCodecAddress = config.getChainCodecAddress(chainConfig.chainType);

    const { domainSeparator, signingThreshold, serviceName, verifierSetDiffThreshold, encoder, keyType } = contractConfig;
    const adminAddress = contractConfig.adminAddress || config.axelar.multisigProverAdminAddress;
    const governanceAddress = config.axelar.governanceAddress;

    if (!validateAddress(routerAddress)) {
        throw new Error('Missing or invalid Router.address in axelar info');
    }

    if (!validateAddress(chainCodecAddress)) {
        throw new Error(`Missing or invalid ChainCodec address for chain ${chainName} in axelar info`);
    }

    if (!isString(axelarChainId)) {
        throw new Error(`Missing or invalid chain ID`);
    }

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

    if (!isStringArray(signingThreshold)) {
        throw new Error(`Missing or invalid MultisigProver[${chainName}].signingThreshold in axelar info`);
    }

    if (!isString(serviceName)) {
        throw new Error(`Missing or invalid MultisigProver[${chainName}].serviceName in axelar info`);
    }

    if (!isNumber(verifierSetDiffThreshold)) {
        throw new Error(`Missing or invalid MultisigProver[${chainName}].verifierSetDiffThreshold in axelar info`);
    }

    if (!isString(keyType)) {
        throw new Error(`Missing or invalid MultisigProver[${chainName}].keyType in axelar info`);
    }

    contractConfig.domainSeparator = contractConfig.domainSeparator || calculateDomainSeparator(chainName, routerAddress, axelarChainId);

    if (!isKeccak256Hash(contractConfig.domainSeparator)) {
        throw new Error(`Invalid MultisigProver[${chainName}].domainSeparator in axelar info`);
    }

    return {
        admin_address: adminAddress,
        governance_address: governanceAddress,
        gateway_address: gatewayAddress,
        coordinator_address: coordinatorAddress,
        multisig_address: multisigAddress,
        service_registry_address: serviceRegistryAddress,
        voting_verifier_address: verifierAddress,
        chain_codec_address: chainCodecAddress,
        signing_threshold: signingThreshold,
        service_name: serviceName,
        chain_name: chainName,
        verifier_set_diff_threshold: verifierSetDiffThreshold,
        key_type: keyType,
        domain_separator: contractConfig.domainSeparator.replace('0x', ''),
        expect_full_message_payloads: Boolean(contractConfig.expectFullMessagePayloads) || false,
        notify_signing_session: Boolean(contractConfig.notifySigningSession) || false,
        sig_verifier_address: contractConfig.sigVerifierAddress || null,
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
    const { operatorAddress } = contractConfig;
    const adminAddress = contractConfig.adminAddress || config.axelar.adminAddress;
    const governanceAddress = config.axelar.governanceAddress;
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
        operator_address: operatorAddress,
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

const fetchCodeIdFromContract = async (client, contractConfig) => {
    const { address } = contractConfig;

    if (!address) {
        throw new Error('Contract address not found in the config');
    }

    const { codeId } = await client.getContract(address);

    return codeId;
};

const itsHubDecimalsTruncationParams = (config, chainConfig) => {
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

const itsHubChainParams = (config, chainConfig) => {
    const { maxUintBits, maxDecimalsWhenTruncating } = itsHubDecimalsTruncationParams(config, chainConfig);
    const itsEdgeContractAddress = itsEdgeContract(chainConfig);

    const key = chainConfig.axelarId.toLowerCase();
    const chainParams = config.axelar.contracts.InterchainTokenService[key];
    const itsMsgTranslator =
        chainParams?.msgTranslator ||
        config.validateRequired(config.getContractConfig('ItsAbiTranslator').address, 'ItsAbiTranslator.address');

    return {
        itsEdgeContractAddress,
        itsMsgTranslator,
        maxUintBits,
        maxDecimalsWhenTruncating,
    };
};

const getInstantiatePermission = (accessType, addresses) => {
    return {
        permission: accessType,
        addresses,
    };
};

const encodeStoreCode = (options) => {
    const { source, builder, instantiateAddresses } = options;
    const wasm = readContractCode(options);

    const instantiatePermission =
        instantiateAddresses && instantiateAddresses.length > 0
            ? getInstantiatePermission(AccessType.ACCESS_TYPE_ANY_OF_ADDRESSES, instantiateAddresses)
            : getInstantiatePermission(AccessType.ACCESS_TYPE_NOBODY, []);

    const storeMsg = MsgStoreCode.fromPartial({
        sender: GOVERNANCE_MODULE_ADDRESS,
        wasmByteCode: zlib.gzipSync(wasm),
        instantiatePermission,
        source,
        builder,
    });

    return {
        typeUrl: '/cosmwasm.wasm.v1.MsgStoreCode',
        value: Uint8Array.from(MsgStoreCode.encode(storeMsg).finish()),
    };
};

const encodeStoreInstantiate = (options, msg) => {
    const { source, builder, instantiateAddresses, admin } = options;
    const wasm = readContractCode(options);

    const instantiatePermission =
        instantiateAddresses && instantiateAddresses.length > 0
            ? getInstantiatePermission(AccessType.ACCESS_TYPE_ANY_OF_ADDRESSES, instantiateAddresses)
            : getInstantiatePermission(AccessType.ACCESS_TYPE_NOBODY, []);

    const storeAndInstantiateMsg = MsgStoreAndInstantiateContract.fromPartial({
        authority: GOVERNANCE_MODULE_ADDRESS,
        wasmByteCode: zlib.gzipSync(wasm),
        instantiatePermission,
        admin,
        label: getLabel(options),
        msg: Buffer.from(JSON.stringify(msg)),
        funds: [],
        source,
        builder,
    });

    return {
        typeUrl: '/cosmwasm.wasm.v1.MsgStoreAndInstantiateContract',
        value: Uint8Array.from(MsgStoreAndInstantiateContract.encode(storeAndInstantiateMsg).finish()),
    };
};

const encodeInstantiate = (config, options, msg) => {
    const { admin, contractName, salt, chainName, instantiate2 } = options;
    const { contractConfig } = getAmplifierContractConfig(config, options);

    if (instantiate2) {
        const instantiateMsg = MsgInstantiateContract2.fromPartial({
            sender: GOVERNANCE_MODULE_ADDRESS,
            admin,
            codeId: contractConfig.codeId,
            label: getLabel(options),
            msg: Buffer.from(JSON.stringify(msg)),
            funds: [],
            salt: getSalt(salt, contractName, chainName),
            fixMsg: false,
        });
        return {
            typeUrl: '/cosmwasm.wasm.v1.MsgInstantiateContract2',
            value: Uint8Array.from(MsgInstantiateContract2.encode(instantiateMsg).finish()),
        };
    } else {
        const instantiateMsg = MsgInstantiateContract.fromPartial({
            sender: GOVERNANCE_MODULE_ADDRESS,
            admin,
            codeId: contractConfig.codeId,
            label: getLabel(options),
            msg: Buffer.from(JSON.stringify(msg)),
            funds: [],
        });
        return {
            typeUrl: '/cosmwasm.wasm.v1.MsgInstantiateContract',
            value: Uint8Array.from(MsgInstantiateContract.encode(instantiateMsg).finish()),
        };
    }
};

const encodeExecuteContract = (config, options, chainName) => {
    const { contractName, msg } = options;
    const {
        axelar: {
            contracts: { [contractName]: contractConfig },
        },
    } = config;
    const chainConfig = getChainConfig(config.chains, chainName);

    const executeMsg = MsgExecuteContract.fromPartial({
        sender: GOVERNANCE_MODULE_ADDRESS,
        contract: contractConfig[chainConfig?.axelarId]?.address || contractConfig.address,
        msg: Buffer.from(msg),
        funds: [],
    });

    return {
        typeUrl: '/cosmwasm.wasm.v1.MsgExecuteContract',
        value: Uint8Array.from(MsgExecuteContract.encode(executeMsg).finish()),
    };
};

const encodeUpdateInstantiateConfigProposal = (options) => {
    const accessConfigUpdates = JSON.parse(options.msg);

    if (!Array.isArray(accessConfigUpdates) || accessConfigUpdates.length !== 1) {
        throw new Error('msg must contain exactly one access config update');
    }

    const { codeId, instantiatePermission } = accessConfigUpdates[0];

    const msg = MsgUpdateInstantiateConfig.fromPartial({
        sender: GOVERNANCE_MODULE_ADDRESS,
        codeId: BigInt(codeId),
        newInstantiatePermission: {
            permission: instantiatePermission.permission,
            addresses: instantiatePermission.addresses,
        },
    });

    return {
        typeUrl: '/cosmwasm.wasm.v1.MsgUpdateInstantiateConfig',
        value: Uint8Array.from(MsgUpdateInstantiateConfig.encode(msg).finish()),
    };
};

const encodeMigrate = (config, options) => {
    const { msg, chainName } = options;

    let contractConfig;
    let chainConfig;
    if (!options.address || !options.codeId) {
        contractConfig = getAmplifierContractConfig(config, options).contractConfig;
        chainConfig = getChainConfig(config.chains, chainName);
    }

    const migrateMsg = MsgMigrateContract.fromPartial({
        sender: GOVERNANCE_MODULE_ADDRESS,
        contract: options.address ?? (contractConfig[chainConfig?.axelarId]?.address || contractConfig.address),
        codeId: options.codeId ?? contractConfig.codeId,
        msg: Buffer.from(msg),
    });

    return {
        typeUrl: '/cosmwasm.wasm.v1.MsgMigrateContract',
        value: Uint8Array.from(MsgMigrateContract.encode(migrateMsg).finish()),
    };
};

const loadProtoDefinition = (protoName) => {
    const fullPath = path.join(__dirname, 'proto', protoName);
    try {
        return fs.readFileSync(fullPath, 'utf8');
    } catch (error) {
        throw new Error(`Failed to load proto: ${fullPath}. ${error.message}`);
    }
};

const encodeCallContracts = (proposalData) => {
    const { title, description, contract_calls: contractCallsInput } = proposalData;

    if (!title || !description || !Array.isArray(contractCallsInput)) {
        throw new Error('Invalid proposal data: must have title, description, and contract_calls array');
    }

    const protoDefinition = loadProtoDefinition('axelarnet_call_contracts.proto');

    const parsed = protobuf.parse(protoDefinition, { keepCase: true });
    const root = parsed.root;

    const CallContractsProposal = root.lookupType('axelar.axelarnet.v1beta1.CallContractsProposal');
    const ContractCall = root.lookupType('axelar.axelarnet.v1beta1.ContractCall');

    if (!CallContractsProposal || !ContractCall) {
        throw new Error('Failed to lookup proto types');
    }

    const contractCalls = contractCallsInput.map((call, index) => {
        const { chain, contract_address: contractAddress, payload } = call || {};

        if (!chain || !contractAddress || !payload) {
            throw new Error(`Invalid contract_call at index ${index}: must have chain, contract_address, and payload`);
        }

        const payloadBytes = Buffer.from(payload, 'base64');

        const contractCall = ContractCall.create({
            chain,
            contract_address: contractAddress,
            payload: payloadBytes,
        });

        const errMsg = ContractCall.verify(contractCall);
        if (errMsg) {
            throw new Error(`Invalid ContractCall at index ${index}: ${errMsg}`);
        }

        return contractCall;
    });

    const proposal = CallContractsProposal.create({
        title,
        description,
        contract_calls: contractCalls,
    });

    const errMsg = CallContractsProposal.verify(proposal);
    if (errMsg) {
        throw new Error(`Invalid CallContractsProposal: ${errMsg}`);
    }

    const message = CallContractsProposal.encode(proposal).finish();

    return {
        typeUrl: '/axelar.axelarnet.v1beta1.CallContractsProposal',
        value: Uint8Array.from(message),
    };
};

const encodeSubmitProposal = (messages, config, options, proposer) => {
    const {
        axelar: { tokenSymbol },
    } = config;
    const { deposit, title, description, standardProposal } = options;

    const initialDeposit = [{ denom: `u${tokenSymbol.toLowerCase()}`, amount: deposit }];

    const proposalData = {
        messages,
        initialDeposit,
        proposer,
        metadata: '',
        title,
        summary: description,
        expedited: !standardProposal,
    };

    return {
        typeUrl: '/cosmos.gov.v1.MsgSubmitProposal',
        value: MsgSubmitProposalV1.fromPartial(proposalData),
    };
};

// Retries sign-and-broadcast on transient RPC socket closures
const signAndBroadcastWithRetry = async (client, signerAddress, msgs, fee, memo = '', maxAttempts = 3) => {
    let lastError;
    for (let attempt = 0; attempt < maxAttempts; attempt++) {
        try {
            return await client.signAndBroadcast(signerAddress, msgs, fee, memo);
        } catch (error) {
            lastError = error;
            const code = error?.cause?.code || error?.code;
            const message = error?.message || '';

            // Confirm err is socket error
            const isTransient = code === 'UND_ERR_SOCKET' || /fetch failed/i.test(message);
            if (!isTransient || attempt === maxAttempts - 1) {
                throw error;
            }

            printInfo('Retrying proposal submission..... 🔄');
        }
    }
};

const getNexusProtoType = (typeName) => {
    const protoDefinition = loadProtoDefinition('nexus_chain.proto');

    const parsed = protobuf.parse(protoDefinition, { keepCase: true });
    const root = parsed.root;

    const fullTypeName = `axelar.nexus.v1beta1.${typeName}`;
    const ProtoType = root.lookupType(fullTypeName);

    if (!ProtoType) {
        throw new Error(`Failed to lookup ${typeName} proto type`);
    }

    return ProtoType;
};

const encodeChainStatusRequest = (chains, requestType) => {
    if (!Array.isArray(chains) || chains.length === 0 || !chains.every((chain) => typeof chain === 'string' && chain.trim() !== '')) {
        throw new Error('chains must be a non-empty array of non-empty strings');
    }

    const RequestType = getNexusProtoType(requestType);

    const request = RequestType.create({
        sender: GOVERNANCE_MODULE_ADDRESS,
        chains: chains,
    });

    const errMsg = RequestType.verify(request);
    if (errMsg) {
        throw new Error(`Invalid ${requestType}: ${errMsg}`);
    }

    const message = RequestType.encode(request).finish();

    return {
        typeUrl: `/axelar.nexus.v1beta1.${requestType}`,
        value: Uint8Array.from(message),
    };
};

const submitProposal = async (client, config, options, proposal, fee) => {
    const deposit =
        options.deposit ?? (options.standardProposal ? config.proposalDepositAmount() : config.proposalExpeditedDepositAmount());
    const proposalOptions = { ...options, deposit };

    const [account] = await client.signer.getAccounts();

    printInfo('Proposer address', account.address);

    const messages = toArray(proposal);

    const submitProposalMsg = encodeSubmitProposal(messages, config, proposalOptions, account.address);

    const result = await signAndBroadcastWithRetry(client, account.address, [submitProposalMsg], fee, '');
    const { events } = result;

    const proposalEvent = events.find(({ type }) => type === 'proposal_submitted' || type === 'submit_proposal');
    if (!proposalEvent) {
        throw new Error('Proposal submission event not found');
    }

    const proposalId = proposalEvent.attributes.find(({ key }) => key === 'proposal_id')?.value;
    if (!proposalId) {
        throw new Error('Proposal ID not found in events');
    }

    return proposalId;
};

const submitCallContracts = async (client, config, options, proposalData, fee) => {
    if (!proposalData.title || !proposalData.description || !proposalData.contract_calls) {
        throw new Error('Invalid proposal data: must have title, description, and contract_calls');
    }

    const content = encodeCallContracts(proposalData);

    const { deposit, title, description } = options;

    const initialDeposit = [{ denom: getUnitDenom(config), amount: deposit }];

    const accounts = client.accounts || (await client.signer.getAccounts());
    const [account] = accounts;

    if (!account || !account.address) {
        throw new Error('Failed to determine proposer account from client');
    }

    // Always submit CallContractsProposal via legacy MsgSubmitProposal (v1beta1) regardless of SDK version
    const submitProposalMsg = {
        typeUrl: '/cosmos.gov.v1beta1.MsgSubmitProposal',
        value: MsgSubmitProposal.fromPartial({
            content,
            initialDeposit,
            proposer: account.address,
        }),
    };

    printInfo('Proposer address', account.address);
    printInfo('Proposal title', title);
    printInfo('Proposal description', description);

    const result = await signAndBroadcastWithRetry(client, account.address, [submitProposalMsg], fee, '');
    const { events } = result;

    const proposalEvent = events.find(({ type }) => type === 'proposal_submitted' || type === 'submit_proposal');
    if (!proposalEvent) {
        throw new Error('Proposal submission event not found');
    }

    const proposalId = proposalEvent.attributes.find(({ key }) => key === 'proposal_id')?.value;
    if (!proposalId) {
        throw new Error('Proposal ID not found in events');
    }

    return proposalId;
};

const getContractR2Url = (contractName, contractVersion) => {
    const pathName = getCrateName(contractName);
    const fileName = getFileName(contractName);

    if (VERSION_REGEX.test(contractVersion)) {
        return `${AXELAR_R2_BASE_URL}/releases/cosmwasm/${pathName}/${contractVersion}/${fileName}`;
    }

    if (SHORT_COMMIT_HASH_REGEX.test(contractVersion)) {
        return `${AXELAR_R2_BASE_URL}/pre-releases/cosmwasm/${contractVersion}/${fileName}`;
    }

    throw new Error(`Invalid contractVersion format: ${contractVersion}. Must be a semantic version (including prefix v) or a commit hash`);
};

const getContractArtifactPath = (artifactDir, contractName) => {
    const basePath = artifactDir.endsWith('/') ? artifactDir : artifactDir + '/';
    const fileName = getFileName(contractName);

    return basePath + fileName;
};

const getCrateName = (contractName) => {
    return pascalToKebab(contractName);
};

const getFileName = (contractName) => {
    return `${pascalToSnake(contractName)}.wasm`;
};

const getContractCodePath = async (options, contractName) => {
    if (options.artifactDir) {
        return getContractArtifactPath(options.artifactDir, contractName);
    }

    if (options.version) {
        const url = getContractR2Url(contractName, options.version);
        return downloadContractCode(url, contractName, options.version);
    }

    throw new Error('Either --artifact-dir or --version must be provided');
};

const makeItsAbiTranslatorInstantiateMsg = (_config, _options, _contractConfig) => {
    return {};
};

const validateItsChainChange = async (client, config, chainName, proposedConfig) => {
    const chainConfig = getChainConfig(config.chains, chainName);

    const itsEdgeContract = tryItsEdgeContract(chainConfig);
    if (!itsEdgeContract) {
        throw new Error(`ITS edge contract not found for chain '${chainName}'.`);
    }

    const currentConfig = await client.queryContractSmart(config.axelar.contracts.InterchainTokenService.address, {
        its_chain: {
            chain: chainConfig.axelarId,
        },
    });

    const hasChanges =
        currentConfig.chain !== proposedConfig.chain ||
        currentConfig.its_edge_contract !== proposedConfig.its_edge_contract ||
        currentConfig.msg_translator !== proposedConfig.msg_translator ||
        currentConfig.truncation.max_uint_bits !== proposedConfig.truncation.max_uint_bits ||
        currentConfig.truncation.max_decimals_when_truncating !== proposedConfig.truncation.max_decimals_when_truncating;

    if (!hasChanges) {
        throw new Error(`No changes detected for chain '${chainName}'.`);
    }
};

const getCodeDetails = async (config, codeId) => {
    const tendermintClient = await Tendermint34Client.connect(config?.axelar?.rpc);
    let codeInfo;

    try {
        const data = QueryCodeRequest.encode({
            codeId: BigInt(codeId),
        }).finish();

        const { value } = await tendermintClient.abciQuery({
            path: '/cosmwasm.wasm.v1.Query/Code',
            data: data,
        });

        codeInfo = QueryCodeResponse.decode(value)?.codeInfo;
        if (!codeInfo) {
            throw new Error(`Info not found for code id ${codeId}`);
        }
    } finally {
        tendermintClient.disconnect();
    }

    return codeInfo;
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
    EventVerifier: {
        scope: CONTRACT_SCOPE_GLOBAL,
        makeInstantiateMsg: makeEventVerifierInstantiateMsg,
    },
    VotingVerifier: {
        scope: CONTRACT_SCOPE_CHAIN,
        makeInstantiateMsg: makeVotingVerifierInstantiateMsg,
    },
    ChainCodecEvm: {
        scope: CONTRACT_SCOPE_GLOBAL,
        makeInstantiateMsg: makeChainCodecInstantiateMsg,
    },
    ChainCodecSui: {
        scope: CONTRACT_SCOPE_GLOBAL,
        makeInstantiateMsg: makeChainCodecInstantiateMsg,
    },
    ChainCodecStellar: {
        scope: CONTRACT_SCOPE_GLOBAL,
        makeInstantiateMsg: makeChainCodecInstantiateMsg,
    },
    ChainCodecSolana: {
        scope: CONTRACT_SCOPE_GLOBAL,
        makeInstantiateMsg: makeChainCodecInstantiateMsg,
    },
    XrplVotingVerifier: {
        scope: CONTRACT_SCOPE_CHAIN,
        makeInstantiateMsg: makeXrplVotingVerifierInstantiateMsg,
    },
    Gateway: {
        scope: CONTRACT_SCOPE_CHAIN,
        makeInstantiateMsg: makeGatewayInstantiateMsg,
    },
    XrplGateway: {
        scope: CONTRACT_SCOPE_CHAIN,
        makeInstantiateMsg: makeXrplGatewayInstantiateMsg,
    },
    MultisigProver: {
        scope: CONTRACT_SCOPE_CHAIN,
        makeInstantiateMsg: makeMultisigProverInstantiateMsg,
    },
    XrplMultisigProver: {
        scope: CONTRACT_SCOPE_CHAIN,
        makeInstantiateMsg: makeXrplMultisigProverInstantiateMsg,
    },
    AxelarnetGateway: {
        scope: CONTRACT_SCOPE_GLOBAL,
        makeInstantiateMsg: makeAxelarnetGatewayInstantiateMsg,
    },
    InterchainTokenService: {
        scope: CONTRACT_SCOPE_GLOBAL,
        makeInstantiateMsg: makeInterchainTokenServiceInstantiateMsg,
    },
    ItsAbiTranslator: {
        scope: CONTRACT_SCOPE_GLOBAL,
        makeInstantiateMsg: makeItsAbiTranslatorInstantiateMsg,
    },
};

module.exports = {
    CONTRACT_SCOPE_CHAIN,
    CONTRACT_SCOPE_GLOBAL,
    CONTRACTS,
    AXELAR_GATEWAY_CONTRACT_NAME,
    fromHex,
    toArray,
    getSalt,
    calculateDomainSeparator,
    getAmplifierContractConfig,
    getCodeId,
    getCodeDetails,
    executeTransaction,
    uploadContract,
    instantiateContract,
    migrateContract,
    fetchCodeIdFromCodeHash,
    fetchCodeIdFromContract,
    itsHubDecimalsTruncationParams,
    itsHubChainParams,
    encodeStoreCode,
    encodeStoreInstantiate,
    encodeInstantiate,
    encodeExecuteContract,
    encodeUpdateInstantiateConfigProposal,
    encodeMigrate,
    encodeCallContracts,
    encodeSubmitProposal,
    encodeChainStatusRequest,
    submitProposal,
    submitCallContracts,
    signAndBroadcastWithRetry,
    loadProtoDefinition,
    getNexusProtoType,
    isValidCosmosAddress,
    getContractCodePath,
    validateItsChainChange,
    validateGovernanceMode,
    getUnitDenom,
    GOVERNANCE_MODULE_ADDRESS,
};
