import { createHash } from 'crypto';

import { loadConfig, printInfo, printWarn, readContractCode, saveConfig } from '../../common';

export const AMPLIFIER_CONTRACTS_TO_HANDLE = ['VotingVerifier', 'MultisigProver', 'Gateway'];

export interface FullConfig {
    axelar?: {
        contracts?: {
            [key: string]: ContractConfig & {
                governanceAddress?: string;
                governanceAccount?: string;
            };
        };
        rpc?: string;
        gasPrice?: string;
        gasLimit?: string | number;
        govProposalInstantiateAddresses?: string[];
    };
    chains?: {
        [chainName: string]: ChainConfig;
    };
    [key: string]: unknown;
}

export interface ChainConfig {
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

export interface ContractConfig {
    address?: string;
    codeId?: number;
    storeCodeProposalCodeHash?: string;
    storeCodeProposalId?: string;
    lastUploadedCodeId?: number;
    [key: string]: unknown;
}

export class ConfigManager {
    private environment: string;
    private fullConfig: FullConfig;

    constructor(environment: string) {
        this.environment = environment;
        this.fullConfig = loadConfig(this.environment);
    }

    public getChainConfig(chainName: string): ChainConfig {
        const chainConfig = this.fullConfig.chains[chainName];
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
        }

        return axelarContracts[configContractName];
    }

    public getContractAddressFromConfig(contractName: string): string | undefined {
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

        return contract.address;
    }

    public getContractAddressFromChainConfig(chainName: string, contractName: string): string {
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

    public getDefaultGovernanceAddress(): string {
        const axelarConfig = this.fullConfig.axelar;
        if (!axelarConfig) {
            throw new Error('Axelar configuration not found in config');
        }

        const instantiateAddresses = axelarConfig.govProposalInstantiateAddresses;
        if (instantiateAddresses && Array.isArray(instantiateAddresses) && instantiateAddresses.length > 0) {
            const defaultAddress = instantiateAddresses[0];
            printInfo(`Using default governance address from config: ${defaultAddress}`);
            return defaultAddress;
        }

        const contracts = axelarConfig.contracts;
        if (contracts) {
            const coordinatorContract = contracts.Coordinator as { governanceAddress?: string };
            if (coordinatorContract?.governanceAddress) {
                printInfo(`Using Coordinator governance address read from config: ${coordinatorContract.governanceAddress}`);
                return coordinatorContract.governanceAddress;
            }

            const serviceRegistryContract = contracts.ServiceRegistry as { governanceAccount?: string };
            if (serviceRegistryContract?.governanceAccount) {
                printInfo(`Using ServiceRegistry governance account read from config: ${serviceRegistryContract.governanceAccount}`);
                return serviceRegistryContract.governanceAccount;
            }
        }

        throw new Error(
            `No governance addresses found in config for environment ${this.environment}. ` +
                `Please add 'govProposalInstantiateAddresses' array to the axelar section of your config file, ` +
                `or ensure that contract configurations have governance addresses.`,
        );
    }

    public updateContractCodeId(configContractName: string, codeId: number): void {
        if (!this.fullConfig.axelar?.contracts?.[configContractName]) {
            throw new Error(`Contract ${configContractName} not found in config`);
        }

        this.fullConfig.axelar.contracts[configContractName].codeId = codeId;
        this.fullConfig.axelar.contracts[configContractName].lastUploadedCodeId = codeId;
    }

    public storeContractInfo(configContractName: string, proposalId: string, contractCodePath: string): void {
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
        } catch (error) {
            printWarn(`Failed to extract code hash for ${configContractName}: ${(error as Error).message}`);
            printWarn(`Code hash will be extracted when fetching code ID from the chain`);
            codeHash = undefined;
        }

        this.fullConfig.axelar.contracts[configContractName].storeCodeProposalId = proposalId;
        this.fullConfig.axelar.contracts[configContractName].storeCodeProposalCodeHash = codeHash;
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

        for (const contractName of AMPLIFIER_CONTRACTS_TO_HANDLE) {
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

        for (const contractName of AMPLIFIER_CONTRACTS_TO_HANDLE) {
            const contract = axelarContracts[contractName]?.[chainName] as { proposalId?: string };
            if (contract?.proposalId) {
                return contract.proposalId;
            }
        }

        return undefined;
    }
}
