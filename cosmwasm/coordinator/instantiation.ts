import type { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import type { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import * as crypto from 'crypto';

import { printInfo, prompt } from '../../common';
import { encodeExecuteContractProposal, fetchCodeIdFromCodeHash, prepareClient, prepareWallet, submitProposal } from '../utils';
import { ConfigManager } from './config';
import { CONTRACTS_TO_HANDLE, DEFAULTS } from './constants';
import { RetryManager } from './retry';
import type { CoordinatorOptions, InstantiateChainContractsMsg, WalletAndClient } from './types';

export class InstantiationManager {
    public configManager: ConfigManager;

    constructor(configManager: ConfigManager) {
        this.configManager = configManager;
    }

    public async instantiateChainContracts(chainName: string, options: CoordinatorOptions): Promise<void> {
        printInfo(`Instantiating chain contracts for ${chainName}...`);
        printInfo(`Environment: ${this.configManager.getEnvironment()}`);

        const { wallet, client } = await this.prepareWalletAndClient(options);

        if (prompt('Are the deployment proposals executed?', options.yes)) {
            printInfo('Deployment proposals are not finished yet, please wait for them to be executed');
            return;
        }

        await this.fetchAndUpdateCodeIds(client, CONTRACTS_TO_HANDLE);
        await this.executeMessageViaGovernance(chainName, options, client, wallet);
    }

    private constructExecuteMessage(chainName: string, options: CoordinatorOptions, deploymentName: string): InstantiateChainContractsMsg {
        const chainConfig = this.configManager.getChainConfig(chainName);

        let salt: string;

        if (options.salt) {
            salt = options.salt;
            printInfo(`Using provided salt: ${salt}`);
        } else {
            salt = this.generateSalt();
            printInfo(`Generated salt: ${salt}`);
        }

        const saltBase64 = Buffer.from(salt, 'hex').toString('base64');

        const governanceAddress = options.governanceAddress || null;
        const serviceName = options.serviceName || DEFAULTS.serviceName;
        const rewardsAddress = options.rewardsAddress || this.configManager.getContractAddressFromConfig('Rewards');
        const multisigAddress = this.configManager.getContractAddressFromConfig('Multisig');
        const sourceGatewayAddress =
            options.sourceGatewayAddress || this.configManager.getContractAddressFromChainConfig(chainName, 'AxelarGateway');

        printInfo(`Governance address: ${governanceAddress}`);
        printInfo(`Service name: ${serviceName}`);
        printInfo(`Rewards address: ${rewardsAddress}`);
        printInfo(`Source gateway address: ${sourceGatewayAddress}`);

        const gatewayCodeId: number = this.configManager.getContractConfig('Gateway').codeId;
        const verifierCodeId: number = this.configManager.getContractConfig('VotingVerifier').codeId;
        const proverCodeId: number = this.configManager.getContractConfig('MultisigProver').codeId;

        printInfo(`Code IDs - Gateway: ${gatewayCodeId}, Verifier: ${verifierCodeId}, Prover: ${proverCodeId}`);

        const msgIdFormat = options.msgIdFormat || DEFAULTS.msgIdFormat;
        const addressFormat = options.addressFormat || DEFAULTS.addressFormat;
        const encoder = options.encoder || DEFAULTS.encoder;
        const keyType = options.keyType || DEFAULTS.keyType;
        const domainSeparator = (options.domainSeparator || DEFAULTS.domainSeparator).replace('0x', '');
        const contractAdminAddress = options.contractAdmin;
        const multisigAdminAddress = options.multisigAdmin;

        return {
            instantiate_chain_contracts: {
                deployment_name: deploymentName,
                salt: saltBase64,
                params: {
                    manual: {
                        gateway: {
                            code_id: gatewayCodeId,
                            label: `Gateway-${chainName}`,
                            msg: null,
                            contract_admin: contractAdminAddress,
                        },
                        verifier: {
                            code_id: verifierCodeId,
                            label: `Verifier-${chainName}`,
                            msg: {
                                governance_address: governanceAddress,
                                service_name: serviceName,
                                source_gateway_address: sourceGatewayAddress,
                                voting_threshold: [
                                    options.votingThreshold?.[0] || DEFAULTS.votingThreshold[0],
                                    options.votingThreshold?.[1] || DEFAULTS.votingThreshold[1],
                                ],
                                block_expiry: (options.blockExpiry || DEFAULTS.blockExpiry).toString(),
                                confirmation_height:
                                    typeof options.confirmationHeight === 'number'
                                        ? options.confirmationHeight
                                        : options.confirmationHeight
                                          ? parseInt(options.confirmationHeight.toString())
                                          : DEFAULTS.confirmationHeight,
                                source_chain: chainConfig.axelarId,
                                rewards_address: rewardsAddress,
                                msg_id_format: msgIdFormat,
                                address_format: addressFormat,
                            },
                            contract_admin: contractAdminAddress,
                        },
                        prover: {
                            code_id: proverCodeId,
                            label: `Prover-${chainName}`,
                            msg: {
                                governance_address: governanceAddress,
                                admin_address: multisigAdminAddress,
                                multisig_address: multisigAddress,
                                signing_threshold: [
                                    options.signingThreshold?.[0] || DEFAULTS.signingThreshold[0],
                                    options.signingThreshold?.[1] || DEFAULTS.signingThreshold[1],
                                ],
                                service_name: serviceName,
                                chain_name: chainConfig.axelarId,
                                verifier_set_diff_threshold:
                                    typeof options.verifierSetDiffThreshold === 'number'
                                        ? options.verifierSetDiffThreshold
                                        : options.verifierSetDiffThreshold
                                          ? parseInt(options.verifierSetDiffThreshold.toString())
                                          : DEFAULTS.verifierSetDiffThreshold,
                                encoder: encoder,
                                key_type: keyType,
                                domain_separator: domainSeparator,
                            },
                            contract_admin: contractAdminAddress,
                        },
                    },
                },
            },
        };
    }

    private async executeMessageViaGovernance(
        chainName: string,
        options: CoordinatorOptions,
        client: SigningCosmWasmClient,
        wallet: DirectSecp256k1HdWallet,
    ): Promise<void> {
        printInfo('Executing message via governance proposal...');

        const deploymentName = this.generateDeploymentName(chainName);
        const message = this.constructExecuteMessage(chainName, options, deploymentName);
        const messageJson = JSON.stringify(message, null, 2);

        printInfo(`Message: ${messageJson}`);
        printInfo(`Deployment name: ${deploymentName}`);

        const title = `Instantiate Chain Contracts for ${chainName}`;
        const description = `Instantiate Gateway, VotingVerifier, and MultisigProver contracts for chain ${chainName}`;

        printInfo('Creating governance proposal...');
        const proposal = encodeExecuteContractProposal(
            this.configManager.getFullConfig(),
            {
                ...options,
                contractName: 'Coordinator',
                msg: messageJson,
                title,
                description,
            },
            chainName,
        );

        if (prompt('Proceed with proposal submission?', options.yes)) {
            printInfo('Proposal submission cancelled');
            return;
        }

        printInfo('Submitting proposal...');

        const proposalId = await RetryManager.withRetry(() =>
            submitProposal(client, wallet, this.configManager.getFullConfig(), options, proposal),
        );
        printInfo(`Proposal submitted successfully with ID: ${proposalId}`);

        this.storeChainSpecificParams(chainName, options, deploymentName, proposalId);
        this.configManager.saveConfig();

        printInfo(`Chain contracts instantiation for ${chainName} completed successfully!`);
        printInfo(`Deployment name: ${deploymentName}`);
        printInfo(`Proposal ID: ${proposalId}`);
    }

    private storeChainSpecificParams(chainName: string, options: CoordinatorOptions, deploymentName?: string, proposalId?: string): void {
        printInfo(`Storing chain-specific parameters for ${chainName}...`);

        const chainConfig = this.configManager.getChainConfig(chainName);
        const governanceAddress = options.governanceAddress || this.configManager.getDefaultGovernanceAddress();
        const serviceName = options.serviceName || DEFAULTS.serviceName;
        const rewardsAddress = options.rewardsAddress || this.configManager.getContractAddressFromConfig('Rewards');

        const sourceGatewayAddress = options.sourceGatewayAddress || '';

        const votingVerifierParams = {
            governanceAddress,
            serviceName,
            sourceGatewayAddress,
            votingThreshold: [
                options.votingThreshold?.[0] || DEFAULTS.votingThreshold[0],
                options.votingThreshold?.[1] || DEFAULTS.votingThreshold[1],
            ],
            blockExpiry: options.blockExpiry || DEFAULTS.blockExpiry,
            confirmationHeight:
                typeof options.confirmationHeight === 'number'
                    ? options.confirmationHeight
                    : options.confirmationHeight
                      ? parseInt(options.confirmationHeight.toString())
                      : DEFAULTS.confirmationHeight,
            sourceChain: chainConfig.axelarId,
            rewardsAddress,
            msgIdFormat: options.msgIdFormat || DEFAULTS.msgIdFormat,
            addressFormat: options.addressFormat || DEFAULTS.addressFormat,
            deploymentName,
            proposalId,
        };

        const multisigProverParams = {
            governanceAddress,
            multisigAddress: this.configManager.getContractAddressFromConfig('Multisig'),
            signingThreshold: [
                options.signingThreshold?.[0] || DEFAULTS.signingThreshold[0],
                options.signingThreshold?.[1] || DEFAULTS.signingThreshold[1],
            ],
            serviceName,
            chainName: chainConfig.axelarId,
            verifierSetDiffThreshold:
                typeof options.verifierSetDiffThreshold === 'number'
                    ? options.verifierSetDiffThreshold
                    : options.verifierSetDiffThreshold
                      ? parseInt(options.verifierSetDiffThreshold.toString())
                      : DEFAULTS.verifierSetDiffThreshold,
            encoder: options.encoder || DEFAULTS.encoder,
            keyType: options.keyType || DEFAULTS.keyType,
            domainSeparator: (options.domainSeparator || DEFAULTS.domainSeparator).replace('0x', ''),
            deploymentName,
            proposalId,
        };

        const gatewayParams = {
            deploymentName,
            proposalId,
        };

        const axelarContracts = this.configManager.getFullConfig().axelar?.contracts;
        if (!axelarContracts) {
            throw new Error('Axelar contracts section not found in config');
        }

        if (!axelarContracts.VotingVerifier) {
            axelarContracts.VotingVerifier = {};
        }
        if (!axelarContracts.MultisigProver) {
            axelarContracts.MultisigProver = {};
        }
        if (!axelarContracts.Gateway) {
            axelarContracts.Gateway = {};
        }

        (axelarContracts.VotingVerifier as Record<string, unknown>)[chainName] = votingVerifierParams;
        (axelarContracts.MultisigProver as Record<string, unknown>)[chainName] = multisigProverParams;
        (axelarContracts.Gateway as Record<string, unknown>)[chainName] = gatewayParams;

        printInfo('Chain-specific parameters stored successfully');
    }

    private generateDeploymentName(chainName: string): string {
        return `deployment-${chainName}-${Date.now()}`;
    }

    private generateSalt(): string {
        return crypto.randomBytes(32).toString('hex');
    }

    /**
     * Fetches and updates code IDs from proposals for all contracts that need them
     */
    public async fetchAndUpdateCodeIds(client: SigningCosmWasmClient, contractsToUpdate: string[]): Promise<void> {
        printInfo('Fetching and updating code IDs from proposals...');

        for (const contractName of contractsToUpdate) {
            const contractConfig = this.configManager.getContractConfig(contractName);

            if (contractConfig.storeCodeProposalId && contractConfig.storeCodeProposalCodeHash) {
                printInfo(`Found proposal data for ${contractName}, fetching latest code ID from chain...`);

                const contractBaseConfig = {
                    storeCodeProposalCodeHash: contractConfig.storeCodeProposalCodeHash,
                };

                try {
                    const codeId = await fetchCodeIdFromCodeHash(client, contractBaseConfig);
                    printInfo(`Successfully fetched code ID ${codeId} for ${contractName} from chain`);

                    this.configManager.updateContractCodeId(contractName, codeId);
                    printInfo(`Updated ${contractName} code ID in config: ${codeId}`);
                } catch (error) {
                    printInfo(`Failed to fetch code ID for ${contractName} from chain: ${(error as Error).message}`);
                    if (contractConfig.codeId) {
                        printInfo(`Using existing code ID from config as fallback: ${contractConfig.codeId}`);
                        this.configManager.updateContractCodeId(contractName, contractConfig.codeId);
                    }
                }
            }
        }
    }

    private async prepareWalletAndClient(options: CoordinatorOptions): Promise<WalletAndClient> {
        const wallet = await prepareWallet(options as { mnemonic: string });
        const client = await prepareClient(this.configManager.getFullConfig() as { axelar: { rpc: string; gasPrice: string } }, wallet);
        return { wallet, client };
    }
}
