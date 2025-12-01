import { StdFee } from '@cosmjs/stargate';
import { Command, Option } from 'commander';

import { getChainConfig, itsEdgeContract, printInfo, prompt } from '../common';
import { ConfigManager, GATEWAY_CONTRACT_NAME, VERIFIER_CONTRACT_NAME } from '../common/config';
import { addAmplifierOptions } from './cli-utils';
import { CoordinatorManager } from './coordinator';
import { ClientManager, Options } from './processor';
import { mainProcessor } from './processor';
import { execute } from './submit-proposal';
import { executeTransaction, getChainTruncationParams, getCodeId, toArray, usesGovernanceBypass, validateItsChainChange } from './utils';

interface ContractCommandOptions extends Omit<Options, 'contractName'> {
    yes?: boolean;
    title?: string;
    description?: string;
    chains?: string[];
    itsEdgeContract?: string;
    itsMsgTranslator?: string;
    update?: boolean;
    contractName?: string;
    msg?: string | string[];
    epochDuration?: string;
    participationThreshold?: string;
    rewardsPerEpoch?: string;
    salt?: string;
    admin?: string;
    gatewayCodeId?: number;
    verifierCodeId?: number;
    proverCodeId?: number;
    fetchCodeId?: boolean;
    [key: string]: unknown;
}

const confirmDirectExecution = (options: ContractCommandOptions, messages: string | string[], contractAddress: string): boolean => {
    printInfo('Contract address', contractAddress);

    const msgs = toArray(messages);
    msgs.forEach((msg, index) => {
        const message = typeof msg === 'string' ? JSON.parse(msg) : msg;
        printInfo(`Message ${index + 1}/${msgs.length}`, JSON.stringify(message, null, 2));
    });

    if (prompt('Proceed with direct execution?', options.yes)) {
        return false;
    }
    return true;
};

const executeDirectly = async (
    client: ClientManager,
    contractAddress: string,
    msg: string | string[],
    fee?: string | StdFee,
): Promise<void> => {
    const msgs = toArray(msg);

    for (let i = 0; i < msgs.length; i++) {
        const msgJson = msgs[i];
        const message = typeof msgJson === 'string' ? JSON.parse(msgJson) : msgJson;

        const { transactionHash } = await executeTransaction(client, contractAddress, message, fee);
        printInfo(`Transaction ${i + 1}/${msgs.length} executed`, transactionHash);
    }
};

const executeContractMessage = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    contractName: string,
    msg: string | string[],
    fee?: string | StdFee,
): Promise<void> => {
    const contractAddress = config.validateRequired(config.getContractConfig(contractName).address, `${contractName}.address`);

    const msgArray = toArray(msg);

    if (usesGovernanceBypass(config, contractName)) {
        if (!confirmDirectExecution(options, msgArray, contractAddress)) {
            return;
        }
        return executeDirectly(client, contractAddress, msg, fee);
    } else {
        if (!options.title || !options.description) {
            throw new Error('Title and description are required for proposal submission');
        }
        return execute(client, config, { ...options, contractName, msg }, undefined, fee);
    }
};

