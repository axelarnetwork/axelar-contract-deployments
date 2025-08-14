import { createHash } from 'crypto';
import * as fs from 'fs';
import * as path from 'path';

import { loadConfig, printInfo, printWarn, readContractCode, saveConfig } from '../../common';
import { CONTRACTS_TO_HANDLE } from './constants';
import type { ChainConfig, ConfigFile, ContractConfig, CoordinatorOptions, FullConfig } from './types';

export class ConfigManager {
    private config: ConfigFile;
    private environment: string;
    private fullConfig: FullConfig;

    constructor(environment: string) {
        this.environment = environment;
        this.loadConfig();
        this.fullConfig = loadConfig(this.environment);
    }

    private loadConfig(): void {
        const configPath = path.join(__dirname, '../../axelar-chains-config/info', `${this.environment}.json`);

        if (!fs.existsSync(configPath)) {
            throw new Error(`Config file not found: ${configPath}`);
        }

        const configData = fs.readFileSync(configPath, 'utf8');
        this.config = JSON.parse(configData);
    }

    public getChainConfig(chainName: string): ChainConfig {
        const chainConfig = this.config.chains[chainName];
        if (!chainConfig) {
            throw new Error(`Chain '${chainName}' not found in ${this.environment} config`);
        }
        return chainConfig;
    }

    public getContractConfig(configContractName: string): ContractConfig {
        const axelarContracts = this.fullConfig.axelar?.contracts;
        if (!axelarContracts) {
            throw new Error(`Axelar contracts section not found in config for environment ${this.environment}`);
        }

        if (!axelarContracts[configContractName]) {
            axelarContracts[configContractName] = {};
            printInfo(`Initialized contract config for ${configContractName}`);
        }

        return axelarContracts[configContractName];
    }

    public getContractAddressFromConfig(contractName: string): string | undefined {
        printInfo(`Getting ${contractName} address from config...`);

        const axelarContracts = this.fullConfig.axelar?.contracts;
        if (!axelarContracts) {
            throw new Error('Axelar contracts not found in config');
        }

        const contract = axelarContracts[contractName];
        if (!contract) {
            throw new Error(`${contractName} contract not found in axelar config`);
        }

        if (!contract.address) {
            throw new Error(`${contractName} address not found in axelar config. Please ensure the contract has been deployed.`);
        }

        printInfo(`Found ${contractName} address in axelar config: ${contract.address}`);
        return contract.address;
    }

    public getContractAddressFromChainConfig(chainName: string, contractName: string): string {
        printInfo(`Getting ${contractName} address from chain config...`);

        const chainConfig = this.getChainConfig(chainName);
        if (!chainConfig) {
            throw new Error(`Chain ${chainName} not found in config`);
        }

        const contract = chainConfig.contracts[contractName];
        if (!contract) {
            throw new Error(`${contractName} contract not found in ${chainName} config`);
        }

        if (!contract.address) {
            throw new Error(`${contractName} address not found in ${chainName} config. Please ensure the contract has been deployed.`);
        }

        return contract.address;
    }

    public getDefaultAddress(addressType: 'runAs' | 'governance'): string {
        const axelarConfig = this.fullConfig.axelar;
        if (!axelarConfig) {
            throw new Error('Axelar configuration not found in config');
        }

        const instantiateAddresses = axelarConfig.govProposalInstantiateAddresses;
        if (instantiateAddresses && Array.isArray(instantiateAddresses) && instantiateAddresses.length > 0) {
            const defaultAddress = instantiateAddresses[0];
            printInfo(`Using default ${addressType} address from config: ${defaultAddress}`);
            return defaultAddress;
        }

        const contracts = axelarConfig.contracts;
        if (contracts) {
            const coordinatorContract = contracts.Coordinator as { governanceAddress?: string };
            if (coordinatorContract?.governanceAddress) {
                printInfo(
                    `Using Coordinator governance address as fallback ${addressType} address: ${coordinatorContract.governanceAddress}`,
                );
                return coordinatorContract.governanceAddress;
            }

            const serviceRegistryContract = contracts.ServiceRegistry as { governanceAccount?: string };
            if (serviceRegistryContract?.governanceAccount) {
                printInfo(
                    `Using ServiceRegistry governance account as fallback ${addressType} address: ${serviceRegistryContract.governanceAccount}`,
                );
                return serviceRegistryContract.governanceAccount;
            }
        }

        throw new Error(
            `No ${addressType} addresses found in config for environment ${this.environment}. ` +
                `Please add 'govProposalInstantiateAddresses' array to the axelar section of your config file, ` +
                `or ensure that contract configurations have governance addresses.`,
        );
    }

