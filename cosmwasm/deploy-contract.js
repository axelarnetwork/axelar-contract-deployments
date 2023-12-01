'use strict';

require('dotenv').config();

const { SigningCosmWasmClient } = require('@cosmjs/cosmwasm-stargate');
const { DirectSecp256k1HdWallet } = require('@cosmjs/proto-signing');

const { printInfo, loadConfig, saveConfig, isString, isStringArray, isNumber } = require('../evm/utils');
const { uploadContract, instantiateContract } = require('./utils');

const { Command, Option } = require('commander');

const getServiceRegistryInstantiateMsg = ({ governanceAccount }) => {
    if (!isString(governanceAccount)) {
        throw new Error('Missing ServiceRegistry.governanceAccount in axelar info');
    }

    return { governance_account: governanceAccount };
};

const getMultisigInstantiateMsg = ({ governanceAddress, rewardsAddress, gracePeriod }) => {
    if (!isString(governanceAddress)) {
        throw new Error('Missing Multisig.governanceAddress in axelar info');
    }

    if (!isString(rewardsAddress)) {
        throw new Error('Missing Rewards.address in axelar info');
    }

    if (!isNumber(gracePeriod)) {
        throw new Error(`Missing Multisig.gracePeriod in axelar info`);
    }

    return { governance_address: governanceAddress, rewards_address: rewardsAddress, grace_period: gracePeriod };
};

const getRewardsInstantiateMsg = ({ governanceAddress, rewardsDenom, params }) => {
    if (!isString(governanceAddress)) {
        throw new Error('Missing Rewards.governanceAddress in axelar info');
    }

    if (!isString(rewardsDenom)) {
        throw new Error('Missing Rewards.rewardsDenom in axelar info');
    }

    return { governance_address: governanceAddress, rewards_denom: rewardsDenom, params };
};

const getConnectionRouterInstantiateMsg = ({ adminAddress, governanceAddress }, { NexusGateway: { address: nexusGateway } }) => {
    if (!isString(adminAddress)) {
        throw new Error('Missing ConnectionRouter.adminAddress in axelar info');
    }

    if (!isString(governanceAddress)) {
        throw new Error('Missing ConnectionRouter.governanceAddress in axelar info');
    }

    if (!isString(nexusGateway)) {
        throw new Error('Missing NexusGateway.address in axelar info');
    }

    return { admin_address: adminAddress, governance_address: governanceAddress, nexus_gateway: nexusGateway };
};

const getNexusGatewayInstantiateMsg = ({ nexus }, { ConnectionRouter: { address: router } }) => {
    if (!isString(nexus)) {
        throw new Error('Missing NexusGateway.nexus in axelar info');
    }

    if (!isString(router)) {
        throw new Error('Missing ConnectionRouter.address in axelar info');
    }

    return { nexus, router };
};

const getVotingVerifierInstantiateMsg = (
    contractConfig,
    { ServiceRegistry: { address: serviceRegistryAddress }, Rewards: { address: rewardsAddress } },
    chainId,
) => {
    const {
        [chainId]: { serviceName, sourceGatewayAddress, votingThreshold, blockExpiry, confirmationHeight },
    } = contractConfig;

    if (!isString(serviceRegistryAddress)) {
        throw new Error('Missing ServiceRegistry.address in axelar info');
    }

    if (!isString(rewardsAddress)) {
        throw new Error('Missing Rewards.address in axelar info');
    }

    if (!isString(serviceName)) {
        throw new Error(`Missing VotingVerifier[${chainId}].serviceName in axelar info`);
    }

    if (!isString(sourceGatewayAddress)) {
        throw new Error(`Missing VotingVerifier[${chainId}].sourceGatewayAddress in axelar info`);
    }

    if (!isStringArray(votingThreshold)) {
        throw new Error(`Missing VotingVerifier[${chainId}].votingThreshold in axelar info`);
    }

    if (!isNumber(blockExpiry)) {
        throw new Error(`Missing VotingVerifier[${chainId}].blockExpiry in axelar info`);
    }

    if (!isNumber(confirmationHeight)) {
        throw new Error(`Missing VotingVerifier[${chainId}].confirmationHeight in axelar info`);
    }

    return {
        service_registry_address: serviceRegistryAddress,
        rewards_address: rewardsAddress,
        service_name: serviceName,
        source_gateway_address: sourceGatewayAddress,
        voting_threshold: votingThreshold,
        block_expiry: blockExpiry,
        confirmation_height: confirmationHeight,
        source_chain: chainId,
    };
};

