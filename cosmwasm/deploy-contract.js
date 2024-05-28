'use strict';

require('dotenv').config();
const { isNil } = require('lodash');

const { SigningCosmWasmClient } = require('@cosmjs/cosmwasm-stargate');
const { DirectSecp256k1HdWallet } = require('@cosmjs/proto-signing');

const { printInfo, loadConfig, saveConfig, isString, isStringArray, isKeccak256Hash, isNumber, prompt } = require('../evm/utils');
const { uploadContract, instantiateContract, isValidCosmosAddress, governanceAddress } = require('./utils');

const { Command, Option } = require('commander');

const { ethers } = require('hardhat');
const {
    utils: { arrayify },
} = ethers;

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

const makeMultisigInstantiateMsg = ({ governanceAddress, blockExpiry }, { Rewards: { address: rewardsAddress } }) => {
    if (!validateAddress(governanceAddress)) {
        throw new Error('Missing or invalid Multisig.governanceAddress in axelar info');
    }

    if (!validateAddress(rewardsAddress)) {
        throw new Error('Missing or invalid Rewards.address in axelar info');
    }

    if (!isNumber(blockExpiry)) {
        throw new Error(`Missing or invalid Multisig.blockExpiry in axelar info`);
    }

    return { governance_address: governanceAddress, rewards_address: rewardsAddress, block_expiry: blockExpiry };
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
        [chainId]: { governanceAddress, serviceName, sourceGatewayAddress, votingThreshold, blockExpiry, confirmationHeight, msgIdFormat },
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

    return {
        service_registry_address: serviceRegistryAddress,
        rewards_address: rewardsAddress,
        governance_address: governanceAddress,
        service_name: serviceName,
        source_gateway_address: sourceGatewayAddress,
        voting_threshold: votingThreshold,
        block_expiry: blockExpiry,
        confirmation_height: confirmationHeight,
        source_chain: chainId,
        msg_id_format: msgIdFormat,
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
        axelar: { contracts, chainId: network, axelarId },
        chains: { [chainName]: chainConfig },
    } = config;

    const {axelarId: chainId } = chainConfig;

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
            workerSetDiffThreshold,
            encoder,
            keyType,
        },
    } = contractConfig;

    const separator = domainSeparator || calculateDomainSeparator(axelarId, routerAddress, network);

    if (!validateAddress(adminAddress)) {
        throw new Error(`Missing or invalid MultisigProver[${chainId}].adminAddress in axelar info`);
    }

    if (!validateAddress(governanceAddress)) {
        throw new Error(`Missing or invalid MultisigProver[${chainId}].governanceAddress in axelar info`);
    }

    if (!validateAddress(gatewayAddress)) {
        throw new Error(`Missing or invalid Gateway[${chainId}].address in axelar info`);
    }

    if (!validateAddress(routerAddress)) {
        throw new Error('Missing or invalid Router.address in axelar info');
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
        governance_address: governanceAddress,
        gateway_address: gatewayAddress,
        coordinator_address: coordinatorAddress,
        multisig_address: multisigAddress,
        service_registry_address: serviceRegistryAddress,
        voting_verifier_address: verifierAddress,
        domain_separator: arrayify(separator),
        signing_threshold: signingThreshold,
        service_name: serviceName,
        chain_name: chainId,
        worker_set_diff_threshold: workerSetDiffThreshold,
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

    const { codeId } = contractConfig;

    if (!isNumber(codeId)) {
        throw new Error('Code Id is not defined');
    }

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

const prepareWallet = ({ mnemonic }) => DirectSecp256k1HdWallet.fromMnemonic(mnemonic, { prefix: 'axelar' });

const prepareClient = ({ axelar: { rpc, gasPrice } }, wallet) =>
    SigningCosmWasmClient.connectWithSigner(rpc, wallet, { gasPrice }).then((client) => {
        return { wallet, client };
    });

const upload = (client, wallet, chainName, config, options) => {
    const { reuseCodeId, contractName } = options;
    const {
        axelar: {
            contracts: { [contractName]: contractConfig },
        },
        chains: { [chainName]: chainConfig },
    } = config;

    if (!reuseCodeId || isNil(contractConfig.codeId)) {
        printInfo('Uploading contract binary');

        return uploadContract(client, wallet, config, options)
            .then(({ address, codeId }) => {
                printInfo('Uploaded contract binary');
                contractConfig.codeId = codeId;

                if (!address) {
                    return;
                }

                if (chainConfig) {
                    contractConfig[chainConfig.axelarId] = {
                        ...contractConfig[chainConfig.axelarId],
                        address,
                    };
                } else {
                    contractConfig.address = address;
                }

                printInfo('Expected contract address', address);
            })
            .then(() => ({ wallet, client }));
    }

    printInfo('Skipping upload. Reusing previously uploaded binary');
    return Promise.resolve({ wallet, client });
};

const instantiate = (client, wallet, chainName, config, options) => {
    const { contractName } = options;
    const {
        axelar: {
            contracts: { [contractName]: contractConfig },
        },
        chains: { [chainName]: chainConfig },
    } = config;

    const initMsg = makeInstantiateMsg(contractName, chainName, config);
    return instantiateContract(client, wallet, initMsg, config, options).then((contractAddress) => {
        if (chainConfig) {
            contractConfig[chainConfig.axelarId] = {
                ...contractConfig[chainConfig.axelarId],
                address: contractAddress,
            };
        } else {
            contractConfig.address = contractAddress;
        }

        printInfo(`Instantiated ${chainName === 'none' ? '' : chainName.concat(' ')}${contractName}. Address`, contractAddress);
    });
};

const main = async (options) => {
    const { env, chainNames, uploadOnly, yes, instantiate2 } = options;
    const config = loadConfig(env);

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

    await prepareWallet(options)
        .then((wallet) => prepareClient(config, wallet))
        .then(({ wallet, client }) => upload(client, wallet, chains[0], config, options))
        .then(({ wallet, client }) => {
            if (uploadOnly || prompt(`Proceed with deployment on axelar?`, yes)) {
                return;
            }

            return chains.reduce((promise, chain) => {
                return promise.then(() => instantiate(client, wallet, chain.toLowerCase(), config, options));
            }, Promise.resolve());
        })
        .then(() => saveConfig(config, env));
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
        new Option('--admin <address>', 'when instantiating contract, set an admin address. Defaults to governance module account').default(
            governanceAddress,
        ),
    );
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
