import { StdFee } from '@cosmjs/stargate';
import { Command, Option } from 'commander';

import { getChainConfig, itsEdgeContract, printInfo, prompt, validateParameters } from '../common';
import { ConfigManager, GATEWAY_CONTRACT_NAME, VERIFIER_CONTRACT_NAME } from '../common/config';
import { addAmplifierOptions } from './cli-utils';
import { CoordinatorManager } from './coordinator';
import { ClientManager, Options } from './processor';
import { mainProcessor } from './processor';
import { execute } from './submit-proposal';
import { executeTransaction, getChainTruncationParams, getCodeId, usesGovernanceBypass, validateItsChainChange } from './utils';

interface ContractCommandOptions extends Omit<Options, 'contractName'> {
    yes?: boolean;
    title?: string;
    description?: string;
    itsEdgeContract?: string;
    itsMsgTranslator?: string;
    update?: boolean;
    contractName?: string;
    msg?: string[];
    epochDuration?: string;
    participationThreshold?: string;
    rewardsPerEpoch?: string;
    salt?: string;
    admin?: string;
    fetchCodeId?: boolean;
    [key: string]: unknown;
}

const confirmDirectExecution = (options: ContractCommandOptions, messages: string[], contractAddress: string): boolean => {
    printInfo('Contract address', contractAddress);

    messages.forEach((msg, index) => {
        const message = typeof msg === 'string' ? JSON.parse(msg) : msg;
        printInfo(`Message ${index + 1}/${messages.length}`, JSON.stringify(message, null, 2));
    });

    if (prompt('Proceed with direct execution?', options.yes)) {
        return false;
    }
    return true;
};

const executeDirectly = async (client: ClientManager, contractAddress: string, msg: string[], fee?: string | StdFee): Promise<void> => {
    if (msg.length === 0) {
        throw new Error('At least one message is required');
    }

    for (let i = 0; i < msg.length; i++) {
        const msgJson = msg[i];
        const message = typeof msgJson === 'string' ? JSON.parse(msgJson) : msgJson;

        const { transactionHash } = await executeTransaction(client, contractAddress, message, fee);
        printInfo(`Transaction ${i + 1}/${msg.length} executed`, transactionHash);
    }
};

const executeContractMessage = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    contractName: string,
    msg: string[],
    fee?: string | StdFee,
): Promise<void> => {
    if (msg.length === 0) {
        throw new Error('At least one message is required');
    }

    const contractAddress = config.validateRequired(config.getContractConfig(contractName).address, `${contractName}.address`);

    if (usesGovernanceBypass(config, contractName)) {
        if (!confirmDirectExecution(options, msg, contractAddress)) {
            return;
        }
        return executeDirectly(client, contractAddress, msg, fee);
    } else {
        validateParameters({ isNonEmptyString: { title: options.title, description: options.description } });
        return execute(client, config, { ...options, contractName, msg }, [], fee);
    }
};

const registerItsChain = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    args?: string[],
    fee?: string | StdFee,
): Promise<void> => {
    if (!args || args.length === 0) {
        throw new Error('At least one chain is required');
    }

    if (options.itsEdgeContract && args.length > 1) {
        throw new Error('Cannot use --its-edge-contract option with multiple chains.');
    }

    const itsMsgTranslator =
        options.itsMsgTranslator ||
        config.validateRequired(config.getContractConfig('ItsAbiTranslator').address, 'ItsAbiTranslator.address');

    const chains = args.map((chain) => {
        const chainConfig = getChainConfig(config.chains, chain);
        const { maxUintBits, maxDecimalsWhenTruncating } = getChainTruncationParams(config, chainConfig);
        const itsEdgeContractAddress = options.itsEdgeContract || itsEdgeContract(chainConfig);

        return {
            chain: chainConfig.axelarId,
            its_edge_contract: itsEdgeContractAddress,
            msg_translator: itsMsgTranslator,
            truncation: {
                max_uint_bits: maxUintBits,
                max_decimals_when_truncating: maxDecimalsWhenTruncating,
            },
        };
    });

    if (options.update) {
        for (let i = 0; i < args.length; i++) {
            const chain = args[i];
            await validateItsChainChange(client, config, chain, chains[i]);
        }
    }

    const operation = options.update ? 'update' : 'register';
    const msg = [JSON.stringify({ [`${operation}_chains`]: { chains } })];

    if (!options.title || !options.description) {
        const chainsList = args.join(', ');
        options.title = options.title || `${operation} ${chainsList} on ITS Hub`;
        options.description = options.description || `${operation} ${chainsList} on ITS Hub`;
    }

    return executeContractMessage(client, config, options, 'InterchainTokenService', msg, fee);
};

