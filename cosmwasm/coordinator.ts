#!/usr/bin/env ts-node
import { Command } from 'commander';
import * as fs from 'fs';
import * as path from 'path';

// Import functions from submit-proposal.js and utils.js
import { loadConfig, printError, printInfo, prompt, saveConfig } from '../common';
import { encodeExecuteContractProposal, initContractConfig, prepareClient, prepareWallet, submitProposal } from './utils';

interface ChainConfig {
    name: string;
    axelarId: string;
    chainId: number;
    rpc: string;
    tokenSymbol: string;
    decimals: number;
    confirmations?: number;
    chainType: string;
    contracts: {
        [key: string]: ContractConfig;
    };
}

interface ContractConfig {
    address?: string;
    codeId?: number;
    [key: string]: unknown;
}

interface ConfigFile {
    chains: {
        [chainName: string]: ChainConfig;
    };
}

interface GatewayParams {
    code_id: number;
    label: string;
}

interface MajorityThreshold {
    numerator: number;
    denominator: number;
}

interface VerifierParams {
    code_id: number;
    label: string;
    msg: {
        governance_address: string;
        service_name: string;
        source_gateway_address: string;
        voting_threshold: MajorityThreshold;
        block_expiry: string;
        confirmation_height: number;
        source_chain: string;
        rewards_address: string;
        msg_id_format: string;
        address_format: string;
    };
}

interface ProverParams {
    code_id: number;
    label: string;
    msg: {
        governance_address: string;
        multisig_address: string;
        signing_threshold: [string, string];
        service_name: string;
        chain_name: string;
        verifier_set_diff_threshold: number;
        encoder: string;
        key_type: string;
        domain_separator: string;
    };
}

interface InstantiateChainContractsMsg {
    instantiate_chain_contracts: {
        chain: string;
        deployment_name: string;
        salt: string;
        params: {
            gateway: GatewayParams;
            verifier: VerifierParams;
            prover: ProverParams;
        };
    };
}

interface CoordinatorOptions {
    governanceAddress?: string;
    serviceName?: string;
    rewardsAddress?: string;
    votingThreshold?: [string, string];
    signingThreshold?: [string, string];
    blockExpiry?: string;
    confirmationHeight?: number;
    msgIdFormat?: string;
    addressFormat?: string;
    verifierSetDiffThreshold?: number;
    encoder?: string;
    keyType?: string;
    domainSeparator?: string;
    salt?: string;
    yes?: boolean;
    runAs?: string;
    mnemonic?: string;
}

interface FullConfig {
    axelar?: {
        contracts?: {
            [key: string]: ContractConfig;
        };
        rpc?: string;
        gasPrice?: string;
    };
    [key: string]: unknown;
}

export class CoordinatorScript {
    private config: ConfigFile;
    private environment: string;
    private fullConfig: FullConfig;

    // Contract name mapping
    private static readonly CONTRACT_NAME_MAP: { [key: string]: string } = {
        Gateway: 'Gateway',
        Verifier: 'VotingVerifier',
        Prover: 'MultisigProver',
    };

    // Default values
    public static readonly DEFAULTS = {
        governanceAddress: 'axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj',
        serviceName: 'amplifier',
        votingThreshold: ['51', '100'] as [string, string],
        signingThreshold: ['51', '100'] as [string, string],
        blockExpiry: '10',
        confirmationHeight: 1000000,
        msgIdFormat: 'hex_tx_hash_and_event_index',
        addressFormat: 'eip55',
        verifierSetDiffThreshold: 1,
        encoder: 'abi',
        keyType: 'ecdsa',
        domainSeparator: '0x79191ee0824b0f995492dc4ac6e737040f4d9fd4501f6078e56671da70968259',
    };

    constructor(environment: string) {
        this.environment = environment;
        this.loadConfig();
        this.fullConfig = loadConfig(this.environment);
    }

    private loadConfig(): void {
        const configPath = path.join(__dirname, '../axelar-chains-config/info', `${this.environment}.json`);

        if (!fs.existsSync(configPath)) {
            throw new Error(`Config file not found: ${configPath}`);
        }

        const configData = fs.readFileSync(configPath, 'utf8');
        this.config = JSON.parse(configData);
    }

    private getChainConfig(chainName: string): ChainConfig {
        const chainConfig = this.config.chains[chainName];
        if (!chainConfig) {
            throw new Error(`Chain '${chainName}' not found in ${this.environment} config`);
        }
        return chainConfig;
    }

    private generateDeploymentName(chainName: string): string {
        const timestamp = Date.now();
        return `${chainName}-deployment-${timestamp}`;
    }

