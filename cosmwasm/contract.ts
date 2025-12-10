import { StdFee } from '@cosmjs/stargate';
import { Command, Option } from 'commander';

import { getChainConfig, printInfo, validateParameters } from '../common';
import { ConfigManager, GATEWAY_CONTRACT_NAME, VERIFIER_CONTRACT_NAME } from '../common/config';
import { addAmplifierOptions } from './cli-utils';
import { CoordinatorManager } from './coordinator';
import { ClientManager, Options } from './processor';
import { mainProcessor } from './processor';
import { executeByGovernance } from './submit-proposal';
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
        const description = options.description || defaultDescription;
        validateParameters({ isNonEmptyString: { title, description } });
        const stringifiedMsg = msg.map((m) => JSON.stringify(m));
        return executeByGovernance(client, config, { ...options, contractName, msg: stringifiedMsg, title, description }, [], fee);
    }

    printDirectExecutionInfo(msg, contractAddress);
    return executeDirectly(client, contractAddress, msg, fee);
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
    args?: string[],
    fee?: string | StdFee,
): Promise<void> => {
    if (!args || args.length === 0) {
        throw new Error('At least one chain is required');
    }

    const chains = buildItsHubChains(config, args);

    const msg = [{ register_chains: { chains } }];

    const chainsList = args.join(', ');
    const defaultTitle = `Register ${chainsList} on ITS Hub`;
    const defaultDescription = `Register ${chainsList} on ITS Hub`;

    return executeContractMessage(client, config, options, 'InterchainTokenService', msg, fee, defaultTitle, defaultDescription);
};

const updateItsChain = async (
    client: ClientManager,
    config: ConfigManager,
    options: ContractCommandOptions,
    args?: string[],
    fee?: string | StdFee,
): Promise<void> => {
    if (!args || args.length === 0) {
        throw new Error('At least one chain is required');
    }

    const chains = buildItsHubChains(config, args);

    for (let i = 0; i < args.length; i++) {
        const chain = args[i];
        await validateItsChainChange(client, config, chain, chains[i]);
    }

    const msg = [{ update_chains: { chains } }];

    const chainsList = args.join(', ');
    const defaultTitle = `Update ${chainsList} on ITS Hub`;
    const defaultDescription = `Update ${chainsList} on ITS Hub`;

    return executeContractMessage(client, config, options, 'InterchainTokenService', msg, fee, defaultTitle, defaultDescription);
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
    const defaultDescription = 'Register Protocol contracts on Coordinator';

    return executeContractMessage(client, config, options, 'Coordinator', msg, fee, defaultTitle, defaultDescription);
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
    const msg = [message];

    const defaultTitle = `Register ${chainName} deployment on Coordinator`;
    const defaultDescription = `Register ${chainName} deployment on Coordinator`;

    return executeContractMessage(client, config, options, 'Coordinator', msg, fee, defaultTitle, defaultDescription);
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

    if (!gatewayConfig.codeId) {
        throw new Error('Gateway code ID is required when --fetchCodeId is not used. Please provide it in the config or use --fetchCodeId');
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
