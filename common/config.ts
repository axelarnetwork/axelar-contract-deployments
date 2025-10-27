import { GasPrice, StdFee, calculateFee } from '@cosmjs/stargate';

import { loadConfig, printWarn, saveConfig } from './utils';

export const VERIFIER_CONTRACT_NAME = 'VotingVerifier';
export const GATEWAY_CONTRACT_NAME = 'Gateway';
export const MULTISIG_PROVER_CONTRACT_NAME = 'MultisigProver';

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

export interface DeploymentConfig {
    deploymentName: string;
    salt: string;
    proposalId: string;
}

export interface ContractConfig {
    deployments?: Record<string, DeploymentConfig>;
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

export interface VotingVerifierChainConfig {
    governanceAddress: string;
    serviceName: string;
    sourceGatewayAddress: string;
    votingThreshold: [string, string];
    blockExpiry: number;
    confirmationHeight: number;
    msgIdFormat: string;
    addressFormat: string;
    codeId?: number;
    contractAdmin?: string;
    address?: string;
}

export interface MultisigProverChainConfig {
    governanceAddress: string;
    encoder: string;
    keyType: string;
    adminAddress: string;
    verifierSetDiffThreshold: number;
    signingThreshold: [string, string];
    codeId?: number;
    contractAdmin?: string;
    address?: string;
    domainSeparator?: string;
}

export interface GatewayChainConfig {
    codeId?: number;
    contractAdmin?: string;
    address?: string;
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
        saveConfig({ chains: this.chains, axelar: this.axelar }, this.environment);
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
            throw new Error(`Contract '${configContractName}' not found in ${this.environment} config`);
        }

        return axelarContracts[configContractName];
    }

    public getContractConfigByChain(configContractName: string, chainName: string): ContractConfig {
        const contractConfig = this.getContractConfig(configContractName);
        if (!contractConfig[chainName]) {
            throw new Error(`Contract '${configContractName}' not found on chain '${chainName}' in ${this.environment} config`);
        }
        return contractConfig[chainName];
    }

    public validateRequired<T>(value: T | undefined | null, configPath: string, type?: string): T {
        if (value === undefined || value === null || (typeof value === 'string' && value.trim() === '')) {
            throw new Error(`Missing required configuration for the chain. Please configure it in ${configPath}.`);
        }
        if (type && typeof value !== type) {
            throw new Error(`Invalid configuration for ${configPath}. Expected ${type}, got: ${typeof value}`);
        }
        return value;
    }

    public validateThreshold(value: [string | number, string | number] | undefined | null, configPath: string): [string, string] {
        if (!value || !Array.isArray(value) || value.length !== 2) {
            throw new Error(
                `Missing or invalid threshold configuration for the chain. Please configure it in ${configPath} as [numerator, denominator].`,
            );
        } else if (Number(value[0]) > Number(value[1])) {
            throw new Error(`Invalid threshold configuration for the chain. Numerator must not be greater than denominator.`);
        }
        if (value[0] && typeof value[0] == 'number') {
            value[0] = String(value[0]);
        }
        if (value[1] && typeof value[1] == 'number') {
            value[1] = String(value[1]);
        }
        if (value[0] && typeof value[0] !== 'string') {
            throw new Error(`Invalid threshold configuration for the chain. Numerator must be a string.`);
        }
        if (value[1] && typeof value[1] !== 'string') {
            throw new Error(`Invalid threshold configuration for the chain. Denominator must be a string.`);
        }
        return value as [string, string];
    }

    public getMultisigProverContractForChainType(chainType: string): string {
        const chainProverMapping: Record<string, string> = {
            svm: 'SolanaMultisigProver',
        };
        return chainProverMapping[chainType] || MULTISIG_PROVER_CONTRACT_NAME;
    }

    public getMultisigProverContract(chainName: string): MultisigProverChainConfig {
        const chainConfig = this.getChainConfig(chainName);
        const multisigProverContractName = this.getMultisigProverContractForChainType(chainConfig.chainType);
        const multisigProverConfig = this.getContractConfigByChain(multisigProverContractName, chainName) as MultisigProverChainConfig;

        this.validateRequired(multisigProverConfig.encoder, `${multisigProverContractName}[${chainName}].encoder`, 'string');
        this.validateRequired(multisigProverConfig.keyType, `${multisigProverContractName}[${chainName}].keyType`, 'string');
        this.validateRequired(multisigProverConfig.adminAddress, `${multisigProverContractName}[${chainName}].adminAddress`, 'string');
        this.validateRequired(
            multisigProverConfig.verifierSetDiffThreshold,
            `${multisigProverContractName}[${chainName}].verifierSetDiffThreshold`,
            'number',
        );
        this.validateThreshold(multisigProverConfig.signingThreshold, `${multisigProverContractName}[${chainName}].signingThreshold`);
        this.validateRequired(
            multisigProverConfig.governanceAddress,
            `${multisigProverContractName}[${chainName}].governanceAddress`,
            'string',
        );

        return multisigProverConfig;
    }

    public getVotingVerifierContract(chainName: string): VotingVerifierChainConfig {
        const votingVerifierConfig = this.getContractConfigByChain(VERIFIER_CONTRACT_NAME, chainName) as VotingVerifierChainConfig;

        this.validateRequired(
            votingVerifierConfig.governanceAddress,
            `${VERIFIER_CONTRACT_NAME}[${chainName}].governanceAddress`,
            'string',
        );
        this.validateRequired(votingVerifierConfig.serviceName, `${VERIFIER_CONTRACT_NAME}[${chainName}].serviceName`, 'string');
        this.validateRequired(
            votingVerifierConfig.sourceGatewayAddress,
            `${VERIFIER_CONTRACT_NAME}[${chainName}].sourceGatewayAddress`,
            'string',
        );
        this.validateThreshold(votingVerifierConfig.votingThreshold, `${VERIFIER_CONTRACT_NAME}[${chainName}].votingThreshold`);
        this.validateRequired(votingVerifierConfig.blockExpiry, `${VERIFIER_CONTRACT_NAME}[${chainName}].blockExpiry`, 'number');
        this.validateRequired(
            votingVerifierConfig.confirmationHeight,
            `${VERIFIER_CONTRACT_NAME}[${chainName}].confirmationHeight`,
            'number',
        );
        this.validateRequired(votingVerifierConfig.msgIdFormat, `${VERIFIER_CONTRACT_NAME}[${chainName}].msgIdFormat`, 'string');
        this.validateRequired(votingVerifierConfig.addressFormat, `${VERIFIER_CONTRACT_NAME}[${chainName}].addressFormat`, 'string');

        return votingVerifierConfig;
    }

    public getGatewayContract(chainName: string): GatewayChainConfig {
        const gatewayConfig = this.getContractConfigByChain(GATEWAY_CONTRACT_NAME, chainName) as GatewayChainConfig;

        return gatewayConfig;
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