    private generateSalt(): string {
        return Math.random().toString(36).substring(2, 15) + Math.random().toString(36).substring(2, 15);
    }

    private validateOptions(options: CoordinatorOptions): void {
        // Validate threshold formats
        if (options.votingThreshold && !Array.isArray(options.votingThreshold)) {
            throw new Error('Voting threshold must be an array of two strings (e.g., ["51", "100"])');
        }
        if (options.signingThreshold && !Array.isArray(options.signingThreshold)) {
            throw new Error('Signing threshold must be an array of two strings (e.g., ["51", "100"])');
        }

        // Validate numeric values
        if (options.confirmationHeight && isNaN(parseInt(options.confirmationHeight.toString()))) {
            throw new Error('Confirmation height must be a valid number');
        }
        if (options.verifierSetDiffThreshold && isNaN(parseInt(options.verifierSetDiffThreshold.toString()))) {
            throw new Error('Verifier set diff threshold must be a valid number');
        }

        // Validate addresses
        if (options.governanceAddress && !this.isValidCosmosAddress(options.governanceAddress)) {
            throw new Error('Invalid governance address format');
        }
        if (options.rewardsAddress && !this.isValidCosmosAddress(options.rewardsAddress)) {
            throw new Error('Invalid rewards address format');
        }

        // Validate domain separator format
        if (options.domainSeparator && !options.domainSeparator.startsWith('0x')) {
            throw new Error('Domain separator must start with 0x');
        }
    }

    private isValidCosmosAddress(address: string): boolean {
        // Basic validation for Cosmos addresses (axelar prefix)
        return address.startsWith('axelar') && address.length > 10;
    }

    private getContractConfig(contractName: string): ContractConfig {
        const configContractName = CoordinatorScript.CONTRACT_NAME_MAP[contractName];
        if (!configContractName) {
            throw new Error(
                `Unknown contract name: ${contractName}. Supported contracts: ${Object.keys(CoordinatorScript.CONTRACT_NAME_MAP).join(', ')}`,
            );
        }

        const axelarContracts = this.fullConfig.axelar?.contracts;
        if (!axelarContracts || !axelarContracts[configContractName]) {
            throw new Error(`Contract ${configContractName} not found in axelar contracts config for environment ${this.environment}`);
        }

        return axelarContracts[configContractName];
    }

    private async getCodeIdFromConfig(contractName: string, chainName: string, client?: unknown, wallet?: unknown): Promise<number> {
        printInfo(`Getting code ID for ${contractName} on chain ${chainName}...`);

        const contractConfig = this.getContractConfig(contractName);

        // Check if the contract has a specific configuration for this chain
        if (contractConfig[chainName] && (contractConfig[chainName] as ContractConfig).codeId) {
            printInfo(`Found chain-specific code ID for ${contractName}: ${(contractConfig[chainName] as ContractConfig).codeId}`);
            return (contractConfig[chainName] as ContractConfig).codeId!;
        }

        // Fallback to the global codeId if no chain-specific config
        if (contractConfig.codeId) {
            printInfo(`Found global code ID for ${contractName}: ${contractConfig.codeId}`);
            return contractConfig.codeId;
        }

        // If no codeId found, try to fetch from chain using code hash
        if (contractConfig.storeCodeProposalCodeHash) {
            if (!client || !wallet) {
                throw new Error(`Code ID for ${contractName} needs to be fetched from chain, but client and wallet are required`);
            }
            printInfo(`Fetching code ID for ${contractName} from chain using code hash...`);
            return await this.fetchCodeIdFromChain(contractName, contractConfig, client, wallet);
        }

        throw new Error(
            `Code ID not found for contract: ${contractName}. Please ensure the contract has been deployed and the code ID is available in the configuration.`,
        );
    }

    private async fetchCodeIdFromChain(
        contractName: string,
        contractConfig: ContractConfig,
        client: unknown,
        wallet: unknown,
    ): Promise<number> {
        if (!contractConfig.storeCodeProposalCodeHash) {
            throw new Error(`No storeCodeProposalCodeHash found for ${contractName}`);
        }

        const { fetchCodeIdFromCodeHash } = await import('./utils');

        const contractBaseConfig = {
            storeCodeProposalCodeHash: contractConfig.storeCodeProposalCodeHash,
        };

        try {
            const codeId = await fetchCodeIdFromCodeHash(client, contractBaseConfig);
            printInfo(`Successfully fetched code ID ${codeId} for ${contractName} from chain`);
            return codeId;
        } catch (error) {
            throw new Error(
                `Failed to fetch code ID for ${contractName} from chain: ${(error as Error).message}. Please ensure the contract has been deployed.`,
            );
        }
    }

