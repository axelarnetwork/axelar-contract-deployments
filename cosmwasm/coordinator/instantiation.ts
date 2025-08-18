import type { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import type { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';

import { printInfo, prompt } from '../../common';
import { encodeExecuteContractProposal, fetchCodeIdFromCodeHash, getSalt, prepareClient, prepareWallet, submitProposal } from '../utils';
import { ConfigManager } from './config';
import { RetryManager } from './retry';
import type {
    GatewayChainConfig,
    InstantiateChainContractsMsg,
    InstantiateChainOptions,
    MultisigProverChainConfig,
    VotingVerifierChainConfig,
    WalletAndClient,
} from './types';
import { AMPLIFIER_CONTRACTS_TO_HANDLE } from './types';

export class InstantiationManager {
    public configManager: ConfigManager;

    constructor(configManager: ConfigManager) {
        this.configManager = configManager;
    }

    public async instantiateChainContracts(options: InstantiateChainOptions): Promise<void> {
        printInfo(`Instantiating chain contracts for ${options.chainName}...`);
        printInfo(`Environment: ${this.configManager.getEnvironment()}`);

        const { wallet, client } = await this.prepareWalletAndClient(options);

        if (prompt('Are the deployment proposals executed?', options.yes)) {
            printInfo('Deployment proposals are not finished yet, please wait for them to be executed');
            return;
        }

        await this.fetchAndUpdateCodeIds(client, AMPLIFIER_CONTRACTS_TO_HANDLE);
        await this.executeMessageViaGovernance(options.chainName, options, client, wallet);
    }

    private constructExecuteMessage(chainName: string, deploymentName: string): InstantiateChainContractsMsg {
        const chainConfig = this.configManager.getChainConfig(chainName);
        const votingVerifierConfig = (this.configManager.getContractConfig('VotingVerifier')[chainName] as VotingVerifierChainConfig) || {};
        const multisigProverConfig = (this.configManager.getContractConfig('MultisigProver')[chainName] as MultisigProverChainConfig) || {};
        const gatewayConfig = (this.configManager.getContractConfig('Gateway')[chainName] as GatewayChainConfig) || {};
        const axelarContracts = this.configManager.getFullConfig().axelar?.contracts;
        if (!axelarContracts) {
            throw new Error('Axelar contracts section not found in config');
        }

        const validateRequired = <T>(value: T | undefined | null, configPath: string): T => {
            if (value === undefined || value === null || (Array.isArray(value) && value.length === 0)) {
                throw new Error(`Missing required configuration for chain ${chainName}. Please configure it in ${configPath}.`);
            }
            return value;
        };

        const validateThreshold = (value: [string, string] | undefined | null, configPath: string): [string, string] => {
            if (!value || !Array.isArray(value) || value.length !== 2) {
                throw new Error(
                    `Missing or invalid threshold configuration for chain ${chainName}. Please configure it in ${configPath} as [numerator, denominator].`,
                );
            }
            return value;
        };

        const governanceAddress = validateRequired(
            votingVerifierConfig.governanceAddress,
            `VotingVerifier[${chainName}].governanceAddress`,
        );
        const serviceName = validateRequired(votingVerifierConfig.serviceName, `VotingVerifier[${chainName}].serviceName`);
        const rewardsAddress = validateRequired(votingVerifierConfig.rewardsAddress, `VotingVerifier[${chainName}].rewardsAddress`);
        const sourceGatewayAddress = validateRequired(
            votingVerifierConfig.sourceGatewayAddress,
            `VotingVerifier[${chainName}].sourceGatewayAddress`,
        );
        const votingThreshold = validateThreshold(votingVerifierConfig.votingThreshold, `VotingVerifier[${chainName}].votingThreshold`);
        const blockExpiry = validateRequired(votingVerifierConfig.blockExpiry, `VotingVerifier[${chainName}].blockExpiry`);
        const confirmationHeight = validateRequired(
            votingVerifierConfig.confirmationHeight,
            `VotingVerifier[${chainName}].confirmationHeight`,
        );
        const msgIdFormat = validateRequired(votingVerifierConfig.msgIdFormat, `VotingVerifier[${chainName}].msgIdFormat`);
        const addressFormat = validateRequired(votingVerifierConfig.addressFormat, `VotingVerifier[${chainName}].addressFormat`);
        const encoder = validateRequired(multisigProverConfig.encoder, `MultisigProver[${chainName}].encoder`);
        const keyType = validateRequired(multisigProverConfig.keyType, `MultisigProver[${chainName}].keyType`);
        const domainSeparator = validateRequired(multisigProverConfig.domainSeparator, `MultisigProver[${chainName}].domainSeparator`);
        const verifierContractAdminAddress = validateRequired(
            votingVerifierConfig.contractAdmin,
            `VotingVerifier[${chainName}].contractAdmin`,
        );
        const multisigContractAdminAddress = validateRequired(
            multisigProverConfig.contractAdmin,
            `MultisigProver[${chainName}].contractAdmin`,
        );
        const gatewayContractAdminAddress = validateRequired(gatewayConfig.contractAdmin, `Gateway[${chainName}].contractAdmin`);
        const multisigAdminAddress = validateRequired(multisigProverConfig.adminAddress, `MultisigProver[${chainName}].adminAddress`);
        const multisigAddress = validateRequired(multisigProverConfig.multisigAddress, `MultisigProver[${chainName}].multisigAddress`);
        const verifierSetDiffThreshold = validateRequired(
            multisigProverConfig.verifierSetDiffThreshold,
            `MultisigProver[${chainName}].verifierSetDiffThreshold`,
        );
        const signingThreshold = validateThreshold(multisigProverConfig.signingThreshold, `MultisigProver[${chainName}].signingThreshold`);
        const salt = validateRequired(gatewayConfig.salt, 'Salt');
        const saltUint8Array = getSalt(salt, chainName, chainConfig.axelarId);
        const gatewayCodeId: number = this.configManager.getContractConfig('Gateway').codeId;
        const verifierCodeId: number = this.configManager.getContractConfig('VotingVerifier').codeId;
        const proverCodeId: number = this.configManager.getContractConfig('MultisigProver').codeId;

        printInfo(`Code IDs - Gateway: ${gatewayCodeId}, Verifier: ${verifierCodeId}, Prover: ${proverCodeId}`);

        return {
            instantiate_chain_contracts: {
                deployment_name: deploymentName,
                salt: Buffer.from(saltUint8Array).toString('base64'),
                params: {
                    manual: {
                        gateway: {
                            code_id: gatewayCodeId,
                            label: `Gateway-${chainName}`,
                            msg: null,
                            contract_admin: gatewayContractAdminAddress,
                        },
                        verifier: {
                            code_id: verifierCodeId,
                            label: `Verifier-${chainName}`,
                            msg: {
                                governance_address: governanceAddress,
                                service_name: serviceName,
                                source_gateway_address: sourceGatewayAddress,
                                voting_threshold: [votingThreshold[0], votingThreshold[1]],
                                block_expiry: String(blockExpiry),
                                confirmation_height: confirmationHeight,
                                source_chain: chainConfig.axelarId,
                                rewards_address: rewardsAddress,
                                msg_id_format: msgIdFormat,
                                address_format: addressFormat,
                            },
                            contract_admin: verifierContractAdminAddress,
                        },
                        prover: {
                            code_id: proverCodeId,
                            label: `Prover-${chainName}`,
                            msg: {
                                governance_address: governanceAddress,
                                admin_address: multisigAdminAddress,
                                multisig_address: multisigAddress,
                                signing_threshold: [signingThreshold[0], signingThreshold[1]],
                                service_name: serviceName,
                                chain_name: chainConfig.axelarId,
                                verifier_set_diff_threshold: verifierSetDiffThreshold,
                                encoder: encoder,
                                key_type: keyType,
                                domain_separator: domainSeparator,
                            },
                            contract_admin: multisigContractAdminAddress,
                        },
                    },
                },
            },
        };
    }

    private async executeMessageViaGovernance(
        chainName: string,
        options: InstantiateChainOptions,
        client: SigningCosmWasmClient,
        wallet: DirectSecp256k1HdWallet,
    ): Promise<void> {
        const deploymentName = this.generateDeploymentName(chainName);
        const message = this.constructExecuteMessage(chainName, deploymentName);
        const messageJson = JSON.stringify(message, null, 2);

        printInfo(`Message: ${messageJson}`);
        printInfo(`Deployment name: ${deploymentName}`);

        const title = `Instantiate Chain Contracts for ${chainName}`;
        const description = `Instantiate Gateway, VotingVerifier, and MultisigProver contracts for chain ${chainName}`;
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

        const proposalId = await RetryManager.withRetry(() =>
            submitProposal(client, wallet, this.configManager.getFullConfig(), options, proposal),
        );

        this.storeDeploymentInfo(chainName, deploymentName, proposalId);
        this.configManager.saveConfig();

        printInfo(`Chain contracts instantiation for ${chainName} completed successfully!`);
        printInfo(`Deployment name: ${deploymentName}`);
        printInfo(`Proposal ID: ${proposalId}`);
    }

    private generateDeploymentName(chainName: string): string {
        return `deployment-${chainName}-${Date.now()}`;
    }

    private storeDeploymentInfo(chainName: string, deploymentName?: string, proposalId?: string): void {
        if (deploymentName) {
            const gatewayConfig = this.configManager.getContractConfig('Gateway');
            const verifierConfig = this.configManager.getContractConfig('VotingVerifier');
            const proverConfig = this.configManager.getContractConfig('MultisigProver');

            if (gatewayConfig[chainName]) {
                (gatewayConfig[chainName] as GatewayChainConfig).deploymentName = deploymentName;
            }
            if (verifierConfig[chainName]) {
                (verifierConfig[chainName] as VotingVerifierChainConfig).deploymentName = deploymentName;
            }
            if (proverConfig[chainName]) {
                (proverConfig[chainName] as MultisigProverChainConfig).deploymentName = deploymentName;
            }
        }

        if (proposalId) {
            const verifierConfig = this.configManager.getContractConfig('VotingVerifier');
            const proverConfig = this.configManager.getContractConfig('MultisigProver');

            if (verifierConfig[chainName]) {
                (verifierConfig[chainName] as VotingVerifierChainConfig).proposalId = proposalId;
            }
            if (proverConfig[chainName]) {
                (proverConfig[chainName] as MultisigProverChainConfig).proposalId = proposalId;
            }
        }

        this.configManager.saveConfig();
    }

    private async fetchAndUpdateCodeIds(client: SigningCosmWasmClient, contractsToUpdate: string[]): Promise<void> {
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

    private async prepareWalletAndClient(options: InstantiateChainOptions): Promise<WalletAndClient> {
        const wallet = await prepareWallet(options as { mnemonic: string });
        const client = await prepareClient(this.configManager.getFullConfig() as { axelar: { rpc: string; gasPrice: string } }, wallet);
        return { wallet, client };
    }
}
