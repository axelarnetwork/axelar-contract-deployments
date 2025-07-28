import { AccessConfigUpdate, UpdateInstantiateConfigProposal } from 'cosmjs-types/cosmwasm/wasm/v1/proposal';
import { AccessType } from 'cosmjs-types/cosmwasm/wasm/v1/types';

import { printError, printInfo, prompt } from '../../common';
import { encodeExecuteContractProposal, initContractConfig, prepareClient, prepareWallet, submitProposal } from '../utils';
import { CodeIdUtils } from './code-id-utils';
import { ConfigManager } from './config';
import { CONTRACTS_TO_HANDLE } from './constants';
import { RetryManager } from './retry';
import type { CoordinatorOptions, RegisterDeploymentMsg, WalletAndClient } from './types';

export class GovernanceManager {
    public configManager: ConfigManager;
    public codeIdUtils: CodeIdUtils;

    constructor(configManager: ConfigManager) {
        this.configManager = configManager;
        this.codeIdUtils = new CodeIdUtils(configManager);
    }

    public async registerProtocol(options: CoordinatorOptions): Promise<void> {
        try {
            printInfo('Preparing register protocol proposal...');
            printInfo(`Environment: ${this.configManager.getEnvironment()}`);

            const processedOptions = this.configManager.processOptions(options);

            initContractConfig(this.configManager.getFullConfig(), { contractName: 'Coordinator', chainName: undefined });

            const { wallet, client } = await this.prepareWalletAndClient(processedOptions);

            const serviceRegistryAddress = this.configManager.getContractAddressFromConfig('ServiceRegistry');
            const routerAddress = this.configManager.getContractAddressFromConfig('Router');
            const multisigAddress = this.configManager.getContractAddressFromConfig('Multisig');

            printInfo(`Using service registry address: ${serviceRegistryAddress}`);
            printInfo(`Using router address: ${routerAddress}`);
            printInfo(`Using multisig address: ${multisigAddress}`);

            const message = {
                register_protocol: {
                    service_registry_address: serviceRegistryAddress,
                    router_address: routerAddress,
                    multisig_address: multisigAddress,
                },
            };
            const messageJson = JSON.stringify(message, null, 2);

            printInfo('Generated register protocol message:', messageJson);

            const title = options.title || 'Register Protocol Contracts';
            const description = options.description || 'Register ServiceRegistry, Router, and Multisig contracts with Coordinator';

            printInfo('Creating governance proposal...');
            const proposal = encodeExecuteContractProposal(
                this.configManager.getFullConfig(),
                {
                    ...processedOptions,
                    contractName: 'Coordinator',
                    msg: messageJson,
                    title,
                    description,
                },
                undefined,
            );

            if (prompt('Proceed with register protocol proposal submission?', options.yes)) {
                printInfo('Register protocol proposal submission cancelled');
                return;
            }

            printInfo('Submitting register protocol proposal...');
            const proposalId = await RetryManager.withRetry(() =>
                submitProposal(client, wallet, this.configManager.getFullConfig(), processedOptions, proposal),
            );
            printInfo('Register protocol proposal submitted successfully', proposalId);

            this.configManager.saveConfig();
        } catch (error) {
            printError('Error in GovernanceManager:', (error as Error).message);
            throw error;
        }
    }

    public async registerDeployment(options: CoordinatorOptions, chainName: string): Promise<void> {
        try {
            printInfo('Preparing register deployment proposal...');
            printInfo(`Environment: ${this.configManager.getEnvironment()}`);
            printInfo(`Chain: ${chainName}`);

            const processedOptions = this.configManager.processOptions(options);
            const { wallet, client } = await this.prepareWalletAndClient(processedOptions);
            const deploymentName = this.configManager.getDeploymentNameFromConfig(chainName);

            printInfo(`Using deployment name from config: ${deploymentName}`);

            const instantiationProposalId = this.configManager.getInstantiationProposalIdFromConfig(chainName);
            if (instantiationProposalId) {
                printInfo(`Found instantiation proposal ID in config: ${instantiationProposalId}`);
            } else {
                printInfo('No instantiation proposal ID found in config, will use current proposal for event extraction');
            }

            const message: RegisterDeploymentMsg = {
                register_deployment: {
                    deployment_name: deploymentName,
                },
            };
            const messageJson = JSON.stringify(message, null, 2);

            printInfo('Generated register deployment message:', messageJson);

            const title = options.title || `Register Deployment for ${deploymentName}`;
            const description = options.description || `Register deployment with name ${deploymentName} with Coordinator`;

            printInfo('Creating governance proposal...');
            const proposal = encodeExecuteContractProposal(
                this.configManager.getFullConfig(),
                {
                    ...processedOptions,
                    contractName: 'Coordinator',
                    msg: messageJson,
                    title,
                    description,
                },
                undefined,
            );

            if (prompt('Proceed with register deployment proposal submission?', options.yes)) {
                printInfo('Register deployment proposal submission cancelled');
                return;
            }

            printInfo('Submitting register deployment proposal...');
            const proposalId = await RetryManager.withRetry(() =>
                submitProposal(client, wallet, this.configManager.getFullConfig(), processedOptions, proposal),
            );
            printInfo('Register deployment proposal submitted successfully', proposalId);

            printInfo('Waiting for proposal execution and extracting events...');
            if (instantiationProposalId) {
                await this.extractAddressesFromProposalResult(instantiationProposalId, client);
            } else {
                printInfo('No instantiation proposal ID found in config, skipping event extraction');
            }

            this.configManager.saveConfig();
        } catch (error) {
            printError('Error in GovernanceManager:', (error as Error).message);
            throw error;
        }
    }

