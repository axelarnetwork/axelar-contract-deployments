import { StdFee } from '@cosmjs/stargate';
import { Command, Option } from 'commander';

import { getChainConfig, printInfo, validateParameters } from '../common';
import { ConfigManager, GATEWAY_CONTRACT_NAME, VERIFIER_CONTRACT_NAME } from '../common/config';
import { addAmplifierOptions } from './cli-utils';
import { CoordinatorManager } from './coordinator';
import { ClientManager, Options } from './processor';
import { mainProcessor } from './processor';
import { executeByGovernance } from './proposal-utils';
import { executeTransaction, getCodeId, itsHubChainParams, validateGovernanceMode, validateItsChainChange } from './utils';

interface ContractCommandOptions extends Omit<Options, 'contractName'> {
    yes?: boolean;
    governance?: boolean;
    title?: string;
    description?: string;
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

const printDirectExecutionInfo = (messages: object[], contractAddress: string): void => {
    printInfo('Contract address', contractAddress);

    messages.forEach((msg, index) => {
        printInfo(`Message ${index + 1}/${messages.length}`, JSON.stringify(msg, null, 2));
    });
};

const executeDirectly = async (client: ClientManager, contractAddress: string, msg: object[], fee?: string | StdFee): Promise<void> => {
    if (msg.length === 0) {
        throw new Error('At least one message is required');
    }

    for (let i = 0; i < msg.length; i++) {
        const message = msg[i];

        const { transactionHash } = await executeTransaction(client, contractAddress, message, fee);
        printInfo(`Transaction ${i + 1}/${msg.length} executed`, transactionHash);
    }
};

const executeContractMessage = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    contractName: string,
    msg: object[],
    fee?: string | StdFee,
    defaultTitle?: string,
    defaultDescription?: string,
    chainName?: string,
): Promise<void> => {
    if (msg.length === 0) {
        throw new Error('At least one message is required');
    }

    const contractAddress = config.validateRequired(config.getContractConfig(contractName).address, `${contractName}.address`);

    if (options.governance) {
        validateGovernanceMode(config, contractName, chainName);
        const title = options.title || defaultTitle;
        const description = options.description || defaultDescription || defaultTitle;
        validateParameters({ isNonEmptyString: { title, description } });
        const stringifiedMsg = msg.map((m) => JSON.stringify(m));
        await executeByGovernance(client, config, { ...options, contractName, msg: stringifiedMsg, title, description }, [], fee);
        return;
    }

    printDirectExecutionInfo(msg, contractAddress);
    await executeDirectly(client, contractAddress, msg, fee);
};

const buildItsHubChains = (config: ConfigManager, chainNames: string[]) => {
    return chainNames.map((chain) => {
        const chainConfig = getChainConfig(config.chains, chain);
        const { itsEdgeContractAddress, itsMsgTranslator, maxUintBits, maxDecimalsWhenTruncating } = itsHubChainParams(config, chainConfig);

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
};

const registerItsChain = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    args: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const chains = buildItsHubChains(config, args);

    const msg = [{ register_chains: { chains } }];

    const defaultTitle = `Register ${args.join(', ')} on ITS Hub`;

    return executeContractMessage(client, config, options, 'InterchainTokenService', msg, fee, defaultTitle);
};

const updateItsChain = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    args: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const chains = buildItsHubChains(config, args);

    for (let i = 0; i < args.length; i++) {
        const chain = args[i];
        await validateItsChainChange(client, config, chain, chains[i]);
    }

    const msg = [{ update_chains: { chains } }];

    const defaultTitle = `Update ${args.join(', ')} on ITS Hub`;

    return executeContractMessage(client, config, options, 'InterchainTokenService', msg, fee, defaultTitle);
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
        {
            register_protocol: {
                service_registry_address: serviceRegistry,
                router_address: router,
                multisig_address: multisig,
            },
        },
    ];

    const defaultTitle = 'Register Protocol contracts on Coordinator';

    return executeContractMessage(client, config, options, 'Coordinator', msg, fee, defaultTitle);
};

