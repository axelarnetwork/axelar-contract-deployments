import { StdFee } from '@cosmjs/stargate';
import { GasPrice, calculateFee } from '@cosmjs/stargate';

import { loadConfig, saveConfig } from '.';

export interface FullConfig {
    axelar: {
        contracts: {
            [key: string]: ContractConfig & {
                governanceAddress?: string;
                governanceAccount?: string;
            };
        };
        rpc: string;
        gasPrice: string;
        gasLimit: string | number;
        govProposalInstantiateAddresses: string[];
        govProposalDepositAmount: string;
    };
    chains: {
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

    constructor(environment: string, fullConfig?: FullConfig) {
        this.environment = environment;

        if (fullConfig) {
            this.fullConfig = fullConfig;
        } else {
            this.fullConfig = loadConfig(this.environment);
        }

        this.validateConfig();
    }

    private validateConfig(): void {
        if (!this.fullConfig.axelar) {
            throw new Error(`Missing 'axelar' section in ${this.environment} config`);
        }

        if (!this.fullConfig.axelar.contracts) {
            throw new Error(`Missing 'axelar.contracts' section in ${this.environment} config`);
        }

        if (!this.fullConfig.chains) {
            throw new Error(
                `Missing 'chains' section in ${this.environment} config. Please ensure the config file has a 'chains' property.`,
            );
        }

        if (typeof this.fullConfig.chains !== 'object' || this.fullConfig.chains === null) {
            throw new Error(`'chains' section in ${this.environment} config must be an object`);
        }

        if (!this.fullConfig.axelar.rpc) {
            throw new Error(`Missing 'axelar.rpc' in ${this.environment} config`);
        }

        if (!this.fullConfig.axelar.gasPrice) {
            throw new Error(`Missing 'axelar.gasPrice' in ${this.environment} config`);
        }

        if (!this.fullConfig.axelar.gasLimit) {
            throw new Error(`Missing 'axelar.gasLimit' in ${this.environment} config`);
        }
    }

    public initContractConfig(contractName: string, chainName: string) {
        if (!this.fullConfig.axelar.contracts[contractName]) {
            this.fullConfig.axelar.contracts[contractName] = {};
        }

        if (chainName) {
            this.fullConfig.axelar.contracts[contractName][chainName] = this.fullConfig.axelar.contracts[contractName][chainName] || {};
        }
    }

    public saveConfig(): void {
        saveConfig(this.fullConfig, this.environment);
    }

    public getFullConfig(): FullConfig {
        return this.fullConfig;
    }

    public getProposalInstantiateAddresses(): string[] {
        return this.fullConfig.axelar.govProposalInstantiateAddresses;
    }

    public getProposalDepositAmount(): string {
        return this.fullConfig.axelar.govProposalDepositAmount;
    }

    public getFee(): string | StdFee {
        const { gasPrice, gasLimit } = this.fullConfig.axelar;

        return gasLimit === 'auto' ? 'auto' : calculateFee(gasLimit as number, GasPrice.fromString(gasPrice));
    }
}