const registerItsChain = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    _args?: string[],
    fee?: string | StdFee,
): Promise<void> => {
    if (!options.chains || options.chains.length === 0) {
        throw new Error('At least one chain is required');
    }

    if (options.itsEdgeContract && options.chains.length > 1) {
        throw new Error('Cannot use --its-edge-contract option with multiple chains.');
    }

    const itsMsgTranslator = options.itsMsgTranslator || config.axelar?.contracts?.ItsAbiTranslator?.address;

    if (!itsMsgTranslator) {
        throw new Error('ItsMsgTranslator address is required for registerItsChain');
    }

    const chains = options.chains.map((chain) => {
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
        for (let i = 0; i < options.chains.length; i++) {
            const chain = options.chains[i];
            await validateItsChainChange(client, config, chain, chains[i]);
        }
    }

    const operation = options.update ? 'update' : 'register';
    const msg = `{ "${operation}_chains": { "chains": ${JSON.stringify(chains)} } }`;

    if (!options.title || !options.description) {
        const chainsList = options.chains.join(', ');
        options.title = options.title || `Register ${chainsList} on ITS Hub`;
        options.description = options.description || `Register ${chainsList} on ITS Hub`;
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
    const serviceRegistry = config.axelar?.contracts?.ServiceRegistry?.address;
    const router = config.axelar?.contracts?.Router?.address;
    const multisig = config.axelar?.contracts?.Multisig?.address;

    const msg = JSON.stringify({
        register_protocol: {
            service_registry_address: serviceRegistry,
            router_address: router,
            multisig_address: multisig,
        },
    });

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
    _args?: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const { chainName } = options;
    const coordinator = new CoordinatorManager(config);
    const message = coordinator.constructRegisterDeploymentMessage(chainName);
    const msg = JSON.stringify(message);

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
    _args?: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const { chainName, epochDuration, participationThreshold, rewardsPerEpoch } = options;

    let parsedThreshold: string[];
    try {
        parsedThreshold = JSON.parse(participationThreshold);
        if (!Array.isArray(parsedThreshold)) {
            throw new Error('Participation threshold must be a JSON array');
        }
    } catch (error) {
        throw new Error(`Invalid participation threshold format: ${error instanceof Error ? error.message : String(error)}`);
    }

    const votingVerifierConfig = config.getVotingVerifierContract(chainName);
    const votingVerifierAddress = config.validateRequired(votingVerifierConfig.address, `VotingVerifier[${chainName}].address`);

    const multisigConfig = config.getContractConfig('Multisig');
    const multisigAddress = config.validateRequired(multisigConfig.address, 'Multisig.address');

    if (!options.title || !options.description) {
        options.title = options.title || `Create reward pools for ${chainName}`;
        options.description = options.description || `Create reward pools for ${chainName} voting verifier and multisig`;
    }

    const messages = [
        JSON.stringify({
            create_pool: {
                params: {
                    epoch_duration: epochDuration,
                    participation_threshold: parsedThreshold,
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
                    participation_threshold: parsedThreshold,
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
    const { chainName, salt, gatewayCodeId, verifierCodeId, proverCodeId, admin } = options;

    const coordinatorAddress = config.validateRequired(config.getContractConfig('Coordinator').address, 'Coordinator.address');

    if (!admin) {
        throw new Error('Admin address is required when instantiating chain contracts');
    }

    if (!salt) {
        throw new Error('Salt is required when instantiating chain contracts');
    }

    const chainConfig = config.getChainConfig(chainName);
    const multisigProverContractName = config.getMultisigProverContractForChainType(chainConfig.chainType);

    let gatewayConfig = config.getGatewayContract(chainName);
    let votingVerifierConfig = config.getVotingVerifierContract(chainName);
    let multisigProverConfig = config.getMultisigProverContract(chainName);

    if (options.fetchCodeId) {
        const gatewayCode = gatewayCodeId || (await getCodeId(client, config, { ...options, contractName: GATEWAY_CONTRACT_NAME }));
        const votingVerifierCode =
            verifierCodeId || (await getCodeId(client, config, { ...options, contractName: VERIFIER_CONTRACT_NAME }));
        const multisigProverCode =
            proverCodeId || (await getCodeId(client, config, { ...options, contractName: multisigProverContractName }));
        gatewayConfig.codeId = gatewayCode;
        votingVerifierConfig.codeId = votingVerifierCode;
        multisigProverConfig.codeId = multisigProverCode;
    } else {
        if (!gatewayConfig.codeId && !gatewayCodeId) {
            throw new Error(
                'Gateway code ID is required when --fetchCodeId is not used. Please provide it with --gatewayCodeId or in the config',
            );
        }
        if (!votingVerifierConfig.codeId && !verifierCodeId) {
            throw new Error(
                'VotingVerifier code ID is required when --fetchCodeId is not used. Please provide it with --verifierCodeId or in the config',
            );
        }
        if (!multisigProverConfig.codeId && !proverCodeId) {
            throw new Error(
                'MultisigProver code ID is required when --fetchCodeId is not used. Please provide it with --proverCodeId or in the config',
            );
        }

        gatewayConfig.codeId = gatewayCodeId || gatewayConfig.codeId;
        votingVerifierConfig.codeId = verifierCodeId || votingVerifierConfig.codeId;
        multisigProverConfig.codeId = proverCodeId || multisigProverConfig.codeId;
    }

    const coordinator = new CoordinatorManager(config);
    const message = coordinator.constructExecuteMessage(chainName, salt, admin);
    const msg = JSON.stringify(message);

    if (!options.title || !options.description) {
        options.title = options.title || `Instantiate chain contracts for ${chainName}`;
        options.description =
            options.description || `Instantiate Gateway, VotingVerifier and MultisigProver contracts for ${chainName} via Coordinator`;
    }

    // Need to save deployment info to config, so we can't use executeContractMessage
    // Handle direct execution and proposal submission separately
    const msgArray = toArray(msg);

    if (usesGovernanceBypass(config, 'Coordinator')) {
        if (!confirmDirectExecution(options, msgArray, coordinatorAddress)) {
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
            undefined,
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

    program.name('contract').description('Execute contract operations');

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
            options.chains = chains;
            return mainProcessor(registerItsChain, options);
        });
    addAmplifierOptions(registerItsChainCmd, { optionalProposalOptions: true });

    const registerProtocolCmd = program
        .command('register-protocol-contracts')
        .description('Register the main protocol contracts (e.g. Router)')
        .action((options) => mainProcessor(registerProtocol, options));
    addAmplifierOptions(registerProtocolCmd, { optionalProposalOptions: true });

    const registerDeploymentCmd = program
        .command('register-deployment')
        .description('Register a deployment')
        .requiredOption('-n, --chainName <chainName>', 'chain name')
        .action((options) => mainProcessor(registerDeployment, options));
    addAmplifierOptions(registerDeploymentCmd, { optionalProposalOptions: true });

    const createRewardPoolsCmd = program
        .command('create-reward-pools')
        .description('Create reward pools for VotingVerifier and Multisig contracts for a chain')
        .requiredOption('-n, --chainName <chainName>', 'chain name')
        .requiredOption('--epochDuration <epochDuration>', 'epoch duration (e.g., 3000)')
        .requiredOption('--participationThreshold <participationThreshold>', 'participation threshold as JSON array (e.g., ["7", "10"])')
        .requiredOption('--rewardsPerEpoch <rewardsPerEpoch>', 'rewards per epoch (e.g., 1000000)')
        .action((options) => mainProcessor(createRewardPools, options));
    addAmplifierOptions(createRewardPoolsCmd, { optionalProposalOptions: true });

    const instantiateChainContractsCmd = program
        .command('instantiate-chain-contracts')
        .description('Instantiate Gateway, VotingVerifier and MultisigProver contracts via Coordinator')
        .requiredOption('-n, --chainName <chainName>', 'chain name')
        .requiredOption('-s, --salt <salt>', 'salt for instantiate2')
        .option('--gatewayCodeId <gatewayCodeId>', 'code ID for Gateway contract')
        .option('--verifierCodeId <verifierCodeId>', 'code ID for VotingVerifier contract')
        .option('--proverCodeId <proverCodeId>', 'code ID for MultisigProver contract')
        .action((options) => mainProcessor(instantiateChainContracts, options));
    addAmplifierOptions(instantiateChainContractsCmd, {
        optionalProposalOptions: true,
        fetchCodeId: true,
        instantiateOptions: true,
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