const getGatewayInstantiateMsg = ({ ConnectionRouter: { address: connectionRouterAddress }, VotingVerifier }, chainId) => {
    const {
        [chainId]: { address: verifierAddress },
    } = VotingVerifier;

    if (!isString(connectionRouterAddress)) {
        throw new Error('Missing ConnectionRouter.address in axelar info');
    }

    if (!isString(verifierAddress)) {
        throw new Error(`Missing VotingVerifier[${chainId}].address in axelar info`);
    }

    return { router_address: connectionRouterAddress, verifier_address: verifierAddress };
};

const getMultisigProverInstantiateMsg = (contractConfig, contracts, chainId) => {
    const {
        Multisig: { address: multisigAddress },
        ServiceRegistry: { address: serviceRegistryAddress },
        VotingVerifier: {
            [chainId]: { address: verifierAddress },
        },
        Gateway: {
            [chainId]: { address: gatewayAddress },
        },
    } = contracts;
    const {
        [chainId]: { adminAddress, destinationChainID, signingThreshold, serviceName, workerSetDiffThreshold, encoder, keyType },
    } = contractConfig;

    if (!isString(adminAddress)) {
        throw new Error(`Missing MultisigProver[${chainId}].adminAddress in axelar info`);
    }

    if (!isString(gatewayAddress)) {
        throw new Error(`Missing Gateway[${chainId}].address in axelar info`);
    }

    if (!isString(multisigAddress)) {
        throw new Error('Missing Multisig.address in axelar info');
    }

    if (!isString(serviceRegistryAddress)) {
        throw new Error('Missing ServiceRegistry.address in axelar info');
    }

    if (!isString(verifierAddress)) {
        throw new Error(`Missing VotingVerifier[${chainId}].address in axelar info`);
    }

    if (!isString(destinationChainID)) {
        throw new Error(`Missing MultisigProver[${chainId}].destinationChainID in axelar info`);
    }

    if (!isStringArray(signingThreshold)) {
        throw new Error(`Missing MultisigProver[${chainId}].signingThreshold in axelar info`);
    }

    if (!isString(serviceName)) {
        throw new Error(`Missing MultisigProver[${chainId}].serviceName in axelar info`);
    }

    if (!isNumber(workerSetDiffThreshold)) {
        throw new Error(`Missing MultisigProver[${chainId}].workerSetDiffThreshold in axelar info`);
    }

    if (!isString(encoder)) {
        throw new Error(`Missing MultisigProver[${chainId}].encoder in axelar info`);
    }

    if (!isString(keyType)) {
        throw new Error(`Missing MultisigProver[${chainId}].keyType in axelar info`);
    }

    return {
        admin_address: adminAddress,
        gateway_address: gatewayAddress,
        multisig_address: multisigAddress,
        service_registry_address: serviceRegistryAddress,
        voting_verifier_address: verifierAddress,
        destination_chain_id: destinationChainID,
        signing_threshold: signingThreshold,
        service_name: serviceName,
        chain_name: chainId,
        worker_set_diff_threshold: workerSetDiffThreshold,
        encoder,
        key_type: keyType,
    };
};

