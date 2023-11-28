'use strict';

require('dotenv').config();

const { SigningCosmWasmClient } = require('@cosmjs/cosmwasm-stargate');
const { DirectSecp256k1HdWallet } = require('@cosmjs/proto-signing');

const { printInfo, loadConfig, saveConfig, isString, isStringArray, isNumber } = require('../evm/utils');
const { uploadContract, instantiateContract } = require('./utils');

const { Command, Option } = require('commander');

async function getInstantiateMsg(contractName, config, chain) {
    let contractConfig = config.axelar.contracts[contractName];

    if (!isNumber(contractConfig.codeID)) {
        throw new Error('Code ID is not defined');
    }

    switch (contractName) {
        case 'ServiceRegistry': {
            if (chain) {
                throw new Error('ServiceRegistry does not support chainNames option');
            }

            const governanceAccount = contractConfig.governanceAccount;

            if (!isString(governanceAccount)) {
                throw new Error('Missing ServiceRegistry.governanceAccount in axelar info');
            }

            return { governance_account: governanceAccount };
        }

        case 'Multisig': {
            if (chain) {
                throw new Error('Multisig does not support chainNames option');
            }
            const governanceAddress = contractConfig.governanceAddress;

            if (!isString(governanceAddress)) {
                throw new Error('Missing Multisig.governanceAddress in axelar info');
            }

            const rewardsAddress = config.axelar.contracts.Rewards.address;

            if (!isString(rewardsAddress)) {
                throw new Error('Missing Rewards.address in axelar info');
            }

            const gracePeriod = contractConfig.gracePeriod;

            if (!isNumber(gracePeriod)) {
                throw new Error(`Missing Multisig.gracePeriod in axelar info`);
            }

            return {governance_address: governanceAddress, rewards_address: rewardsAddress, grace_period: gracePeriod};
        }

        case 'Rewards': {
        
            if (chain) {
                throw new Error('Rewards does not support chainNames option');
            }
            const governanceAddress = contractConfig.governanceAddress;

            if (!isString(governanceAddress)) {
                throw new Error('Missing Rewards.governanceAddress in axelar info');
            }

            const rewardsDenom = contractConfig.rewardsDenom;

            if (!isString(rewardsDenom)) {
                throw new Error('Missing Rewards.rewardsDenom in axelar info');
            }

            const params = contractConfig.params;

            return {governance_address: governanceAddress, rewards_denom: rewardsDenom, params: params} ;

        }

        case 'ConnectionRouter': {
            if (chain) {
                throw new Error('ConnectionRouter does not support chainNames option');
            }

            const adminAddress = contractConfig.adminAddress;

            if (!isString(adminAddress)) {
                throw new Error('Missing ConnectionRouter.adminAddress in axelar info');
            }
            const governanceAddress = contractConfig.governanceAddress;

            if (!isString(governanceAddress)) {
                throw new Error('Missing ConnectionRouter.governanceAddress in axelar info');
            }

            return { admin_address: adminAddress, governance_address: governanceAddress };
        }

        case 'NexusGateway': {
            if (chain) {
                throw new Error('ConnectionRouter does not support chainNames option');
            }

            const nexus = contractConfig.nexus;

            if (!isString(nexus)) {
                throw new Error('Missing NexusGateway.nexus in axelar info');
            }

            const router = config.axelar.contracts.ConnectionRouter.address;

            if (!isString(router)) {
                throw new Error('Missing NexusGateway.router in axelar info');
            }

            return { nexus, router};

        }

        case 'VotingVerifier': {
            if (!chain) {
                throw new Error('VotingVerifier requires chainNames option');
            }

            contractConfig = contractConfig[chain.id];

            const serviceRegistryAddress = config.axelar.contracts.ServiceRegistry.address;

            if (!isString(serviceRegistryAddress)) {
                throw new Error('Missing ServiceRegistry.address in axelar info');
            }

            const rewardsAddress = config.axelar.contracts.ServiceRegistry.address;

            if (!isString(rewardsAddress)) {
                throw new Error('Missing Rewards.address in axelar info');
            }

            const serviceName = contractConfig.serviceName;

            if (!isString(serviceName)) {
                throw new Error(`Missing VotingVerifier[${chain.id}].serviceName in axelar info`);
            }

            const sourceGatewayAddress = contractConfig.sourceGatewayAddress;

            if (!isString(sourceGatewayAddress)) {
                throw new Error(`Missing VotingVerifier[${chain.id}].sourceGatewayAddress in axelar info`);
            }

            const votingThreshold = contractConfig.votingThreshold;

            if (!isStringArray(votingThreshold)) {
                throw new Error(`Missing VotingVerifier[${chain.id}].votingThreshold in axelar info`);
            }

            const blockExpiry = contractConfig.blockExpiry;

            if (!isNumber(blockExpiry)) {
                throw new Error(`Missing VotingVerifier[${chain.id}].blockExpiry in axelar info`);
            }

            const confirmationHeight = contractConfig.confirmationHeight;

            if (!isNumber(confirmationHeight)) {
                throw new Error(`Missing VotingVerifier[${chain.id}].confirmationHeight in axelar info`);
            }

            return {
                service_registry_address: serviceRegistryAddress,
                rewards_address: rewardsAddress,
                service_name: serviceName,
                source_gateway_address: sourceGatewayAddress,
                voting_threshold: votingThreshold,
                block_expiry: blockExpiry,
                confirmation_height: confirmationHeight,
                source_chain: chain.name,
            };
        }

        case 'Gateway': {
            if (!chain) {
                throw new Error('Gateway requires chainNames option');
            }

            contractConfig = contractConfig[chain.id];

            const connectionRouterAddress = config.axelar.contracts.ConnectionRouter.address;

            if (!isString(connectionRouterAddress)) {
                throw new Error('Missing ConnectionRouter.address in axelar info');
            }

            const verifierAddress = config.axelar.contracts.VotingVerifier[chain.id].address;

            if (!isString(verifierAddress)) {
                throw new Error(`Missing VotingVerifier[${chain.id}].address in axelar info`);
            }

            return { router_address: connectionRouterAddress, verifier_address: verifierAddress };
        }

        case 'MultisigProver': {
            if (!chain) {
                throw new Error('MultisigProver requires chainNames option');
            }

            contractConfig = contractConfig[chain.id];

            const adminAddress = contractConfig.adminAddress;

            if (!isString(adminAddress)) {
                throw new Error(`Missing MultisigProver[${chain.id}].adminAddress in axelar info`);
            }

            const gatewayAddress = config.axelar.contracts.Gateway[chain.id].address;

            if (!isString(gatewayAddress)) {
                throw new Error(`Missing Gateway[${chain.id}].address in axelar info`);
            }

            const multisigAddress = config.axelar.contracts.Multisig.address;

            if (!isString(multisigAddress)) {
                throw new Error('Missing Multisig.address in axelar info');
            }

            const serviceRegistryAddress = config.axelar.contracts.ServiceRegistry.address;

            if (!isString(serviceRegistryAddress)) {
                throw new Error('Missing ServiceRegistry.address in axelar info');
            }

            const verifierAddress = config.axelar.contracts.VotingVerifier[chain.id].address;

            if (!isString(verifierAddress)) {
                throw new Error(`Missing VotingVerifier[${chain.id}].address in axelar info`);
            }

            const destinationChainID = contractConfig.destinationChainID;

            if (!isString(destinationChainID)) {
                throw new Error(`Missing MultisigProver[${chain.id}].destinationChainID in axelar info`);
            }

            const signingThreshold = contractConfig.signingThreshold;

            if (!isStringArray(signingThreshold)) {
                throw new Error(`Missing MultisigProver[${chain.id}].signingThreshold in axelar info`);
            }

            const serviceName = contractConfig.serviceName;

            if (!isString(serviceName)) {
                throw new Error(`Missing MultisigProver[${chain.id}].serviceName in axelar info`);
            }

            const workerSetDiffThreshold = contractConfig.workerSetDiffThreshold;

            if (!isNumber(workerSetDiffThreshold)) {
                throw new Error(`Missing MultisigProver[${chain.id}].workerSetDiffThreshold in axelar info`);
            }

            const encoder = contractConfig.encoder;

            if (!isString(encoder)) {
                throw new Error(`Missing MultisigProver[${chain.id}].encoder in axelar info`);
            }

            const keyType = contractConfig.keyType;

            if (!isString(keyType)) {
                throw new Error(`Missing MultisigProver[${chain.id}].keyType in axelar info`);
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
                chain_name: chain.name,
                worker_set_diff_threshold: workerSetDiffThreshold,
                encoder,
                key_type:keyType,
            };
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

    const reuseCodeID = !!options.reuseCodeID && !!contractConfig.codeID;
    printInfo('Reusing codeID', reuseCodeID.toString());

    if (!reuseCodeID) {
        const codeID = await uploadContract(config, options, wallet, client);
        contractConfig.codeID = codeID;
    }

    printInfo('Code ID', contractConfig.codeID);

    const initMsg = await getInstantiateMsg(options.contractName, config, chain);
    const contractAddress = await instantiateContract(config, options.contractName, initMsg, wallet, client);

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

        options.reuseCodeID = true;
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
    program.addOption(new Option('-r, --reuseCodeID', 'reuse code ID'));
    program.addOption(new Option('--aarch64', 'aarch64').env('AARCH64').default(false));

    program.action((options) => {
        main(options);
    });

    program.parse();
}

if (require.main === module) {
    programHandler();
}