const registerDeployment = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    args: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const [chainName] = args;
    const coordinator = new CoordinatorManager(config);
    const message = coordinator.constructRegisterDeploymentMessage(chainName);
    const msg = [message];

    const defaultTitle = `Register ${chainName} deployment on Coordinator`;

    return executeContractMessage(client, config, options, 'Coordinator', msg, fee, defaultTitle);
};

const createRewardPools = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    args: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const [chainName] = args;
    const { epochDuration, participationThreshold, rewardsPerEpoch } = options;

    const threshold: string[] = config.parseThreshold(participationThreshold, '--participationThreshold');

    const votingVerifierAddress = config.validateRequired(
        config.getVotingVerifierContract(chainName).address,
        `VotingVerifier[${chainName}].address`,
    );
    const multisigAddress = config.validateRequired(config.getContractConfig('Multisig').address, 'Multisig.address');

    const messages = [
        {
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
        },
        {
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
        },
    ];

    const defaultTitle = `Create reward pools for ${chainName}`;
    const defaultDescription = `Create reward pools for ${chainName} voting verifier and multisig`;

    return executeContractMessage(client, config, options, 'Rewards', messages, fee, defaultTitle, defaultDescription);
};

// ==================== Emergency Operations ====================

// Router operations (Admin EOA only - cannot use governance)
const routerFreezeChain = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    args: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const [chainName] = args;
    const chainConfig = getChainConfig(config.chains, chainName);
    const msg = [{ freeze_chain: { chain: chainConfig.axelarId } }];

    if (options.governance) {
        throw new Error('Router freeze_chain can only be executed by Admin EOA, not via governance');
    }

    const contractAddress = config.validateRequired(config.getContractConfig('Router').address, 'Router.address');
    printDirectExecutionInfo(msg, contractAddress);
    return executeDirectly(client, contractAddress, msg, fee);
};

const routerUnfreezeChain = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    args: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const [chainName] = args;
    const chainConfig = getChainConfig(config.chains, chainName);
    const msg = [{ unfreeze_chain: { chain: chainConfig.axelarId } }];

    if (options.governance) {
        throw new Error('Router unfreeze_chain can only be executed by Admin EOA, not via governance');
    }

    const contractAddress = config.validateRequired(config.getContractConfig('Router').address, 'Router.address');
    printDirectExecutionInfo(msg, contractAddress);
    return executeDirectly(client, contractAddress, msg, fee);
};

const routerDisableRouting = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    _args?: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const msg = [{ disable_routing: {} }];

    if (options.governance) {
        throw new Error('Router disable_routing can only be executed by Admin EOA, not via governance');
    }

    const contractAddress = config.validateRequired(config.getContractConfig('Router').address, 'Router.address');
    printDirectExecutionInfo(msg, contractAddress);
    return executeDirectly(client, contractAddress, msg, fee);
};

const routerEnableRouting = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    _args?: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const msg = [{ enable_routing: {} }];

    if (options.governance) {
        throw new Error('Router enable_routing can only be executed by Admin EOA, not via governance');
    }

    const contractAddress = config.validateRequired(config.getContractConfig('Router').address, 'Router.address');
    printDirectExecutionInfo(msg, contractAddress);
    return executeDirectly(client, contractAddress, msg, fee);
};

// Multisig operations (Admin EOA or Governance)
const multisigDisableSigning = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    _args?: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const msg = [{ disable_signing: {} }];
    const defaultTitle = 'Disable signing on Multisig';
    return executeContractMessage(client, config, options, 'Multisig', msg, fee, defaultTitle);
};

const multisigEnableSigning = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    _args?: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const msg = [{ enable_signing: {} }];
    const defaultTitle = 'Enable signing on Multisig';
    return executeContractMessage(client, config, options, 'Multisig', msg, fee, defaultTitle);
};

