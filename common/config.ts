import { GasPrice, StdFee, calculateFee } from '@cosmjs/stargate';

import { loadConfig, printWarn, saveConfig } from './utils';

export interface FullConfig {
    axelar: AxelarConfig;
    chains: Record<string, ChainConfig>;
}

export interface AxelarConfig {
    contracts: Record<string, AxelarContractConfig>;
    rpc: string;
    gasPrice: string;
    gasLimit: string | number;
    govProposalInstantiateAddresses: string[];
    govProposalDepositAmount: string;
    chainId: string;
}

export interface NonEVMChainConfig {
    name: string;
    axelarId: string;
    rpc: string;
    tokenSymbol: string;
    decimals: number;
    confirmations?: number;
    chainType: string;
    explorer: ExplorerConfig;
    finality: string;
    approxFinalityWaitTime: number;
    contracts: Record<string, ContractConfig>;
}

export type ChainConfig = NonEVMChainConfig | EVMChainConfig;

export interface EVMChainConfig extends NonEVMChainConfig {
    chainId: number;
}

export interface ExplorerConfig {
    name?: string;
    url?: string;
    api?: string;
}

export interface ContractConfig {
    address?: string;
    codeId?: number;
    storeCodeProposalCodeHash?: string;
    storeCodeProposalId?: string;
    lastUploadedCodeId?: number;
}

export interface AxelarContractConfig extends ContractConfig {
    governanceAddress?: string;
    governanceAccount?: string;
    [chainName: string]: unknown;
}

export class ConfigManager implements FullConfig {
    private environment: string;

    public axelar: AxelarConfig;
    public chains: Record<string, ChainConfig>;

    constructor(environment: string, fullConfig?: FullConfig) {
        this.environment = environment;

        if (!fullConfig) {
            const loadedConfig = loadConfig(this.environment);
            if (!loadedConfig) {
                throw new Error(`Failed to load configuration for environment: ${this.environment}`);
            }
            fullConfig = loadedConfig;
        }

        this.axelar = fullConfig.axelar;
        this.chains = fullConfig.chains;

        this.validateConfig();
    }

    private validateConfig(): void {
        const errors: string[] = [...this.validateBasicStructure(), ...this.validateAxelarConfig(), ...this.validateChainConfigs()];

        if (errors.length > 0) {
            this.printValidationReport(errors);
        }
    }

    private validateBasicStructure(): string[] {
        const errors: string[] = [];
        const { axelar, chains } = this;

        if (!axelar) errors.push(`Missing 'axelar' section in ${this.environment} config`);
        if (!chains)
            errors.push(`Missing 'chains' section in ${this.environment} config. Please ensure the config file has a 'chains' property.`);
        else if (typeof chains !== 'object' || chains === null)
            errors.push(`'chains' section in ${this.environment} config must be an object`);

        return errors;
    }

    private validateAxelarConfig(): string[] {
        const errors: string[] = [];
        const { axelar } = this;
        if (!axelar) return errors;

        const requiredFields = [
            'contracts',
            'rpc',
            'gasPrice',
            'gasLimit',
            'govProposalInstantiateAddresses',
            'govProposalDepositAmount',
            'chainId',
        ];
        requiredFields.forEach((field) => {
            if (axelar[field] === undefined || axelar[field] === null) {
                errors.push(`Missing 'axelar.${field}' in ${this.environment} config`);
            }
        });

        const validations = [
            {
                condition: !axelar.gasPrice || axelar.gasPrice.trim() === '' || !this.isValidGasPrice(axelar.gasPrice),
                message: `Invalid 'axelar.gasPrice' format: ${axelar.gasPrice} - must be a non-empty valid gas price`,
            },
            {
                condition:
                    !axelar.gasLimit ||
                    (typeof axelar.gasLimit !== 'number' &&
                        axelar.gasLimit !== 'auto' &&
                        (typeof axelar.gasLimit !== 'string' || axelar.gasLimit.trim() === '' || !this.isValidNumber(axelar.gasLimit))),
                message: `Invalid 'axelar.gasLimit' format: ${axelar.gasLimit} - must be a number, 'auto', or a valid string number`,
            },
            {
                condition: !axelar.govProposalInstantiateAddresses || !Array.isArray(axelar.govProposalInstantiateAddresses),
                message: `Invalid 'axelar.govProposalInstantiateAddresses' in ${this.environment} config`,
            },
            {
                condition: !axelar.chainId || typeof axelar.chainId !== 'string' || axelar.chainId.trim() === '',
                message: `Invalid 'axelar.chainId' format: ${axelar.chainId} - must be a non-empty string`,
            },
        ];

        validations.forEach(({ condition, message }) => condition && errors.push(message));
        return errors;
    }