const registerProtocol = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    _args?: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const serviceRegistry = config.validateRequired(config.getContractConfig('ServiceRegistry').address, 'ServiceRegistry.address');
    const router = config.validateRequired(config.getContractConfig('Router').address, 'Router.address');
    const multisig = config.validateRequired(config.getContractConfig('Multisig').address, 'Multisig.address');

    const msg = [
        JSON.stringify({
            register_protocol: {
                service_registry_address: serviceRegistry,
                router_address: router,
                multisig_address: multisig,
            },
        }),
    ];

    if (!options.title || !options.description) {
        options.title = options.title || 'Register Protocol contracts on Coordinator';
        options.description = options.description || 'Register Protocol contracts on Coordinator';
    }

    return executeContractMessage(client, config, options, 'Coordinator', msg, fee);
};

const registerDeployment = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    args?: string[],
    fee?: string | StdFee,
): Promise<void> => {
    if (!args || args.length === 0) {
        throw new Error('chainName is required');
    }
    const [chainName] = args;
    const coordinator = new CoordinatorManager(config);
    const message = coordinator.constructRegisterDeploymentMessage(chainName);
    const msg = [JSON.stringify(message)];

    if (!options.title || !options.description) {
        options.title = options.title || `Register ${chainName} deployment on Coordinator`;
        options.description = options.description || `Register ${chainName} deployment on Coordinator`;
    }

    return executeContractMessage(client, config, options, 'Coordinator', msg, fee);
};

const createRewardPools = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    args?: string[],
    fee?: string | StdFee,
): Promise<void> => {
    if (!args || args.length === 0) {
        throw new Error('chainName is required');
    }
    const [chainName] = args;
    const { epochDuration, participationThreshold, rewardsPerEpoch } = options;

    let parsedThreshold;
    try {
        parsedThreshold = JSON.parse(participationThreshold);
    } catch {
        throw new Error(`Invalid participationThreshold format. Expected JSON array, got: ${participationThreshold}`);
    }

    const threshold: string[] = config.validateThreshold(parsedThreshold, '--participationThreshold');

    const votingVerifierAddress = config.validateRequired(
        config.getVotingVerifierContract(chainName).address,
        `VotingVerifier[${chainName}].address`,
    );
    const multisigAddress = config.validateRequired(config.getContractConfig('Multisig').address, 'Multisig.address');

    if (!options.title || !options.description) {
        options.title = options.title || `Create reward pools for ${chainName}`;
        options.description = options.description || `Create reward pools for ${chainName} voting verifier and multisig`;
    }

    const messages = [
        JSON.stringify({
            create_pool: {
                params: {
                    epoch_duration: epochDuration,
                    participation_threshold: threshold,
                    rewards_per_epoch: rewardsPerEpoch,
                },
                pool_id: {
                    chain_name: chainName,
                    contract: votingVerifierAddress,
                },
            },
        }),
        JSON.stringify({
            create_pool: {
                params: {
                    epoch_duration: epochDuration,
                    participation_threshold: threshold,
                    rewards_per_epoch: rewardsPerEpoch,
                },
                pool_id: {
                    chain_name: chainName,
                    contract: multisigAddress,
                },
            },
        }),
    ];

    return executeContractMessage(client, config, options, 'Rewards', messages, fee);
};

