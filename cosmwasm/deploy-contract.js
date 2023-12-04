'use strict';

require('dotenv').config();

const { SigningCosmWasmClient } = require('@cosmjs/cosmwasm-stargate');
const { DirectSecp256k1HdWallet } = require('@cosmjs/proto-signing');

const { printInfo, loadConfig, saveConfig, isString, isStringArray, isNumber, prompt } = require('../evm/utils');
const { uploadContract, instantiateContract, isValidCosmosAddress } = require('./utils');

const { Command, Option } = require('commander');

const validateAddress = (address) => {
    return isString(address) && isValidCosmosAddress(address);
};

const makeServiceRegistryInstantiateMsg = ({ governanceAccount }) => {
    if (!validateAddress(governanceAccount)) {
        throw new Error('Missing or invalid ServiceRegistry.governanceAccount in axelar info');
    }

    return { governance_account: governanceAccount };
};

const makeMultisigInstantiateMsg = ({ governanceAddress, gracePeriod }, { Rewards: { address: rewardsAddress } }) => {
    if (!validateAddress(governanceAddress)) {
        throw new Error('Missing or invalid Multisig.governanceAddress in axelar info');
    }

    if (!validateAddress(rewardsAddress)) {
        throw new Error('Missing or invalid Rewards.address in axelar info');
    }

    if (!isNumber(gracePeriod)) {
        throw new Error(`Missing or invalid Multisig.gracePeriod in axelar info`);
    }

    return { governance_address: governanceAddress, rewards_address: rewardsAddress, grace_period: gracePeriod };
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

const makeConnectionRouterInstantiateMsg = ({ adminAddress, governanceAddress }, { NexusGateway: { address: nexusGateway } }) => {
    if (!validateAddress(adminAddress)) {
        throw new Error('Missing or invalid ConnectionRouter.adminAddress in axelar info');
    }

    if (!validateAddress(governanceAddress)) {
        throw new Error('Missing or invalid ConnectionRouter.governanceAddress in axelar info');
    }

    if (!validateAddress(nexusGateway)) {
        throw new Error('Missing or invalid NexusGateway.address in axelar info');
    }

    return { admin_address: adminAddress, governance_address: governanceAddress, nexus_gateway: nexusGateway };
};

const makeNexusGatewayInstantiateMsg = ({ nexus }, { ConnectionRouter: { address: router } }) => {
    if (!validateAddress(nexus)) {
        throw new Error('Missing or invalid NexusGateway.nexus in axelar info');
    }

    if (!validateAddress(router)) {
        throw new Error('Missing or invalid ConnectionRouter.address in axelar info');
    }

    return { nexus, router };
};

const makeVotingVerifierInstantiateMsg = (
    contractConfig,
    { ServiceRegistry: { address: serviceRegistryAddress }, Rewards: { address: rewardsAddress } },
    { id: chainId },
) => {
    const {
        [chainId]: { serviceName, sourceGatewayAddress, votingThreshold, blockExpiry, confirmationHeight },
    } = contractConfig;

    if (!validateAddress(serviceRegistryAddress)) {
        throw new Error('Missing or invalid ServiceRegistry.address in axelar info');
    }

    if (!validateAddress(rewardsAddress)) {
        throw new Error('Missing or invalid Rewards.address in axelar info');
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

const makeGatewayInstantiateMsg = ({ ConnectionRouter: { address: connectionRouterAddress }, VotingVerifier }, { id: chainId }) => {
    const {
        [chainId]: { address: verifierAddress },
    } = VotingVerifier;

    if (!validateAddress(connectionRouterAddress)) {
        throw new Error('Missing or invalid ConnectionRouter.address in axelar info');
    }

    if (!validateAddress(verifierAddress)) {
        throw new Error(`Missing or invalid VotingVerifier[${chainId}].address in axelar info`);
    }

    return { router_address: connectionRouterAddress, verifier_address: verifierAddress };
};

const makeMultisigProverInstantiateMsg = (contractConfig, contracts, { id: chainId }) => {
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

    if (!validateAddress(adminAddress)) {
        throw new Error(`Missing or invalid MultisigProver[${chainId}].adminAddress in axelar info`);
    }

    if (!validateAddress(gatewayAddress)) {
        throw new Error(`Missing or invalid Gateway[${chainId}].address in axelar info`);
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

    if (!isString(destinationChainID)) {
        throw new Error(`Missing or invalid MultisigProver[${chainId}].destinationChainID in axelar info`);
    }

    if (!isStringArray(signingThreshold)) {
        throw new Error(`Missing or invalid MultisigProver[${chainId}].signingThreshold in axelar info`);
    }

    if (!isString(serviceName)) {
        throw new Error(`Missing or invalid MultisigProver[${chainId}].serviceName in axelar info`);
    }

    if (!isNumber(workerSetDiffThreshold)) {
        throw new Error(`Missing or invalid MultisigProver[${chainId}].workerSetDiffThreshold in axelar info`);
    }

    if (!isString(encoder)) {
        throw new Error(`Missing or invalid MultisigProver[${chainId}].encoder in axelar info`);
    }

    if (!isString(keyType)) {
        throw new Error(`Missing or invalid MultisigProver[${chainId}].keyType in axelar info`);
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

const makeInstantiateMsg = (contractName, chain, config) => {
    const {
        axelar: { contracts },
    } = config;

    const { [contractName]: contractConfig } = contracts;

    const { codeId } = contractConfig;

    if (!isNumber(codeId)) {
        throw new Error('Code Id is not defined');
    }

    switch (contractName) {
        case 'ServiceRegistry': {
            if (chain) {
                throw new Error('ServiceRegistry does not support chainNames option');
            }

            return makeServiceRegistryInstantiateMsg(contractConfig);
        }

        case 'Multisig': {
            if (chain) {
                throw new Error('Multisig does not support chainNames option');
            }

            return makeMultisigInstantiateMsg(contractConfig, contracts);
        }

        case 'Rewards': {
            if (chain) {
                throw new Error('Rewards does not support chainNames option');
            }

            return makeRewardsInstantiateMsg(contractConfig);
        }

        case 'ConnectionRouter': {
            if (chain) {
                throw new Error('ConnectionRouter does not support chainNames option');
            }

            return makeConnectionRouterInstantiateMsg(contractConfig, contracts);
        }

        case 'NexusGateway': {
            if (chain) {
                throw new Error('NexusGateway does not support chainNames option');
            }

            return makeNexusGatewayInstantiateMsg(contractConfig, contracts);
        }

        case 'VotingVerifier': {
            if (!chain) {
                throw new Error('VotingVerifier requires chainNames option');
            }

            return makeVotingVerifierInstantiateMsg(contractConfig, contracts, chain);
        }

        case 'Gateway': {
            if (!chain) {
                throw new Error('Gateway requires chainNames option');
            }

            return makeGatewayInstantiateMsg(contracts, chain);
        }

        case 'MultisigProver': {
            if (!chain) {
                throw new Error('MultisigProver requires chainNames option');
            }

            return makeMultisigProverInstantiateMsg(contractConfig, contracts, chain);
        }
    }

    throw new Error(`${contractName} is not supported.`);
};

const deploy = async (options, chain, config) => {
    printInfo('Deploying for chain', chain ? chain.name : 'none');

    const wallet = await DirectSecp256k1HdWallet.fromMnemonic(options.mnemonic, { prefix: 'axelar' });
    const client = await SigningCosmWasmClient.connectWithSigner(config.axelar.rpc, wallet);

    const {
        axelar: {
            contracts: { [options.contractName]: contractConfig = {} },
        },
    } = config;
    console.log(options);

    printInfo('Contract name', options.contractName);

    const reuseCodeId = !!options.reuseCodeId && !!contractConfig.codeId;
    printInfo('Reusing codeId', reuseCodeId.toString());

    if (!reuseCodeId) {
        const result = await uploadContract(client, wallet, config, options);
        contractConfig.codeId = result.codeId;

        if (result.address) {
            contractConfig.address = result.address;
            printInfo('Expected contract address', contractConfig.address);
        }
    }

    printInfo('Code Id', contractConfig.codeId);

    if (options.uploadOnly || prompt(`Proceed with deployment on axelar?`, options.yes)) {
        return;
    }

    const initMsg = makeInstantiateMsg(options.contractName, chain, config);
    const contractAddress = await instantiateContract(client, wallet, initMsg, config, options);

    if (chain) {
        contractConfig[chain.id] = {
            ...contractConfig[chain.id],
            address: contractAddress,
        };
    } else {
        contractConfig.address = contractAddress;
    }

    printInfo('Contract address', contractAddress);
};

const main = async (options) => {
    const config = loadConfig(options.env);

    let chains = options.chainNames.split(',').map((str) => str.trim());

    if (options.chainNames === 'all') {
        chains = Object.keys(config.chains);
    }

    const undefinedChain = chains.find((chain) => !config.chains[chain.toLowerCase()] && chain !== 'none');

    if (undefinedChain) {
        throw new Error(`Chain ${undefinedChain} is not defined in the info file`);
    }

    for (const chain of chains) {
        await deploy(options, config.chains[chain.toLowerCase()], config);
        saveConfig(config, options.env);

        options.reuseCodeId = true;
    }
};

const programHandler = () => {
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
    program.addOption(new Option('-s, --salt <salt>', 'salt for instantiate2. defaults to contract name').env('SALT'));
    program.addOption(
        new Option(
            '-u, --uploadOnly',
            'upload the contract without instantiating. prints expected contract address if --instantiate2 is passed',
        ),
    );
    program.addOption(new Option('--instantiate2', 'use instantiate2 for constant address deployment'));
    program.addOption(new Option('--aarch64', 'aarch64').env('AARCH64').default(false));
    program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));

    program.action((options) => {
        main(options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
