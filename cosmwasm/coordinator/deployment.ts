import { printInfo, prompt } from '../../common';
import { encodeStoreCodeProposal, getContractCodePath, initContractConfig } from '../utils';
import { ConfigManager } from './config';
import type { DeployContractsOptions } from './option-processor';
import { ProposalManager } from './proposal-manager';

export class DeploymentManager {
    private configManager: ConfigManager;
    private proposalManager: ProposalManager;

    constructor(configManager: ConfigManager, proposalManager: ProposalManager) {
        this.configManager = configManager;
        this.proposalManager = proposalManager;
    }

    public async deployContract(contractName: string, processedOptions: DeployContractsOptions, version: string): Promise<void> {
        initContractConfig(this.configManager.getFullConfig(), { contractName, chainName: undefined });

        const contractCodePath = await getContractCodePath({ artifactDir: processedOptions.artifactDir, version }, contractName);

        printInfo(`The contract ${contractName} is being deployed from ${contractCodePath}.`);

        if (prompt(`Proceed with ${contractName} deployment?`, processedOptions.yes)) {
            printInfo(`${contractName} deployment cancelled`);
            return;
        }

        const title = `Store Code for ${contractName}`;
        const description = `Store ${contractName} contract code on Axelar`;
        const coordinatorAddress = this.configManager.getContractAddressFromConfig('Coordinator');
        const [account] = await this.proposalManager.getAccounts(processedOptions.mnemonic);
        const senderAddress = account.address;
        const instantiateAddresses = [coordinatorAddress, senderAddress];
        const proposalId = await this.proposalManager.submitProposal(
            encodeStoreCodeProposal({
                ...processedOptions,
                contractName,
                contractCodePath,
                title,
                description,
                instantiateAddresses,
            }),
            processedOptions.mnemonic,
            processedOptions.deposit,
        );

        printInfo(`Submitted governance proposal for ${contractName} with proposalId: ${proposalId}`);

        this.configManager.storeContractInfo(contractName, proposalId, contractCodePath);
        this.configManager.saveConfig();
    }

    public async deployContracts(options: DeployContractsOptions): Promise<void> {
        printInfo('Deploying VotingVerifier, MultisigProver, and Gateway contracts...');
        printInfo(`Environment: ${this.configManager.getEnvironment()}`);
        printInfo(`\n--- Deploying VotingVerifier ---`);
        await this.deployContract('VotingVerifier', options, options.versionVerifier);
        printInfo(`\n--- Deploying MultisigProver ---`);
        await this.deployContract('MultisigProver', options, options.versionMultisig);
        printInfo(`\n--- Deploying Gateway ---`);
        await this.deployContract('Gateway', options, options.versionGateway);
    }
}
