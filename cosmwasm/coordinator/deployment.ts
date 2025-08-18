import { printInfo, prompt } from '../../common';
import { encodeStoreCodeProposal, getContractCodePath, initContractConfig, prepareClient, prepareWallet, submitProposal } from '../utils';
import { ConfigManager } from './config';
import { RetryManager } from './retry';
import type { CoordinatorOptions, WalletAndClient } from './types';
import { AMPLIFIER_CONTRACTS_TO_HANDLE } from './types';

export class DeploymentManager {
    private configManager: ConfigManager;

    constructor(configManager: ConfigManager) {
        this.configManager = configManager;
    }

    public async deployContract(contractName: string, options: CoordinatorOptions): Promise<void> {
        printInfo(`Deploying ${contractName} contract...`);

        initContractConfig(this.configManager.getFullConfig(), { contractName, chainName: undefined });

        const processedOptions = this.configManager.processOptions(options);
        const { wallet, client } = await this.prepareWalletAndClient(processedOptions);
        const contractCodePath = await getContractCodePath(processedOptions, contractName);

        printInfo(`The contract ${contractName} is being deployed from ${contractCodePath}.`);

        if (prompt(`Proceed with ${contractName} deployment?`, options.yes)) {
            printInfo(`${contractName} deployment cancelled`);
            return;
        }

        printInfo(`Submitting governance proposal for ${contractName}...`);

        const title = `Store Code for ${contractName}`;
        const description = `Store ${contractName} contract code on Axelar`;

        const coordinatorAddress = this.configManager.getContractAddressFromConfig('Coordinator');
        const accounts = await wallet.getAccounts();
        const senderAddress = accounts[0].address;
        const instantiateAddresses = [coordinatorAddress, senderAddress];

        printInfo(`Setting instantiate permissions for ${contractName} with addresses: ${instantiateAddresses.join(', ')}`);

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
                    instantiateAddresses,
                }),
            ),
        );

        printInfo(`Submitted governance proposal for ${contractName} with proposalId: ${proposalId}`);

        this.configManager.storeContractInfo(contractName, proposalId, contractCodePath);
        this.configManager.saveConfig();
    }

    public async deployContracts(options: CoordinatorOptions): Promise<void> {
        printInfo('Deploying VotingVerifier, MultisigProver, and Gateway contracts...');
        printInfo(`Environment: ${this.configManager.getEnvironment()}`);

        for (const contractName of AMPLIFIER_CONTRACTS_TO_HANDLE) {
            printInfo(`\n--- Deploying ${contractName} ---`);
            await this.deployContract(contractName, options);
        }
    }

    private async prepareWalletAndClient(options: CoordinatorOptions): Promise<WalletAndClient> {
        printInfo('Preparing wallet and client...');
        const wallet = await prepareWallet(options as { mnemonic: string });
        const client = await prepareClient(this.configManager.getFullConfig() as { axelar: { rpc: string; gasPrice: string } }, wallet);
        return { wallet, client };
    }
}