    private getContractAddress(contractName: string, chainName: string): string {
        printInfo(`Getting contract address for ${contractName} on chain ${chainName}...`);

        const chainConfig = this.getChainConfig(chainName);
        const contract = chainConfig.contracts[contractName];

        if (!contract) {
            throw new Error(
                `Contract ${contractName} not found for chain ${chainName}. Available contracts: ${Object.keys(chainConfig.contracts).join(', ')}`,
            );
        }

        if (!contract.address) {
            throw new Error(
                `Contract ${contractName} address not found for chain ${chainName}. Please ensure the contract has been deployed.`,
            );
        }

        printInfo(`Found contract address for ${contractName}: ${contract.address}`);
        return contract.address;
    }

    private getDefaultValue<T>(value: T | undefined, defaultValue: T): T {
        return value !== undefined ? value : defaultValue;
    }

    private async constructExecuteMessage(
        chainName: string,
        options: CoordinatorOptions,
        client?: unknown,
        wallet?: unknown,
    ): Promise<InstantiateChainContractsMsg> {
        printInfo(`Constructing execute message for chain: ${chainName}`);

        const chainConfig = this.getChainConfig(chainName);
        const deploymentName = this.generateDeploymentName(chainName);

        // Use provided salt or generate one
        const salt = options.salt || this.generateSalt();
        if (options.salt) {
            printInfo(`Using provided salt: ${salt}`);
        } else {
            printInfo(`Generated salt: ${salt}`);
        }

        // Get values with defaults
        const governanceAddress = this.getDefaultValue(options.governanceAddress, CoordinatorScript.DEFAULTS.governanceAddress);
        const serviceName = this.getDefaultValue(options.serviceName, CoordinatorScript.DEFAULTS.serviceName);
        const rewardsAddress = this.getDefaultValue(options.rewardsAddress, governanceAddress);

        printInfo(`Using governance address: ${governanceAddress}`);
        printInfo(`Using service name: ${serviceName}`);
        printInfo(`Using rewards address: ${rewardsAddress}`);

        // Get contract addresses
        const gatewayAddress = this.getContractAddress('AxelarGateway', chainName);
        const multisigAddress = this.getContractAddress('Multisig', chainName);

        // Get code IDs - these may need to be fetched from chain
        printInfo('Fetching code IDs for contracts...');
        const [gatewayCodeId, verifierCodeId, proverCodeId] = await Promise.all([
            this.getCodeIdFromConfig('Gateway', chainName, client, wallet),
            this.getCodeIdFromConfig('Verifier', chainName, client, wallet),
            this.getCodeIdFromConfig('Prover', chainName, client, wallet),
        ]);

        printInfo(`Code IDs - Gateway: ${gatewayCodeId}, Verifier: ${verifierCodeId}, Prover: ${proverCodeId}`);

        return {
            instantiate_chain_contracts: {
                chain: chainName,
                deployment_name: deploymentName,
                salt: salt,
                params: {
                    gateway: {
                        code_id: gatewayCodeId,
                        label: `Gateway-${chainName}`,
                    },
                    verifier: {
                        code_id: verifierCodeId,
                        label: `Verifier-${chainName}`,
                        msg: {
                            governance_address: governanceAddress,
                            service_name: serviceName,
                            source_gateway_address: gatewayAddress,
                            voting_threshold: {
                                numerator: parseInt(options.votingThreshold?.[0] || '51'),
                                denominator: parseInt(options.votingThreshold?.[1] || '100'),
                            },
                            block_expiry: options.blockExpiry,
                            confirmation_height: options.confirmationHeight,
                            source_chain: chainConfig.axelarId,
                            rewards_address: rewardsAddress,
                            msg_id_format: options.msgIdFormat,
                            address_format: options.addressFormat,
                        },
                    },
                    prover: {
                        code_id: proverCodeId,
                        label: `Prover-${chainName}`,
                        msg: {
                            governance_address: governanceAddress,
                            multisig_address: multisigAddress,
                            signing_threshold: options.signingThreshold,
                            service_name: serviceName,
                            chain_name: chainConfig.axelarId,
                            verifier_set_diff_threshold: options.verifierSetDiffThreshold,
                            encoder: options.encoder,
                            key_type: options.keyType,
                            domain_separator: options.domainSeparator,
                        },
                    },
                },
            },
        };
    }

