'use strict';

require('../common/cli-utils');

const { instantiate2Address } = require('@cosmjs/cosmwasm-stargate');

const { printInfo, loadConfig, saveConfig, prompt } = require('../common');

const {
    CONTRACTS,
    prepareWallet,
    prepareClient,
    fromHex,
    getSalt,
    initContractConfig,
    getAmplifierContractConfig,
    getCodeId,
    uploadContract,
    instantiateContract,
    migrateContract,
} = require('./utils');

const { Command } = require('commander');
const { addAmplifierOptions } = require('./cli-utils');

const upload = async (client, wallet, config, options) => {
    const { contractName, instantiate2, salt, chainName } = options;
    const { contractBaseConfig, contractConfig } = getAmplifierContractConfig(config, options);

    printInfo('Uploading contract binary');
    const { checksum, codeId } = await uploadContract(client, wallet, config, options);

    printInfo('Uploaded contract binary with codeId', codeId);
    contractBaseConfig.lastUploadedCodeId = codeId;

    if (instantiate2) {
        const [account] = await wallet.getAccounts();
        const address = instantiate2Address(fromHex(checksum), account.address, getSalt(salt, contractName, chainName), 'axelar');

        contractConfig.address = address;

        printInfo('Expected contract address', address);
    }
};

const instantiate = async (client, wallet, config, options) => {
    const { contractName, chainName, yes } = options;

    const { contractConfig } = getAmplifierContractConfig(config, options);

    const codeId = await getCodeId(client, config, options);
    printInfo('Using code id', codeId);

    if (prompt(`Proceed with instantiation on axelar?`, yes)) {
        return;
    }

    contractConfig.codeId = codeId;

    const initMsg = await CONTRACTS[contractName].makeInstantiateMsg(config, options, contractConfig);
    const contractAddress = await instantiateContract(client, wallet, initMsg, config, options);

    contractConfig.address = contractAddress;

    printInfo(`Instantiated ${chainName ? chainName.concat(' ') : ''}${contractName}. Address`, contractAddress);
};

const uploadInstantiate = async (client, wallet, config, options) => {
    await upload(client, wallet, config, options);
    await instantiate(client, wallet, config, options);
};

const migrate = async (client, wallet, config, options) => {
    const { yes } = options;
    const { contractConfig } = getAmplifierContractConfig(config, options);

    const codeId = await getCodeId(client, config, options);
    printInfo('Using code id', codeId);

    if (prompt(`Proceed with contract migration on axelar?`, yes)) {
        return;
    }

    contractConfig.codeId = codeId;

    const { transactionHash } = await migrateContract(client, wallet, config, options);
    printInfo('Migration completed. Transaction hash', transactionHash);
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
        [chainId]: { governanceAddress, serviceName, sourceGatewayAddress, votingThreshold, blockExpiry, confirmationHeight },
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

const makeMultisigProverInstantiateMsg = (contractConfig, contracts, { id: chainId }) => {
    const {
        Coordinator: { address: coordinatorAddress },
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
        [chainId]: {
            adminAddress,
            governanceAddress,
            destinationChainID,
            signingThreshold,
            serviceName,
            workerSetDiffThreshold,
            encoder,
            keyType,
        },
    } = contractConfig;

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
        governance_address: governanceAddress,
        gateway_address: gatewayAddress,
        coordinator_address: coordinatorAddress,
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

            return makeMultisigProverInstantiateMsg(contractConfig, contracts, chainConfig);
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
            .then(({ address, codeId, usedSalt }) => {
                printInfo('Uploaded contract binary');
                contractConfig.codeId = codeId;
                contractConfig.salt = usedSalt;

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

    initContractConfig(config, options);

    const wallet = await prepareWallet(options);
    const client = await prepareClient(config, wallet);

    await processor(client, wallet, config, options);

    saveConfig(config, env);
};

const programHandler = () => {
    const program = new Command();

    program.name('deploy-contract').description('Deploy CosmWasm contracts');

    const uploadCmd = program
        .command('upload')
        .description('Upload wasm binary')
        .action((options) => {
            mainProcessor(upload, options);
        });
    addAmplifierOptions(uploadCmd, {
        contractOptions: true,
        storeOptions: true,
        instantiate2Options: true,
    });

    const instantiateCmd = program
        .command('instantiate')
        .description('Instantiate contract')
        .action((options) => {
            mainProcessor(instantiate, options);
        });
    addAmplifierOptions(instantiateCmd, {
        contractOptions: true,
        instantiateOptions: true,
        instantiate2Options: true,
        codeId: true,
        fetchCodeId: true,
    });

    const uploadInstantiateCmd = program
        .command('upload-instantiate')
        .description('Upload wasm binary and instantiate contract')
        .action((options) => {
            mainProcessor(uploadInstantiate, options);
        });
    addAmplifierOptions(uploadInstantiateCmd, {
        contractOptions: true,
        storeOptions: true,
        instantiateOptions: true,
        instantiate2Options: true,
    });

    const migrateCmd = program
        .command('migrate')
        .description('Migrate contract')
        .action((options) => {
            mainProcessor(migrate, options);
        });
    addAmplifierOptions(migrateCmd, {
        contractOptions: true,
        migrateOptions: true,
        codeId: true,
        fetchCodeId: true,
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
