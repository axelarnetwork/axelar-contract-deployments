import type { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import type { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import * as crypto from 'crypto';

import { printError, printInfo, printWarn, prompt } from '../../common';
import { encodeExecuteContractProposal, prepareClient, prepareDummyWallet, prepareWallet, submitProposal } from '../utils';
import { CodeIdUtils } from './code-id-utils';
import { ConfigManager } from './config';
import { CONTRACTS_TO_HANDLE, DEFAULTS } from './constants';
import { RetryManager } from './retry';
import type { ContractInfo, CoordinatorOptions, InstantiateChainContractsMsg, WalletAndClient } from './types';

export class InstantiationManager {
    public configManager: ConfigManager;
    public codeIdUtils: CodeIdUtils;

    constructor(configManager: ConfigManager) {
        this.configManager = configManager;
        this.codeIdUtils = new CodeIdUtils(configManager);
    }

    public async instantiateChainContracts(chainName: string, options: CoordinatorOptions): Promise<void> {
        try {
            printInfo(`Instantiating chain contracts for ${chainName}...`);
            printInfo(`Environment: ${this.configManager.getEnvironment()}`);

            const { wallet, client } = await this.prepareWalletAndClient(options);

            await this.codeIdUtils.fetchAndUpdateCodeIdsFromProposals(client, CONTRACTS_TO_HANDLE);

            let deploymentName: string;
            let proposalId: string | undefined;

            if (options.direct) {
                const result = await this.executeMessageDirect(chainName, options, client, wallet);
                deploymentName = result.deploymentName;
            } else {
                const result = await this.executeMessageViaGovernance(chainName, options, client, wallet);
                deploymentName = result.deploymentName;
                proposalId = result.proposalId;
            }

            printInfo(`Chain contracts instantiation for ${chainName} completed successfully!`);
            printInfo(`Deployment name: ${deploymentName}`);
            if (proposalId) {
                printInfo(`Proposal ID: ${proposalId}`);
            }
        } catch (error) {
            printError('Error in InstantiationManager:', (error as Error).message);
            throw error;
        }
    }

    private constructExecuteMessage(chainName: string, options: CoordinatorOptions, deploymentName: string): InstantiateChainContractsMsg {
        printInfo(`Constructing execute message for chain: ${chainName}`);

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

        // Fix: Add error handling for Rewards contract address lookup
        let rewardsAddress: string | null = null;
        if (options.rewardsAddress) {
            rewardsAddress = options.rewardsAddress;
        } else {
            try {
                rewardsAddress = this.configManager.getContractAddressFromConfig('Rewards');
                printInfo(`Using rewards address from config: ${rewardsAddress}`);
            } catch (error) {
                printWarn(`Could not get rewards address from config: ${(error as Error).message}`);
                rewardsAddress = null;
            }
        }

        const multisigAddress = this.configManager.getContractAddressFromConfig('Multisig');
        const sourceGatewayAddress = options.sourceGatewayAddress || '';

        printInfo(`Using governance address: ${governanceAddress}`);
        printInfo(`Using service name: ${serviceName}`);
        printInfo(`Using rewards address: ${rewardsAddress || 'not configured'}`);
        printInfo(`Using source gateway address: ${sourceGatewayAddress}`);

        const gatewayCodeId: number = this.configManager.getContractConfig('Gateway').codeId;
        const verifierCodeId: number = this.configManager.getContractConfig('VotingVerifier').codeId;
        const proverCodeId: number = this.configManager.getContractConfig('MultisigProver').codeId;

        printInfo(`Code IDs - Gateway: ${gatewayCodeId}, Verifier: ${verifierCodeId}, Prover: ${proverCodeId}`);

        const msgIdFormat = options.msgIdFormat || DEFAULTS.msgIdFormat;
        const addressFormat = options.addressFormat || DEFAULTS.addressFormat;
        const encoder = options.encoder || DEFAULTS.encoder;
        const keyType = options.keyType || DEFAULTS.keyType;
        const domainSeparator = (options.domainSeparator || DEFAULTS.domainSeparator).replace('0x', '');

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
                        },
                        prover: {
                            code_id: proverCodeId,
                            label: `Prover-${chainName}`,
                            msg: {
                                governance_address: governanceAddress,
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
                        },
                    },
                },
            },
        };
    }

    private async executeMessageDirect(
        chainName: string,
        options: CoordinatorOptions,
        client: SigningCosmWasmClient,
        wallet: DirectSecp256k1HdWallet,
    ): Promise<{ deploymentName: string }> {
        printInfo('Executing message directly (no governance proposal)...');

        const deploymentName = this.generateDeploymentName(chainName);
        const message = this.constructExecuteMessage(chainName, options, deploymentName);
        const messageJson = JSON.stringify(message, null, 2);

        printInfo(`Generated execute message (length: ${messageJson.length})`);

        if (prompt('Proceed with direct execution?', options.yes)) {
            printInfo('Direct execution cancelled');
            throw new Error('Direct execution cancelled');
        }

        const accounts = await wallet.getAccounts();
        const account = accounts[0];
        const coordinatorAddress = this.configManager.getContractAddressFromConfig('Coordinator');

        printInfo('Executing message on coordinator contract...');

        const { transactionHash, events } = await RetryManager.withRetry(() =>
            client.execute(account.address, coordinatorAddress, message, 'auto', ''),
        );

        printInfo('Message executed successfully!');
        printInfo(`Transaction hash: ${transactionHash}`);

        const contractInfo = this.extractContractInfoFromEvents(events);

        this.storeChainSpecificParams(chainName, options, contractInfo, deploymentName, undefined);
        this.configManager.saveConfig();

        return { deploymentName };
    }

    private async executeMessageViaGovernance(
        chainName: string,
        options: CoordinatorOptions,
        client: SigningCosmWasmClient,
        wallet: DirectSecp256k1HdWallet,
    ): Promise<{ deploymentName: string; proposalId: string }> {
        printInfo('Executing message via governance proposal...');

        const deploymentName = this.generateDeploymentName(chainName);
        const message = this.constructExecuteMessage(chainName, options, deploymentName);
        const messageJson = JSON.stringify(message, null, 2);

        printInfo(`Deployment name: ${deploymentName}`);

        const title = options.title || `Instantiate Chain Contracts for ${chainName}`;
        const description =
            options.description || `Instantiate Gateway, VotingVerifier, and MultisigProver contracts for chain ${chainName}`;

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
            throw new Error('Proposal submission cancelled');
        }

        printInfo('Submitting proposal...');

        const proposalId = await RetryManager.withRetry(() =>
            submitProposal(client, wallet, this.configManager.getFullConfig(), options, proposal),
        );
        printInfo(`Proposal submitted successfully with ID: ${proposalId}`);

        this.storeChainSpecificParams(chainName, options, undefined, deploymentName, proposalId);
        this.configManager.saveConfig();

        return { deploymentName, proposalId };
    }

    private extractContractInfoFromEvents(events: readonly unknown[]): ContractInfo {
        const contractInfo: ContractInfo = {};

        const instantiateEvents = events.filter((e) => (e as { type: string }).type === 'wasm-instantiate');

        for (const event of instantiateEvents) {
            const eventObj = event as { attributes?: Array<{ key: string; value: string }> };
            if (eventObj.attributes) {
                const contractAddrAttr = eventObj.attributes.find((a: { key: string; value: string }) => a.key === '_contract_address');
                const codeIdAttr = eventObj.attributes.find((a: { key: string; value: string }) => a.key === 'code_id');
                const labelAttr = eventObj.attributes.find((a: { key: string; value: string }) => a.key === 'label');

                if (contractAddrAttr && codeIdAttr && labelAttr) {
                    const address = contractAddrAttr.value;
                    const codeId = parseInt(codeIdAttr.value);
                    const label = labelAttr.value;

                    if (label.includes('Gateway')) {
                        contractInfo.gateway = { address, codeId };
                        printInfo(`Found Gateway - Address: ${address}, Code ID: ${codeId}`);
                    } else if (label.includes('Verifier')) {
                        contractInfo.verifier = { address, codeId };
                        printInfo(`Found VotingVerifier - Address: ${address}, Code ID: ${codeId}`);
                    } else if (label.includes('Prover')) {
                        contractInfo.prover = { address, codeId };
                        printInfo(`Found MultisigProver - Address: ${address}, Code ID: ${codeId}`);
                    }
                }
            }
        }

        return contractInfo;
    }

    private storeChainSpecificParams(
        chainName: string,
        options: CoordinatorOptions,
        contractInfo?: ContractInfo,
        deploymentName?: string,
        proposalId?: string,
    ): void {
        printInfo(`Storing chain-specific parameters for ${chainName}...`);

        const chainConfig = this.configManager.getChainConfig(chainName);
        const governanceAddress = options.governanceAddress || this.configManager.getDefaultGovernanceAddress();
        const serviceName = options.serviceName || DEFAULTS.serviceName;

        let rewardsAddress: string | null = null;
        if (options.rewardsAddress) {
            rewardsAddress = options.rewardsAddress;
        } else {
            try {
                rewardsAddress = this.configManager.getContractAddressFromConfig('Rewards');
            } catch (error) {
                printWarn(`Could not get rewards address from config: ${(error as Error).message}`);
                rewardsAddress = null;
            }
        }

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
            address: contractInfo?.verifier?.address,
            codeId: contractInfo?.verifier?.codeId,
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
            address: contractInfo?.prover?.address,
            codeId: contractInfo?.prover?.codeId,
            deploymentName,
            proposalId,
        };

        const gatewayParams = {
            address: contractInfo?.gateway?.address,
            codeId: contractInfo?.gateway?.codeId,
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

    private async prepareWalletAndClient(options: CoordinatorOptions): Promise<WalletAndClient> {
        const wallet = await prepareWallet(options as { mnemonic: string });
        const client = await prepareClient(this.configManager.getFullConfig() as { axelar: { rpc: string; gasPrice: string } }, wallet);
        return { wallet, client };
    }
}