// ITS Hub operations (Admin EOA or Governance)
const itsDisableExecution = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    _args?: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const msg = [{ disable_execution: {} }];
    const defaultTitle = 'Disable execution on ITS Hub';
    return executeContractMessage(client, config, options, 'InterchainTokenService', msg, fee, defaultTitle);
};

const itsEnableExecution = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    _args?: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const msg = [{ enable_execution: {} }];
    const defaultTitle = 'Enable execution on ITS Hub';
    return executeContractMessage(client, config, options, 'InterchainTokenService', msg, fee, defaultTitle);
};

const itsFreezeChain = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    args: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const [chainName] = args;
    const chainConfig = getChainConfig(config.chains, chainName);
    const msg = [{ freeze_chain: { chain: chainConfig.axelarId } }];
    const defaultTitle = `Freeze chain ${chainName} on ITS Hub`;
    return executeContractMessage(client, config, options, 'InterchainTokenService', msg, fee, defaultTitle);
};

const itsUnfreezeChain = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    args: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const [chainName] = args;
    const chainConfig = getChainConfig(config.chains, chainName);
    const msg = [{ unfreeze_chain: { chain: chainConfig.axelarId } }];
    const defaultTitle = `Unfreeze chain ${chainName} on ITS Hub`;
    return executeContractMessage(client, config, options, 'InterchainTokenService', msg, fee, defaultTitle);
};

// ==================== End Emergency Operations ====================

const instantiateChainContracts = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    _args?: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const { chainName, salt, admin } = options;

    validateParameters({ isNonEmptyString: { admin, salt } });

    const chainConfig = config.getChainConfig(chainName);
    const multisigProverContractName = config.getMultisigProverContractForChainType(chainConfig.chainType);
    const gatewayContractName = config.getGatewayContractForChainType(chainConfig.chainType);
    const verifierContractName = config.getVotingVerifierContractForChainType(chainConfig.chainType);

    config.initContractConfig(gatewayContractName, chainName);

    const gatewayConfig = config.getGatewayContract(chainName);
    const votingVerifierConfig = config.getVotingVerifierContract(chainName);
    const multisigProverConfig = config.getMultisigProverContract(chainName);

    if (options.fetchCodeId) {
        gatewayConfig.codeId = await getCodeId(client, config, { ...options, contractName: gatewayContractName });
        votingVerifierConfig.codeId = await getCodeId(client, config, { ...options, contractName: verifierContractName });
        multisigProverConfig.codeId = await getCodeId(client, config, { ...options, contractName: multisigProverContractName });
    }

    validateParameters({
        isNumber: {
            gatewayCodeId: gatewayConfig.codeId,
            votingVerifierCodeId: votingVerifierConfig.codeId,
            multisigProverCodeId: multisigProverConfig.codeId,
        },
    });

    const coordinator = new CoordinatorManager(config);
    const message = coordinator.constructExecuteMessage(chainName, salt, admin);
    const msg = [message];

    const defaultTitle = `Instantiate chain contracts for ${chainName}`;
    const defaultDescription = `Instantiate Gateway, VotingVerifier and MultisigProver contracts for ${chainName} via Coordinator`;

    if (!config.axelar.contracts.Coordinator.deployments) {
        config.axelar.contracts.Coordinator.deployments = {};
    }
    config.axelar.contracts.Coordinator.deployments[chainName] = {
        deploymentName: message.instantiate_chain_contracts.deployment_name,
        salt: salt,
    };

    return executeContractMessage(client, config, options, 'Coordinator', msg, fee, defaultTitle, defaultDescription);
};