    public getDefaultRunAsAddress(): string {
        return this.getDefaultAddress('runAs');
    }

    public getDefaultGovernanceAddress(): string {
        return this.getDefaultAddress('governance');
    }

    public processOptions(options: CoordinatorOptions): CoordinatorOptions {
        const processedOptions = { ...options };

        if (!processedOptions.runAs) {
            processedOptions.runAs = this.getDefaultRunAsAddress();
        }

        if (!processedOptions.governanceAddress) {
            processedOptions.governanceAddress = this.getDefaultGovernanceAddress();
        }

        if (!processedOptions.rewardsAddress) {
            try {
                processedOptions.rewardsAddress = this.getContractAddressFromConfig('Rewards');
                printInfo(`Using rewards address from config: ${processedOptions.rewardsAddress}`);
            } catch (error) {
                printWarn(`Could not get rewards address from config: ${(error as Error).message}`);
            }
        }

        return processedOptions;
    }

    public updateContractCodeId(configContractName: string, codeId: number): void {
        if (!this.fullConfig.axelar?.contracts?.[configContractName]) {
            throw new Error(`Contract ${configContractName} not found in config`);
        }

        this.fullConfig.axelar.contracts[configContractName].codeId = codeId;
        this.fullConfig.axelar.contracts[configContractName].lastUploadedCodeId = codeId;

        printInfo(`Updated code ID for ${configContractName} in config: ${codeId}`);
    }

    public storeDeploymentInfo(configContractName: string, proposalId: string, contractCodePath: string): void {
        if (!this.fullConfig.axelar?.contracts?.[configContractName]) {
            throw new Error(`Contract ${configContractName} not found in config`);
        }

        let codeHash: string | undefined;
        try {
            const options = {
                contractName: configContractName,
                contractCodePath,
                artifactPath: contractCodePath,
                version: undefined,
            };

            codeHash = this.getContractCodeHash(options);
            printInfo(`Successfully extracted code hash for ${configContractName}: ${codeHash}`);
        } catch (error) {
            printWarn(`Failed to extract code hash for ${configContractName}: ${(error as Error).message}`);
            printWarn(`Code hash will be extracted when fetching code ID from the chain`);
            codeHash = undefined;
        }

        this.fullConfig.axelar.contracts[configContractName].storeCodeProposalId = proposalId;
        this.fullConfig.axelar.contracts[configContractName].storeCodeProposalCodeHash = codeHash;

        printInfo(`Stored deployment info for ${configContractName}:`);
        printInfo(`  Proposal ID: ${proposalId}`);
        printInfo(`  Code Hash: ${codeHash || 'Will be extracted when fetching code ID'}`);
    }

    private getContractCodeHash(options: {
        contractName: string;
        contractCodePath: string;
        artifactPath: string;
        version?: string;
    }): string {
        const wasm = readContractCode(options);
        return createHash('sha256').update(wasm).digest('hex');
    }

    public saveConfig(): void {
        saveConfig(this.fullConfig, this.environment);
    }

    public getFullConfig(): FullConfig {
        return this.fullConfig;
    }

    public getEnvironment(): string {
        return this.environment;
    }

    public getDeploymentNameFromConfig(chainName: string): string {
        const axelarContracts = this.fullConfig.axelar?.contracts;

        if (!axelarContracts) {
            throw new Error('Axelar contracts section not found in config');
        }

        const deploymentNames = new Set<string>();

        for (const contractName of CONTRACTS_TO_HANDLE) {
            const contract = axelarContracts[contractName]?.[chainName] as { deploymentName?: string };
            if (!contract) {
                throw new Error(`Contract ${contractName} not found in config`);
            }

            if (contract.deploymentName === undefined) {
                throw new Error(`Contract ${contractName} is missing the deploymentName property in its chain-specific configuration`);
            }

            deploymentNames.add(contract.deploymentName);
        }

        if (deploymentNames.size !== 1) {
            throw new Error(`All contracts must have the same deployment name. Found: ${Array.from(deploymentNames).join(', ')}`);
        }

        const deploymentName = Array.from(deploymentNames)[0];
        return deploymentName;
    }

    public getInstantiationProposalIdFromConfig(chainName: string): string | undefined {
        const axelarContracts = this.fullConfig.axelar?.contracts;
        if (!axelarContracts) {
            return undefined;
        }

        for (const contractName of CONTRACTS_TO_HANDLE) {
            const contract = axelarContracts[contractName]?.[chainName] as { proposalId?: string };
            if (contract?.proposalId) {
                return contract.proposalId;
            }
        }

        return undefined;
    }
}