const instantiateChainContracts = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    _args?: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const { chainName, salt, admin } = options;

    const coordinatorAddress = config.validateRequired(config.getContractConfig('Coordinator').address, 'Coordinator.address');

    validateParameters({ isNonEmptyString: { admin, salt } });

    const chainConfig = config.getChainConfig(chainName);
    const multisigProverContractName = config.getMultisigProverContractForChainType(chainConfig.chainType);

    const gatewayConfig = config.getGatewayContract(chainName);
    const votingVerifierConfig = config.getVotingVerifierContract(chainName);
    const multisigProverConfig = config.getMultisigProverContract(chainName);

    if (options.fetchCodeId) {
        gatewayConfig.codeId = await getCodeId(client, config, { ...options, contractName: GATEWAY_CONTRACT_NAME });
        votingVerifierConfig.codeId = await getCodeId(client, config, { ...options, contractName: VERIFIER_CONTRACT_NAME });
        multisigProverConfig.codeId = await getCodeId(client, config, { ...options, contractName: multisigProverContractName });
    } else {
        if (!gatewayConfig.codeId) {
            throw new Error(
                'Gateway code ID is required when --fetchCodeId is not used. Please provide it in the config or use --fetchCodeId',
            );
        }
        if (!votingVerifierConfig.codeId) {
            throw new Error(
                'VotingVerifier code ID is required when --fetchCodeId is not used. Please provide it in the config or use --fetchCodeId',
            );
        }
        if (!multisigProverConfig.codeId) {
            throw new Error(
                'MultisigProver code ID is required when --fetchCodeId is not used. Please provide it in the config or use --fetchCodeId',
            );
        }
    }

    const coordinator = new CoordinatorManager(config);
    const message = coordinator.constructExecuteMessage(chainName, salt, admin);
    const msg = [JSON.stringify(message)];

    if (!options.title || !options.description) {
        options.title = options.title || `Instantiate chain contracts for ${chainName}`;
        options.description =
            options.description || `Instantiate Gateway, VotingVerifier and MultisigProver contracts for ${chainName} via Coordinator`;
    }

    // Need to save deployment info to config, so we can't use executeContractMessage
    // Handle direct execution and proposal submission separately
    if (usesGovernanceBypass(config, 'Coordinator')) {
        if (!confirmDirectExecution(options, msg, coordinatorAddress)) {
            return;
        }
        await executeDirectly(client, coordinatorAddress, msg, fee);
    } else {
        await execute(
            client,
            config,
            {
                ...options,
                contractName: 'Coordinator',
                msg,
            },
            [],
            fee,
        );
    }

    if (!config.axelar.contracts.Coordinator.deployments) {
        config.axelar.contracts.Coordinator.deployments = {};
    }
    config.axelar.contracts.Coordinator.deployments[chainName] = {
        deploymentName: message.instantiate_chain_contracts.deployment_name,
        salt: salt,
    };
};

const programHandler = () => {
    const program = new Command();

    program.name('contract').description('Execute cosmwasm contract operations');

    const registerItsChainCmd = program
        .command('its-hub-register-chains')
        .description('Register or update an InterchainTokenService chain')
        .argument('<chains...>', 'list of chains to register or update on InterchainTokenService hub')
        .addOption(
            new Option(
                '--its-msg-translator <itsMsgTranslator>',
                'address for the message translation contract associated with the chain being registered or updated on ITS Hub',
            ),
        )
        .addOption(
            new Option(
                '--its-edge-contract <itsEdgeContract>',
                'address for the ITS edge contract associated with the chain being registered or updated on ITS Hub',
            ),
        )
        .addOption(new Option('--update', 'update existing chain registration instead of registering new chain'))
        .action((chains, options) => {
            return mainProcessor(registerItsChain, options, chains);
        });
    addAmplifierOptions(registerItsChainCmd);

    const registerProtocolCmd = program
        .command('register-protocol-contracts')
        .description('Register the main protocol contracts (e.g. Router)')
        .action((options) => mainProcessor(registerProtocol, options));
    addAmplifierOptions(registerProtocolCmd);

    const registerDeploymentCmd = program
        .command('register-deployment')
        .description('Register a deployment')
        .argument('<chainName>', 'chain name')
        .action((chainName, options) => {
            return mainProcessor(registerDeployment, options, [chainName]);
        });
    addAmplifierOptions(registerDeploymentCmd);

    const createRewardPoolsCmd = program
        .command('create-reward-pools')
        .description('Create reward pools for VotingVerifier and Multisig contracts for a chain')
        .argument('<chainName>', 'chain name')
        .requiredOption('--epochDuration <epochDuration>', 'epoch duration (e.g., 3000)')
        .requiredOption('--participationThreshold <participationThreshold>', 'participation threshold as JSON array (e.g., ["7", "10"])')
        .requiredOption('--rewardsPerEpoch <rewardsPerEpoch>', 'rewards per epoch (e.g., 1000000)')
        .action((chainName, options) => {
            return mainProcessor(createRewardPools, options, [chainName]);
        });
    addAmplifierOptions(createRewardPoolsCmd);

    const instantiateChainContractsCmd = program
        .command('instantiate-chain-contracts')
        .description('Instantiate Gateway, VotingVerifier and MultisigProver contracts via Coordinator')
        .requiredOption('-n, --chainName <chainName>', 'chain name')
        .requiredOption('-s, --salt <salt>', 'salt for instantiate2')
        .requiredOption('--admin <admin>', 'admin address for the instantiated contracts')
        .action((options) => mainProcessor(instantiateChainContracts, options));
    addAmplifierOptions(instantiateChainContractsCmd, {
        fetchCodeId: true,
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