    private validateChainConfigs(): string[] {
        const errors: string[] = [];
        if (!this.chains) return errors;

        Object.entries(this.chains).forEach(([chainName, chainConfig]) => {
            errors.push(...this.validateSingleChain(chainName, chainConfig));
        });

        return errors;
    }

    private validateSingleChain(chainName: string, chainConfig: ChainConfig): string[] {
        const errors: string[] = [];
        const requiredFields = [
            'name',
            'axelarId',
            'rpc',
            'tokenSymbol',
            'decimals',
            'chainType',
            'explorer',
            'finality',
            'approxFinalityWaitTime',
            'contracts',
        ];
        const validChainTypes = ['evm', 'cosmos', 'stellar', 'sui', 'svm', 'xrpl', 'hedera'];

        requiredFields.forEach((field) => {
            if (chainConfig[field] === undefined || chainConfig[field] === null) {
                errors.push(`Chain '${chainName}': Missing required field '${field}'`);
            }
        });

        if (chainConfig.chainType === 'evm') {
            const evmConfig = chainConfig as EVMChainConfig;
            if (!evmConfig.chainId || !Number.isInteger(evmConfig.chainId) || evmConfig.chainId <= 0) {
                errors.push(`Chain '${chainName}': Missing or invalid chainId '${evmConfig.chainId}' - must be a positive integer`);
            }
        }

        const typeValidations = [
            { condition: typeof chainConfig.tokenSymbol !== 'string', message: `Chain '${chainName}': tokenSymbol must be a string` },
            {
                condition: !Number.isInteger(chainConfig.decimals) || chainConfig.decimals < 0 || chainConfig.decimals > 18,
                message: `Chain '${chainName}': Invalid decimals '${chainConfig.decimals}' - must be an integer between 0 and 18`,
            },
            {
                condition: chainConfig.chainType && !validChainTypes.includes(chainConfig.chainType),
                message: `Chain '${chainName}': Invalid chainType '${chainConfig.chainType}' - must be one of: ${validChainTypes.join(', ')}`,
            },
            {
                condition: chainConfig.finality && typeof chainConfig.finality !== 'string',
                message: `Chain '${chainName}': Finality must be a string`,
            },
            {
                condition: chainConfig.finality && chainConfig.finality !== 'finalized' && !this.isValidNumber(chainConfig.finality),
                message: `Chain '${chainName}': Invalid finality value '${chainConfig.finality}' - must be 'finalized' or a number`,
            },
            {
                condition:
                    chainConfig.approxFinalityWaitTime !== undefined &&
                    (typeof chainConfig.approxFinalityWaitTime !== 'number' || chainConfig.approxFinalityWaitTime < 0),
                message: `Chain '${chainName}': approxFinalityWaitTime must be a non-negative number`,
            },
        ];

        typeValidations.forEach(({ condition, message }) => condition && errors.push(message));

        if (chainConfig.contracts) {
            Object.entries(chainConfig.contracts).forEach(([contractName, contractConfig]) => {
                errors.push(...this.validateContractConfig(chainName, contractName, contractConfig));
            });
        }

        return errors;
    }