async function getInstantiateMsg(contractName, config, chain) {
    const {
        axelar: { contracts },
    } = config;

    const { [contractName]: contractConfig } = contracts;

    const { codeId } = contractConfig;
    const { id: chainId } = chain;

    if (!isNumber(codeId)) {
        throw new Error('Code Id is not defined');
    }

    switch (contractName) {
        case 'ServiceRegistry': {
            if (chain) {
                throw new Error('ServiceRegistry does not support chainNames option');
            }

            return getServiceRegistryInstantiateMsg(contractConfig);
        }

        case 'Multisig': {
            if (chain) {
                throw new Error('Multisig does not support chainNames option');
            }

            return getMultisigInstantiateMsg(contractConfig);
        }

        case 'Rewards': {
            if (chain) {
                throw new Error('Rewards does not support chainNames option');
            }

            return getRewardsInstantiateMsg(contractConfig);
        }

        case 'ConnectionRouter': {
            if (chain) {
                throw new Error('ConnectionRouter does not support chainNames option');
            }

            return getConnectionRouterInstantiateMsg(contractConfig, contracts);
        }

        case 'NexusGateway': {
            if (chain) {
                throw new Error('NexusGateway does not support chainNames option');
            }

            return getNexusGatewayInstantiateMsg(contractConfig, contracts);
        }

        case 'VotingVerifier': {
            if (!chain) {
                throw new Error('VotingVerifier requires chainNames option');
            }

            return getVotingVerifierInstantiateMsg(contractConfig, contracts, chain);
        }

        case 'Gateway': {
            if (!chain) {
                throw new Error('Gateway requires chainNames option');
            }

            return getGatewayInstantiateMsg(contracts, chainId);
        }

        case 'MultisigProver': {
            if (!chain) {
                throw new Error('MultisigProver requires chainNames option');
            }

            return getMultisigProverInstantiateMsg(contractConfig, contracts, chainId);
        }
    }

    throw new Error(`${contractName} is not supported.`);
}

async function deploy(options, chain, config) {
    printInfo('Deploying for chain', chain ? chain.name : 'none');

    const wallet = await DirectSecp256k1HdWallet.fromMnemonic(options.mnemonic, { prefix: 'axelar' });
    const client = await SigningCosmWasmClient.connectWithSigner(config.axelar.rpc, wallet);

    if (config.axelar.contracts[options.contractName] === undefined) {
        config.axelar.contracts[options.contractName] = {};
    }

    const contractConfig = config.axelar.contracts[options.contractName];

    printInfo('Contract name', options.contractName);

    const reuseCodeId = !!options.reuseCodeId && !!contractConfig.codeId;
    printInfo('Reusing codeId', reuseCodeId.toString());

    if (!reuseCodeId) {
        const result = await uploadContract(config, options, wallet, client);
        contractConfig.codeId = result.codeId;

        if (result.address) {
            contractConfig.address = result.address;
            printInfo('Expected contract address', contractConfig.address);
        }
    }

    printInfo('Code Id', contractConfig.codeId);

    if (!options.uploadOnly) {
        const initMsg = await getInstantiateMsg(options.contractName, config, chain);
        const contractAddress = await instantiateContract(config, options, options.contractName, initMsg, wallet, client);

        if (chain) {
            contractConfig[chain.id] = {
                ...contractConfig[chain.id],
                address: contractAddress,
            };
        } else {
            contractConfig.address = contractAddress;
        }

        printInfo('Contract address', contractAddress);
    }
}

async function main(options) {
    const config = loadConfig(options.env);

    let chains = options.chainNames.split(',').map((str) => str.trim());

    if (options.chainNames === 'all') {
        chains = Object.keys(config.chains);
    }

    for (const chain of chains) {
        if (config.chains[chain.toLowerCase()] === undefined && chain !== 'none') {
            throw new Error(`Chain ${chain} is not defined in the info file`);
        }
    }

    for (const chain of chains) {
        await deploy(options, config.chains[chain.toLowerCase()], config);
        saveConfig(config, options.env);

        options.reuseCodeId = true;
    }
}

async function programHandler() {
    const program = new Command();

    program.name('upload-contract').description('Upload CosmWasm contracts');

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(new Option('-m, --mnemonic <mnemonic>', 'mnemonic').makeOptionMandatory(true).env('MNEMONIC'));
    program.addOption(new Option('-a, --artifactPath <artifactPath>', 'artifact path').makeOptionMandatory(true).env('ARTIFACT_PATH'));
    program.addOption(new Option('-c, --contractName <contractName>', 'contract name').makeOptionMandatory(true));
    program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').default('none'));
    program.addOption(new Option('-r, --reuseCodeId', 'reuse code Id'));
    program.addOption(new Option('-s, --salt', 'salt for instantiate2. defaults to contract name'));
    program.addOption(
        new Option(
            '-u, --uploadOnly',
            'upload the contract without instantiating. prints expected contract address if --instantiate2 is passed',
        ),
    );
    program.addOption(new Option('--instantiate2', 'use instantiate2 for constant address deployment'));
    program.addOption(new Option('--aarch64', 'aarch64').env('AARCH64').default(false));

    program.action((options) => {
        main(options);
    });

    program.parse();
}

if (require.main === module) {
    programHandler();
}