    public async execute(chainName: string, options: CoordinatorOptions): Promise<void> {
        try {
            printInfo(`Preparing instantiate chain contracts proposal for chain: ${chainName}`);
            printInfo(`Environment: ${this.environment}`);

            // Validate options
            this.validateOptions(options);

            // Add governance proposal defaults
            const processedOptions = { ...options };
            if (!processedOptions.runAs) {
                processedOptions.runAs =
                    this.environment === 'devnet-amplifier'
                        ? 'axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9'
                        : CoordinatorScript.DEFAULTS.governanceAddress;
            }

            // Initialize contract config
            initContractConfig(this.fullConfig, { contractName: 'Coordinator', chainName });

            // Prepare wallet and client
            printInfo('Preparing wallet and client...');
            const wallet = await prepareWallet(processedOptions as { mnemonic: string });
            const client = await prepareClient(this.fullConfig as { axelar: { rpc: string; gasPrice: string } }, wallet);

            const message = await this.constructExecuteMessage(chainName, options, client, wallet);
            const messageJson = JSON.stringify(message, null, 2);

            printInfo('Generated execute message:', messageJson);

            // Create the proposal
            printInfo('Creating governance proposal...');
            const proposal = encodeExecuteContractProposal(
                this.fullConfig,
                {
                    ...processedOptions,
                    contractName: 'Coordinator',
                    msg: messageJson,
                },
                chainName,
            );

            // Show proposal details and confirm
            printInfo('Proposal details:', JSON.stringify(proposal, null, 2));

            if (!options.yes) {
                const proceed = prompt('Proceed with proposal submission?', false);
                if (!proceed) {
                    printInfo('Proposal submission cancelled');
                    return;
                }
            }

            // Submit the proposal
            printInfo('Submitting proposal...');
            const proposalId = await submitProposal(client, wallet, this.fullConfig, processedOptions, proposal);
            printInfo('Proposal submitted successfully', proposalId);

            // Save config if needed
            saveConfig(this.fullConfig, this.environment);
        } catch (error) {
            printError('Error executing coordinator script:', (error as Error).message);
            process.exit(1);
        }
    }
}

// CLI setup
const program = new Command();

program
    .name('coordinator')
    .description('Submit governance proposal to instantiate chain contracts using Coordinator')
    .requiredOption('-n, --chain <chain>', 'Chain name (e.g., ethereum-sepolia, celo)')
    .option('-e, --env <environment>', 'Environment (testnet, mainnet)', 'testnet')
    .option('-m, --mnemonic <mnemonic>', 'Mnemonic for signing')
    .option('-y, --yes', 'Skip confirmation prompts')
    .option('--salt <salt>', 'Custom salt for deployment (optional, will generate if not provided)')
    .option('--governance-address <address>', 'Governance address')
    .option('--service-name <name>', 'Service name')
    .option('--rewards-address <address>', 'Rewards address')
    .option('--voting-threshold <threshold>', 'Voting threshold (e.g., "51,100")')
    .option('--block-expiry <expiry>', 'Block expiry', '10')
    .option(
        '--confirmation-height <height>',
        'Confirmation height (default is overvalued for safety - double check with the team)',
        '1000000',
    )
    .option('--msg-id-format <format>', 'Message ID format', 'hex_tx_hash_and_event_index')
    .option('--address-format <format>', 'Address format', 'eip55')
    .option('--signing-threshold <threshold>', 'Signing threshold (e.g., "51,100")')
    .option('--verifier-set-diff-threshold <threshold>', 'Verifier set diff threshold', '1')
    .option('--encoder <encoder>', 'Encoder type', 'abi')
    .option('--key-type <type>', 'Key type', 'ecdsa')
    .option('--domain-separator <separator>', 'Domain separator')
    .action(async (options) => {
        try {
            const coordinator = new CoordinatorScript(options.env);

            // Parse threshold arrays
            const votingThreshold = options.votingThreshold
                ? options.votingThreshold.split(',').map((s) => s.trim())
                : CoordinatorScript.DEFAULTS.votingThreshold;
            const signingThreshold = options.signingThreshold
                ? options.signingThreshold.split(',').map((s) => s.trim())
                : CoordinatorScript.DEFAULTS.signingThreshold;

            await coordinator.execute(options.chain, {
                ...options,
                votingThreshold,
                signingThreshold,
                confirmationHeight: parseInt(options.confirmationHeight),
                verifierSetDiffThreshold: parseInt(options.verifierSetDiffThreshold),
            });
        } catch (error) {
            printError('Error:', (error as Error).message);
            process.exit(1);
        }
    });

if (require.main === module) {
    program.parse();
}