    private validateContractConfig(chainName: string, contractName: string, contractConfig: ContractConfig): string[] {
        const errors: string[] = [];
        const contractValidations = [
            {
                condition: contractConfig.address && !this.isValidAddress(contractConfig.address),
                message: `Chain '${chainName}': Contract '${contractName}' has invalid address format: ${contractConfig.address}`,
            },
            {
                condition: contractConfig.codeId && (!Number.isInteger(contractConfig.codeId) || contractConfig.codeId <= 0),
                message: `Chain '${chainName}': Contract '${contractName}' has invalid codeId '${contractConfig.codeId}' - must be a positive integer`,
            },
        ];

        contractValidations.forEach(({ condition, message }) => condition && errors.push(message));
        return errors;
    }

    private printValidationReport(errors: string[]): void {
        printWarn(`\n❌ Configuration Validation Report for ${this.environment.toUpperCase()}`);
        printWarn(`Found ${errors.length} error(s).\n`);

        if (errors.length > 0) {
            errors.forEach((error, index) => {
                printWarn(`  ${index + 1}. ${error}`);
            });
        }

        printWarn('📋 SUMMARY:');
        printWarn(`  Total Errors: ${errors.length}`);
        printWarn(`  Configuration Status: ${errors.length > 0 ? 'INVALID' : 'VALID'}`);
        printWarn('');
    }

    private isValidGasPrice(price: string): boolean {
        const numericOnlyPattern = /^\d+$/;
        const withDenominationPattern = /^\d+(\.\d+)?[a-zA-Z]+$/;

        if (numericOnlyPattern.test(price)) {
            return parseInt(price) > 0;
        }

        if (withDenominationPattern.test(price)) {
            const match = price.match(/^\d+(\.\d+)?/);
            return match ? parseFloat(match[0]) > 0 : false;
        }

        return false;
    }

    private isValidNumber(str: string): boolean {
        return !isNaN(Number(str)) && isFinite(Number(str));
    }

    private isValidAddress(address: string): boolean {
        return typeof address === 'string' && address.length > 0;
    }

    public initContractConfig(contractName: string, chainName: string) {
        if (!contractName) {
            return;
        }

        if (!this.axelar.contracts[contractName]) {
            this.axelar.contracts[contractName] = {};
        }

        if (chainName) {
            if (!this.axelar.contracts[contractName][chainName]) {
                this.axelar.contracts[contractName][chainName] = {};
            }
        }
    }

    public saveConfig(): void {
        saveConfig({ axelar: this.axelar, chains: this.chains }, this.environment);
    }

    public getProposalInstantiateAddresses(): string[] {
        return this.axelar.govProposalInstantiateAddresses;
    }

    public getProposalDepositAmount(): string {
        return this.axelar.govProposalDepositAmount;
    }

    public getChainConfig(chainName: string): ChainConfig {
        const chainConfig = this.chains[chainName];
        if (!chainConfig) {
            throw new Error(`Chain '${chainName}' not found in ${this.environment} config`);
        }
        return chainConfig;
    }

    public getContractConfig(configContractName: string): ContractConfig {
        const axelarContracts = this.axelar.contracts;
        if (!axelarContracts) {
            throw new Error(`Axelar contracts section not found in config for environment ${this.environment}`);
        }

        if (!axelarContracts[configContractName]) {
            axelarContracts[configContractName] = {};
        }

        return axelarContracts[configContractName];
    }

    public getContractConfigByChain(configContractName: string, chainName: string): ContractConfig {
        const contractConfig = this.getContractConfig(configContractName);
        if (!contractConfig[chainName]) {
            contractConfig[chainName] = {};
        }
        return contractConfig[chainName];
    }

    public getFee(): string | StdFee {
        const { gasPrice, gasLimit } = this.axelar;

        if (gasLimit === 'auto') {
            return 'auto';
        }

        const numericGasLimit = typeof gasLimit === 'string' ? Number(gasLimit) : gasLimit;
        return calculateFee(numericGasLimit, GasPrice.fromString(gasPrice));
    }
}