const programHandler = () => {
    const program = new Command();

    program.name('contract').description('Execute cosmwasm contract operations');

    const registerItsChainCmd = program
        .command('its-hub-register-chains')
        .description('Register an InterchainTokenService chain')
        .argument('<chains...>', 'list of chains to register on InterchainTokenService hub')
        .action((chains, options) => {
            return mainProcessor(registerItsChain, options, chains);
        });
    addAmplifierOptions(registerItsChainCmd);

    const updateItsChainCmd = program
        .command('its-hub-update-chains')
        .description('Update an InterchainTokenService chain registration')
        .argument('<chains...>', 'list of chains to update on InterchainTokenService hub')
        .action((chains, options) => {
            return mainProcessor(updateItsChain, options, chains);
        });
    addAmplifierOptions(updateItsChainCmd);

    const registerProtocolCmd = program
        .command('register-protocol')
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

    // ==================== Emergency Operations Commands ====================

    const routerFreezeChainCmd = program
        .command('router-freeze-chain')
        .description('[EMERGENCY] Freeze a chain on Router (Admin EOA only, cannot use governance)')
        .argument('<chainName>', 'chain name to freeze')
        .action((chainName, options) => mainProcessor(routerFreezeChain, options, [chainName]));
    addAmplifierOptions(routerFreezeChainCmd);

    const routerUnfreezeChainCmd = program
        .command('router-unfreeze-chain')
        .description('[EMERGENCY] Unfreeze a chain on Router (Admin EOA only, cannot use governance)')
        .argument('<chainName>', 'chain name to unfreeze')
        .action((chainName, options) => mainProcessor(routerUnfreezeChain, options, [chainName]));
    addAmplifierOptions(routerUnfreezeChainCmd);

    const routerDisableRoutingCmd = program
        .command('router-disable-routing')
        .description('[EMERGENCY] Disable routing on Router - affects ALL chains (Admin EOA only, cannot use governance)')
        .action((options) => mainProcessor(routerDisableRouting, options));
    addAmplifierOptions(routerDisableRoutingCmd);

    const routerEnableRoutingCmd = program
        .command('router-enable-routing')
        .description('[EMERGENCY] Enable routing on Router (Admin EOA only, cannot use governance)')
        .action((options) => mainProcessor(routerEnableRouting, options));
    addAmplifierOptions(routerEnableRoutingCmd);

    const multisigDisableSigningCmd = program
        .command('multisig-disable-signing')
        .description('[EMERGENCY] Disable signing on Multisig (Admin EOA or --governance)')
        .action((options) => mainProcessor(multisigDisableSigning, options));
    addAmplifierOptions(multisigDisableSigningCmd);

    const multisigEnableSigningCmd = program
        .command('multisig-enable-signing')
        .description('[EMERGENCY] Enable signing on Multisig (Admin EOA or --governance)')
        .action((options) => mainProcessor(multisigEnableSigning, options));
    addAmplifierOptions(multisigEnableSigningCmd);

    const itsDisableExecutionCmd = program
        .command('its-disable-execution')
        .description('[EMERGENCY] Disable execution on ITS Hub (Admin EOA or --governance)')
        .action((options) => mainProcessor(itsDisableExecution, options));
    addAmplifierOptions(itsDisableExecutionCmd);

    const itsEnableExecutionCmd = program
        .command('its-enable-execution')
        .description('[EMERGENCY] Enable execution on ITS Hub (Admin EOA or --governance)')
        .action((options) => mainProcessor(itsEnableExecution, options));
    addAmplifierOptions(itsEnableExecutionCmd);

    const itsFreezeChainCmd = program
        .command('its-freeze-chain')
        .description('[EMERGENCY] Freeze a chain on ITS Hub (Admin EOA or --governance)')
        .argument('<chainName>', 'chain name to freeze')
        .action((chainName, options) => mainProcessor(itsFreezeChain, options, [chainName]));
    addAmplifierOptions(itsFreezeChainCmd);

    const itsUnfreezeChainCmd = program
        .command('its-unfreeze-chain')
        .description('[EMERGENCY] Unfreeze a chain on ITS Hub (Admin EOA or --governance)')
        .argument('<chainName>', 'chain name to unfreeze')
        .action((chainName, options) => mainProcessor(itsUnfreezeChain, options, [chainName]));
    addAmplifierOptions(itsUnfreezeChainCmd);

    // ==================== End Emergency Operations Commands ====================

    program.parse();
};

if (require.main === module) {
    programHandler();
}
