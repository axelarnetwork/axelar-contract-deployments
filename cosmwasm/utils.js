'use strict';

const zlib = require('zlib');
const { ethers } = require('hardhat');
const {
    utils: { keccak256 },
} = ethers;
const { createHash } = require('crypto');

const { readFileSync } = require('fs');
const { calculateFee, GasPrice } = require('@cosmjs/stargate');
const { instantiate2Address, SigningCosmWasmClient } = require('@cosmjs/cosmwasm-stargate');
const { DirectSecp256k1HdWallet } = require('@cosmjs/proto-signing');
const { MsgSubmitProposal } = require('cosmjs-types/cosmos/gov/v1beta1/tx');
const {
    StoreCodeProposal,
    StoreAndInstantiateContractProposal,
    InstantiateContractProposal,
    InstantiateContract2Proposal,
    ExecuteContractProposal,
} = require('cosmjs-types/cosmwasm/wasm/v1/proposal');
const { AccessType } = require('cosmjs-types/cosmwasm/wasm/v1/types');
const { getSaltFromKey, isString, isStringArray, isKeccak256Hash, isNumber, toBigNumberString } = require('../evm/utils');
const { normalizeBech32 } = require('@cosmjs/encoding');

const governanceAddress = 'axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj';

const prepareWallet = ({ mnemonic }) => DirectSecp256k1HdWallet.fromMnemonic(mnemonic, { prefix: 'axelar' });

const prepareClient = ({ axelar: { rpc, gasPrice } }, wallet) =>
    SigningCosmWasmClient.connectWithSigner(rpc, wallet, { gasPrice }).then((client) => {
        return { wallet, client };
    });

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

const calculateDomainSeparator = (chain, router, network) => keccak256(Buffer.from(`${chain}${router}${network}`));

const getSalt = (salt, contractName, chainNames) => fromHex(getSaltFromKey(salt || contractName.concat(chainNames)));

const readWasmFile = ({ artifactPath, contractName, aarch64 }) =>
    readFileSync(`${artifactPath}/${pascalToSnake(contractName)}${aarch64 ? '-aarch64' : ''}.wasm`);

const getChains = (config, { chainNames, instantiate2 }) => {
    let chains = chainNames.split(',').map((str) => str.trim());

    if (chainNames === 'all') {
        chains = Object.keys(config.chains);
    }

    if (chains.length !== 1 && instantiate2) {
        throw new Error('Cannot pass --instantiate2 with more than one chain');
    }

    const undefinedChain = chains.find((chain) => !config.chains[chain.toLowerCase()] && chain !== 'none');

    if (undefinedChain) {
        throw new Error(`Chain ${undefinedChain} is not defined in the info file`);
    }

    return chains;
};

const uploadContract = async (client, wallet, config, options) => {
    const { contractName, instantiate2, salt, chainNames } = options;
    return wallet
        .getAccounts()
        .then(([account]) => {
            const wasm = readWasmFile(options);
            const {
                axelar: { gasPrice, gasLimit },
            } = config;
            const uploadFee = gasLimit === 'auto' ? 'auto' : calculateFee(gasLimit, GasPrice.fromString(gasPrice));
            return client.upload(account.address, wasm, uploadFee).then(({ checksum, codeId }) => ({ checksum, codeId, account }));
        })
        .then(({ account, checksum, codeId }) => {
            const address = instantiate2
                ? instantiate2Address(fromHex(checksum), account.address, getSalt(salt, contractName, chainNames), 'axelar')
                : null;

            return { codeId, address };
        });
};

const instantiateContract = (client, wallet, initMsg, config, { contractName, salt, instantiate2, chainNames, admin }) => {
    return wallet
        .getAccounts()
        .then(([account]) => {
            const contractConfig = config.axelar.contracts[contractName];

            const {
                axelar: { gasPrice, gasLimit },
            } = config;
            const initFee = gasLimit === 'auto' ? 'auto' : calculateFee(gasLimit, GasPrice.fromString(gasPrice));

            return instantiate2
                ? client.instantiate2(
                      account.address,
                      contractConfig.codeId,
                      getSalt(salt, contractName, chainNames),
                      initMsg,
                      contractName,
                      initFee,
                      { admin },
                  )
                : client.instantiate(account.address, contractConfig.codeId, initMsg, contractName, initFee, {
                      admin,
                  });
        })
        .then(({ contractAddress }) => contractAddress);
};

const validateAddress = (address) => {
    return isString(address) && isValidCosmosAddress(address);
};