    public async updateInstantiateConfig(options: CoordinatorOptions): Promise<void> {
        try {
            printInfo('Preparing update instantiate config proposals...');
            printInfo(`Environment: ${this.configManager.getEnvironment()}`);

            const processedOptions = this.configManager.processOptions(options);
            const { wallet, client } = await this.prepareWalletAndClient(processedOptions);
            const governanceAddress = processedOptions.governanceAddress;

            if (!governanceAddress) {
                throw new Error(
                    'Governance address is required. Please provide --governance-address or ensure it is configured in the config file.',
                );
            }

            let coordinatorAddress: string | undefined;
            try {
                coordinatorAddress = this.configManager.getContractAddressFromConfig('Coordinator');
            } catch (error) {
                throw new Error(
                    'Coordinator address not found in config. Please provide --coordinator-address or ensure it is configured in the config file.',
                );
            }

            const addresses = [governanceAddress];
            if (coordinatorAddress) {
                addresses.push(coordinatorAddress);
            }

            const accessConfig = {
                permission: AccessType.ACCESS_TYPE_ANY_OF_ADDRESSES,
                addresses: addresses,
            };

            printInfo(`Using addresses for access config: ${addresses.join(', ')}`);

            await this.codeIdUtils.fetchAndUpdateCodeIdsFromProposals(client, CONTRACTS_TO_HANDLE);

            for (const contractName of CONTRACTS_TO_HANDLE) {
                printInfo(`\n--- Updating instantiate config for ${contractName} ---`);

                const contractConfig = this.configManager.getContractConfig(contractName);
                const codeId = contractConfig.codeId;

                if (!codeId) {
                    printInfo(`${contractName} code ID not found. Skipping ${contractName}.`);
                    continue;
                }

                printInfo(`Using ${contractName} code ID: ${codeId}`);
                printInfo(`Code ID: ${codeId}`);
                printInfo(`Access Config: ${JSON.stringify(accessConfig, null, 2)}`);

                const title = options.title || `Update Instantiate Config for ${contractName} Code ID ${codeId}`;
                const description =
                    options.description ||
                    `Allow governance address ${governanceAddress}${coordinatorAddress ? ` and coordinator address ${coordinatorAddress}` : ''} to instantiate ${contractName} code ID ${codeId}`;

                printInfo('Creating governance proposal...');
                const proposal = this.encodeUpdateInstantiateConfigProposal({
                    title,
                    description,
                    accessConfigUpdates: [
                        {
                            codeId: BigInt(codeId),
                            instantiatePermission: {
                                permission: AccessType.ACCESS_TYPE_ANY_OF_ADDRESSES,
                                addresses: addresses,
                                address: '',
                            },
                        },
                    ],
                });

                const printableProposal = {
                    typeUrl: proposal.typeUrl,
                    value: `<Uint8Array of length ${proposal.value.length}>`,
                };
                printInfo('Proposal details:', JSON.stringify(printableProposal, null, 2));

                if (prompt(`Proceed with update instantiate config proposal submission for ${contractName}?`, options.yes)) {
                    printInfo(`${contractName} update instantiate config proposal submission cancelled`);
                    continue;
                }

                printInfo(`Submitting update instantiate config proposal for ${contractName}...`);
                const proposalId = await RetryManager.withRetry(() =>
                    submitProposal(client, wallet, this.configManager.getFullConfig(), processedOptions, proposal),
                );
                printInfo(`${contractName} update instantiate config proposal submitted successfully`, proposalId);

                printInfo(`--- ${contractName} instantiate config update completed ---\n`);
            }

            this.configManager.saveConfig();
            printInfo('All instantiate config update proposals completed!');
        } catch (error) {
            printError('Error in GovernanceManager:', (error as Error).message);
            throw error;
        }
    }

    private async prepareWalletAndClient(options: CoordinatorOptions): Promise<WalletAndClient> {
        printInfo('Preparing wallet and client...');
        const wallet = await prepareWallet(options as { mnemonic: string });
        const client = await prepareClient(this.configManager.getFullConfig() as { axelar: { rpc: string; gasPrice: string } }, wallet);
        return { wallet, client };
    }

    private async extractAddressesFromProposalResult(proposalId: string, client: unknown): Promise<void> {
        // TODO tkulik: implement fetching Gateway, VotingVerifier, and MultisigProver addresses from proposal result
    }

    private getUpdateInstantiateConfigParams(options: { title: string; description: string; accessConfigUpdates: AccessConfigUpdate[] }) {
        const { title, description, accessConfigUpdates } = options;

        return {
            title,
            description,
            accessConfigUpdates,
        };
    }

    private encodeUpdateInstantiateConfigProposal(options: {
        title: string;
        description: string;
        accessConfigUpdates: AccessConfigUpdate[];
    }) {
        const proposal = UpdateInstantiateConfigProposal.fromPartial(this.getUpdateInstantiateConfigParams(options));

        return {
            typeUrl: '/cosmwasm.wasm.v1.UpdateInstantiateConfigProposal',
            value: Uint8Array.from(UpdateInstantiateConfigProposal.encode(proposal).finish()),
        };
    }
}
