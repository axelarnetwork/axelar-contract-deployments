import type { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import type { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';

import { printInfo, prompt } from '../../common';
import { encodeStoreCodeProposal, getContractCodePath, initContractConfig, prepareClient, prepareWallet, submitProposal } from '../utils';
import { AMPLIFIER_CONTRACTS_TO_HANDLE, ConfigManager } from './config';
import type { DeployContractsOptions } from './option-processor';
import { RetryManager } from './retry';

export class DeploymentManager {
    private configManager: ConfigManager;

    constructor(configManager: ConfigManager) {
        this.configManager = configManager;
    }

    public async deployContract(contractName: string, processedOptions: DeployContractsOptions): Promise<void> {
        initContractConfig(this.configManager.getFullConfig(), { contractName, chainName: undefined });

        const { wallet, client } = await this.prepareWalletAndClient(processedOptions.mnemonic);
        const contractCodePath = await getContractCodePath(processedOptions, contractName);

        printInfo(`The contract ${contractName} is being deployed from ${contractCodePath}.`);

        if (prompt(`Proceed with ${contractName} deployment?`, processedOptions.yes)) {
            printInfo(`${contractName} deployment cancelled`);
            return;
        }

        const title = `Store Code for ${contractName}`;
        const description = `Store ${contractName} contract code on Axelar`;
        const coordinatorAddress = this.configManager.getContractAddressFromConfig('Coordinator');
        const accounts = await wallet.getAccounts();
        const senderAddress = accounts[0].address;
        const instantiateAddresses = [coordinatorAddress, senderAddress];
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

    public async deployContracts(options: DeployContractsOptions): Promise<void> {
        printInfo('Deploying VotingVerifier, MultisigProver, and Gateway contracts...');
        printInfo(`Environment: ${this.configManager.getEnvironment()}`);

        for (const contractName of AMPLIFIER_CONTRACTS_TO_HANDLE) {
            printInfo(`\n--- Deploying ${contractName} ---`);
            await this.deployContract(contractName, options);
        }
    }

    private async prepareWalletAndClient(mnemonic: string): Promise<{ wallet: DirectSecp256k1HdWallet; client: SigningCosmWasmClient }> {
        printInfo('Preparing wallet and client...');
        const wallet = await prepareWallet({ mnemonic });
        const client = await prepareClient(this.configManager.getFullConfig() as { axelar: { rpc: string; gasPrice: string } }, wallet);
        return { wallet, client };
    }
}