const makeCoordinatorInstantiateMsg = ({ governanceAddress }) => {
    if (!validateAddress(governanceAddress)) {
        throw new Error('Missing or invalid Coordinator.governanceAddress in axelar info');
    }

    return { governance_address: governanceAddress };
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

const makeRewardsInstantiateMsg = ({ governanceAddress, rewardsDenom, params }) => {
    if (!validateAddress(governanceAddress)) {
        throw new Error('Missing or invalid Rewards.governanceAddress in axelar info');
    }

    if (!isString(rewardsDenom)) {
        throw new Error('Missing or invalid Rewards.rewardsDenom in axelar info');
    }

    return { governance_address: governanceAddress, rewards_denom: rewardsDenom, params };
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

const makeNexusGatewayInstantiateMsg = ({ nexus }, { Router: { address: router } }) => {
    if (!validateAddress(nexus)) {
        throw new Error('Missing or invalid NexusGateway.nexus in axelar info');
    }

    if (!validateAddress(router)) {
        throw new Error('Missing or invalid Router.address in axelar info');
    }

    return { nexus, router };
};

const makeVotingVerifierInstantiateMsg = (
    contractConfig,
    { ServiceRegistry: { address: serviceRegistryAddress }, Rewards: { address: rewardsAddress } },
    { id: chainId },
) => {
    const {
        [chainId]: {
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

    if (!validateAddress(serviceRegistryAddress)) {
        throw new Error('Missing or invalid ServiceRegistry.address in axelar info');
    }

    if (!validateAddress(rewardsAddress)) {
        throw new Error('Missing or invalid Rewards.address in axelar info');
    }

    if (!validateAddress(governanceAddress)) {
        throw new Error(`Missing or invalid VotingVerifier[${chainId}].governanceAddress in axelar info`);
    }

    if (!isString(serviceName)) {
        throw new Error(`Missing or invalid VotingVerifier[${chainId}].serviceName in axelar info`);
    }

    if (!isString(sourceGatewayAddress)) {
        throw new Error(`Missing or invalid VotingVerifier[${chainId}].sourceGatewayAddress in axelar info`);
    }

    if (!isStringArray(votingThreshold)) {
        throw new Error(`Missing or invalid VotingVerifier[${chainId}].votingThreshold in axelar info`);
    }

    if (!isNumber(blockExpiry)) {
        throw new Error(`Missing or invalid VotingVerifier[${chainId}].blockExpiry in axelar info`);
    }

    if (!isNumber(confirmationHeight)) {
        throw new Error(`Missing or invalid VotingVerifier[${chainId}].confirmationHeight in axelar info`);
    }

    if (!isString(msgIdFormat)) {
        throw new Error(`Missing or invalid VotingVerifier[${chainId}].msgIdFormat in axelar info`);
    }

    if (!isString(addressFormat)) {
        throw new Error(`Missing or invalid VotingVerifier[${chainId}].addressFormat in axelar info`);
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
        source_chain: chainId,
        msg_id_format: msgIdFormat,
        address_format: addressFormat,
    };
};

const makeGatewayInstantiateMsg = ({ Router: { address: routerAddress }, VotingVerifier }, { id: chainId }) => {
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
        chains: { [chainName]: chainConfig },
    } = config;

    const { axelarId: chainId } = chainConfig;

    const {
        Router: { address: routerAddress },
        Coordinator: { address: coordinatorAddress },
        Multisig: { address: multisigAddress },
        ServiceRegistry: { address: serviceRegistryAddress },
        VotingVerifier: {
            [chainId]: { address: verifierAddress },
        },
        Gateway: {
            [chainId]: { address: gatewayAddress },
        },
        MultisigProver: contractConfig,
    } = contracts;
    const {
        [chainId]: {
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

    if (!isString(chainId)) {
        throw new Error(`Missing or invalid axelar ID for chain ${chainName}`);
    }

    if (!validateAddress(routerAddress)) {
        throw new Error('Missing or invalid Router.address in axelar info');
    }

    if (!isString(axelarChainId)) {
        throw new Error(`Missing or invalid chain ID`);
    }

    const separator = domainSeparator || calculateDomainSeparator(chainId, routerAddress, axelarChainId);
    contractConfig[chainId].domainSeparator = separator;

    if (!validateAddress(adminAddress)) {
        throw new Error(`Missing or invalid MultisigProver[${chainId}].adminAddress in axelar info`);
    }

    if (!validateAddress(governanceAddress)) {
        throw new Error(`Missing or invalid MultisigProver[${chainId}].governanceAddress in axelar info`);
    }

    if (!validateAddress(gatewayAddress)) {
        throw new Error(`Missing or invalid Gateway[${chainId}].address in axelar info`);
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
        throw new Error(`Missing or invalid VotingVerifier[${chainId}].address in axelar info`);
    }

    if (!isKeccak256Hash(separator)) {
        throw new Error(`Invalid MultisigProver[${chainId}].domainSeparator in axelar info`);
    }

    if (!isStringArray(signingThreshold)) {
        throw new Error(`Missing or invalid MultisigProver[${chainId}].signingThreshold in axelar info`);
    }

    if (!isString(serviceName)) {
        throw new Error(`Missing or invalid MultisigProver[${chainId}].serviceName in axelar info`);
    }

    if (!isNumber(verifierSetDiffThreshold)) {
        throw new Error(`Missing or invalid MultisigProver[${chainId}].verifierSetDiffThreshold in axelar info`);
    }

    if (!isString(encoder)) {
        throw new Error(`Missing or invalid MultisigProver[${chainId}].encoder in axelar info`);
    }

    if (!isString(keyType)) {
        throw new Error(`Missing or invalid MultisigProver[${chainId}].keyType in axelar info`);
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
        chain_name: chainId,
        verifier_set_diff_threshold: verifierSetDiffThreshold,
        encoder,
        key_type: keyType,
    };
};

const makeInstantiateMsg = (contractName, chainName, config) => {
    const {
        axelar: { contracts },
        chains: { [chainName]: chainConfig },
    } = config;

    const { [contractName]: contractConfig } = contracts;

    switch (contractName) {
        case 'Coordinator': {
            if (chainConfig) {
                throw new Error('Coordinator does not support chainNames option');
            }

            return makeCoordinatorInstantiateMsg(contractConfig);
        }

        case 'ServiceRegistry': {
            if (chainConfig) {
                throw new Error('ServiceRegistry does not support chainNames option');
            }

            return makeServiceRegistryInstantiateMsg(contractConfig);
        }

        case 'Multisig': {
            if (chainConfig) {
                throw new Error('Multisig does not support chainNames option');
            }

            return makeMultisigInstantiateMsg(contractConfig, contracts);
        }

        case 'Rewards': {
            if (chainConfig) {
                throw new Error('Rewards does not support chainNames option');
            }

            return makeRewardsInstantiateMsg(contractConfig);
        }

        case 'Router': {
            if (chainConfig) {
                throw new Error('Router does not support chainNames option');
            }

            return makeRouterInstantiateMsg(contractConfig, contracts);
        }

        case 'NexusGateway': {
            if (chainConfig) {
                throw new Error('NexusGateway does not support chainNames option');
            }

            return makeNexusGatewayInstantiateMsg(contractConfig, contracts);
        }

        case 'VotingVerifier': {
            if (!chainConfig) {
                throw new Error('VotingVerifier requires chainNames option');
            }

            return makeVotingVerifierInstantiateMsg(contractConfig, contracts, chainConfig);
        }

        case 'Gateway': {
            if (!chainConfig) {
                throw new Error('Gateway requires chainNames option');
            }

            return makeGatewayInstantiateMsg(contracts, chainConfig);
        }

        case 'MultisigProver': {
            if (!chainConfig) {
                throw new Error('MultisigProver requires chainNames option');
            }

            return makeMultisigProverInstantiateMsg(config, chainName);
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

const instantiate2AddressForProposal = (client, contractConfig, { contractName, salt, chainNames, runAs }) => {
    return client
        .getCodeDetails(contractConfig.codeId)
        .then(({ checksum }) => instantiate2Address(fromHex(checksum), runAs, getSalt(salt, contractName, chainNames), 'axelar'));
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
    const { contractName, admin } = options;

    return {
        ...getStoreCodeParams(options),
        admin,
        label: contractName,
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
        label: contractName,
        msg: Buffer.from(JSON.stringify(msg)),
    };
};

const getInstantiateContract2Params = (config, options, msg) => {
    const { contractName, salt, chainNames } = options;

    return {
        ...getInstantiateContractParams(config, options, msg),
        salt: getSalt(salt, contractName, chainNames),
    };
};

const getExecuteContractParams = (config, options, chainName) => {
    const { contractName, msg } = options;
    const {
        axelar: {
            contracts: { [contractName]: contractConfig },
        },
        chains: { [chainName]: chainConfig },
    } = config;

    return {
        ...getSubmitProposalParams(options),
        contract: chainConfig ? contractConfig[chainConfig.axelarId].address : contractConfig.address,
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

const submitProposal = (client, wallet, config, options, content) => {
    return wallet
        .getAccounts()
        .then(([account]) => {
            const {
                axelar: { gasPrice, gasLimit },
            } = config;

            const submitProposalMsg = encodeSubmitProposal(content, config, options, account.address);

            const fee = gasLimit === 'auto' ? 'auto' : calculateFee(gasLimit, GasPrice.fromString(gasPrice));
            return client.signAndBroadcast(account.address, [submitProposalMsg], fee, '');
        })
        .then(
            ({ events }) => events.find(({ type }) => type === 'submit_proposal').attributes.find(({ key }) => key === 'proposal_id').value,
        );
};

module.exports = {
    governanceAddress,
    prepareWallet,
    prepareClient,
    calculateDomainSeparator,
    readWasmFile,
    getChains,
    uploadContract,
    instantiateContract,
    makeInstantiateMsg,
    fetchCodeIdFromCodeHash,
    instantiate2AddressForProposal,
    decodeProposalAttributes,
    encodeStoreCodeProposal,
    encodeStoreInstantiateProposal,
    encodeInstantiateProposal,
    encodeInstantiate2Proposal,
    encodeExecuteContractProposal,
    submitProposal,
    isValidCosmosAddress,
};
