import { printError, printInfo, prompt } from '../../common';
import {
    encodeStoreCodeProposal,
    getContractCodePath as getContractCodePathUtil,
    initContractConfig,
    prepareClient,
    prepareWallet,
    submitProposal,
    uploadContract,
} from '../utils';
import { ConfigManager } from './config';
import { CONTRACTS_TO_HANDLE } from './constants';
import { RetryManager } from './retry';
import type { CoordinatorOptions, WalletAndClient } from './types';

export class DeploymentManager {
    private configManager: ConfigManager;

    constructor(configManager: ConfigManager) {
        this.configManager = configManager;
    }

    public async deployContract(contractName: string, options: CoordinatorOptions): Promise<void> {
        try {
            printInfo(`Deploying ${contractName} contract...`);

            initContractConfig(this.configManager.getFullConfig(), { contractName, chainName: undefined });

            const processedOptions = this.configManager.processOptions(options);
            const { wallet, client } = await this.prepareWalletAndClient(processedOptions);
            const contractCodePath = await this.getContractCodePath(processedOptions, contractName);

            printInfo(`The contract ${contractName} is being deployed from ${contractCodePath}.`);

            if (prompt(`Proceed with ${contractName} deployment?`, options.yes)) {
                printInfo(`${contractName} deployment cancelled`);
                return;
            }

            printInfo(`Submitting governance proposal for ${contractName}...`);

            const title = options.title || `Store Code for ${contractName}`;
            const description = options.description || `Store ${contractName} contract code on Axelar`;

            const proposalId = await RetryManager.withRetry(() =>
                submitProposal(
                    client,
                    wallet,
                    this.configManager.getFullConfig(),
                    processedOptions,
                    encodeStoreCodeProposal({
                        ...processedOptions,
                        contractName,
                        contractCodePath,
                        title,
                        description,
                    }),
                ),
            );

            printInfo(`Submitted governance proposal for ${contractName} with proposalId: ${proposalId}`);

            this.configManager.storeDeploymentInfo(contractName, proposalId, contractCodePath);
            this.configManager.saveConfig();
        } catch (error) {
            printError('Error in DeploymentManager:', (error as Error).message);
            throw error;
        }
    }

    public async deployContractDirect(contractName: string, options: CoordinatorOptions): Promise<void> {
        try {
            printInfo(`Deploying ${contractName} contract directly (no governance proposal)...`);

            const processedOptions = this.configManager.processOptions(options);

            initContractConfig(this.configManager.getFullConfig(), { contractName, chainName: undefined });

            const { wallet, client } = await this.prepareWalletAndClient(processedOptions);

            const contractCodePath = await this.getContractCodePath(processedOptions, contractName);

            printInfo(`The contract ${contractName} is being deployed from ${contractCodePath}.`);

            if (prompt(`Proceed with ${contractName} direct deployment?`, options.yes)) {
                printInfo(`${contractName} deployment cancelled`);
                return;
            }

            printInfo(`Uploading ${contractName} contract...`);
            const { checksum, codeId } = await RetryManager.withRetry(() =>
                uploadContract(client, wallet, this.configManager.getFullConfig(), {
                    ...processedOptions,
                    contractName,
                    contractCodePath,
                }),
            );

            printInfo(`Uploaded ${contractName} contract with codeId: ${codeId}, checksum: ${checksum}`);

            this.configManager.storeDirectDeploymentInfo(contractName, codeId, checksum);
            this.configManager.saveConfig();
        } catch (error) {
            printError('Error in DeploymentManager:', (error as Error).message);
            throw error;
        }
    }

    public async deployContracts(options: CoordinatorOptions): Promise<void> {
        try {
            printInfo('Deploying VotingVerifier, MultisigProver, and Gateway contracts...');
            printInfo(`Environment: ${this.configManager.getEnvironment()}`);

            for (const contractName of CONTRACTS_TO_HANDLE) {
                printInfo(`\n--- Deploying ${contractName} ---`);
                await this.deployContract(contractName, options);
                printInfo(`--- ${contractName} deployment completed ---\n`);
            }

            printInfo('Deployment information has been stored in the config file.');
            printInfo('You can now use the "update-instantiate-config" command to allow Coordinator to instantiate the contracts.');
        } catch (error) {
            printError('Error in DeploymentManager:', (error as Error).message);
            throw error;
        }
    }

    public async deployContractsDirect(options: CoordinatorOptions): Promise<void> {
        try {
            printInfo('Deploying VotingVerifier, MultisigProver, and Gateway contracts directly (no governance proposals)...');
            printInfo(`Environment: ${this.configManager.getEnvironment()}`);

            for (const contractName of CONTRACTS_TO_HANDLE) {
                printInfo(`\n--- Deploying ${contractName} directly ---`);
                await this.deployContractDirect(contractName, options);
                printInfo(`--- ${contractName} direct deployment completed ---\n`);
            }

            printInfo('Deployment information has been stored in the config file.');
            printInfo('You can now use the "update-instantiate-config" command to allow Coordinator to instantiate the contracts.');
        } catch (error) {
            printError('Error in DeploymentManager:', (error as Error).message);
            throw error;
        }
    }

    private async prepareWalletAndClient(options: CoordinatorOptions): Promise<WalletAndClient> {
        printInfo('Preparing wallet and client...');
        const wallet = await prepareWallet(options as { mnemonic: string });
        const client = await prepareClient(this.configManager.getFullConfig() as { axelar: { rpc: string; gasPrice: string } }, wallet);
        return { wallet, client };
    }

    private async getContractCodePath(options: CoordinatorOptions, contractName: string): Promise<string> {
        return getContractCodePathUtil(options, contractName);
    }
}
