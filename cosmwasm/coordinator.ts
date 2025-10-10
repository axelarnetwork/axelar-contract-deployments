import { calculateDomainSeparator, isKeccak256Hash, printError, printInfo } from '../common';
import { ConfigManager } from '../common/config';
import { getSalt } from './utils';

const getGatewayContractForChain = (chainName: string): string => {
    const chainGatewayMapping: Record<string, string> = {
        solana: 'SolanaGateway',
        stacks: 'StacksGateway',
    };
    return chainGatewayMapping[chainName] || 'Gateway';
};

const getVerifierContractForChain = (chainName: string): string => {
    const chainVerifierMapping: Record<string, string> = {
        solana: 'SolanaVotingVerifier',
        stacks: 'StacksVotingVerifier',
    };
    return chainVerifierMapping[chainName] || 'VotingVerifier';
};

const getProverContractForChain = (chainName: string): string => {
    const chainProverMapping: Record<string, string> = {
        solana: 'SolanaMultisigProver',
        stacks: 'StacksMultisigProver',
    };
    return chainProverMapping[chainName] || 'MultisigProver';
};

export interface GatewayParams {
    code_id: number;
    label: string;
    msg: null;
    contract_admin: string;
}

export interface VerifierParams {
    code_id: number;
    label: string;
    msg: {
        governance_address: string;
        service_name: string;
        source_gateway_address: string;
        voting_threshold: [string, string];
        block_expiry: string;
        confirmation_height: number;
        source_chain: string;
        rewards_address: string;
        msg_id_format: string;
        address_format: string;
    };
    contract_admin: string;
}

export interface ProverParams {
    code_id: number;
    label: string;
    msg: {
        governance_address: string;
        admin_address: string;
        multisig_address: string;
        signing_threshold: [string, string];
        service_name: string;
        chain_name: string;
        verifier_set_diff_threshold: number;
        encoder: string;
        key_type: string;
        domain_separator: string;
    };
    contract_admin: string;
}

export interface InstantiateChainContractsMsg {
    instantiate_chain_contracts: {
        deployment_name: string;
        salt: string;
        params: {
            manual: {
                gateway: GatewayParams;
                verifier: VerifierParams;
                prover: ProverParams;
            };
        };
    };
}

export interface VotingVerifierChainConfig {
    governanceAddress?: string;
    serviceName?: string;
    rewardsAddress?: string;
    sourceGatewayAddress?: string;
    votingThreshold?: [string, string];
    blockExpiry?: string | number;
    confirmationHeight?: number;
    msgIdFormat?: string;
    addressFormat?: string;
    deploymentName?: string;
    proposalId?: string;
    contractAdmin?: string;
    codeId: number;
    address?: string;
}

export interface MultisigProverChainConfig {
    encoder?: string;
    keyType?: string;
    domainSeparator?: string;
    adminAddress?: string;
    multisigAddress?: string;
    verifierSetDiffThreshold?: number;
    signingThreshold?: [string, string];
    deploymentName?: string;
    proposalId?: string;
    contractAdmin?: string;
    codeId: number;
    address?: string;
}

export interface GatewayChainConfig {
    deploymentName?: string;
    proposalId?: string;
    salt?: string;
    contractAdmin?: string;
    codeId: number;
    address?: string;
}

export class CoordinatorManager {
    public configManager: ConfigManager;

    constructor(configManager: ConfigManager) {
        this.configManager = configManager;
    }

    public constructExecuteMessage(chainName: string, salt: string, admin: string): InstantiateChainContractsMsg {
        try {
            const chainConfig = this.configManager.getChainConfig(chainName);
            const rewardsConfig = this.configManager.getContractConfig('Rewards');
            const multisigConfig = this.configManager.getContractConfig('Multisig');
            const routerConfig = this.configManager.getContractConfig('Router');

            const validateRequired = <T>(value: T | undefined | null, configPath: string): T => {
                if (value === undefined || value === null || (typeof value === 'string' && value.trim() === '')) {
                    throw new Error(`Missing required configuration for chain ${chainName}. Please configure it in ${configPath}.`);
                }
                return value;
            };

            const validateThreshold = (value: [string, string] | undefined | null, configPath: string): [string, string] => {
                if (!value || !Array.isArray(value) || value.length !== 2) {
                    throw new Error(
                        `Missing or invalid threshold configuration for chain ${chainName}. Please configure it in ${configPath} as [numerator, denominator].`,
                    );
                } else if (Number(value[0]) > Number(value[1])) {
                    throw new Error(
                        `Invalid threshold configuration for chain ${chainName}. Numerator must not be greater than denominator.`,
                    );
                }
                return value;
            };

            const gatewayContractName = getGatewayContractForChain(chainName);
            const verifierContractName = getVerifierContractForChain(chainName);
            const proverContractName = getProverContractForChain(chainName);

            const votingVerifierConfig = this.configManager.getContractConfigByChain(
                verifierContractName,
                chainName,
            ) as VotingVerifierChainConfig;
            const multisigProverConfig = this.configManager.getContractConfigByChain(
                proverContractName,
                chainName,
            ) as MultisigProverChainConfig;
            const gatewayConfig = this.configManager.getContractConfigByChain(gatewayContractName, chainName) as GatewayChainConfig;

            const gatewayCodeId: number = validateRequired(gatewayConfig.codeId, `${gatewayContractName}.codeId`);
            const verifierCodeId: number = validateRequired(votingVerifierConfig.codeId, `${verifierContractName}.codeId`);
            const proverCodeId: number = validateRequired(multisigProverConfig.codeId, `${proverContractName}.codeId`);
            const deploymentName = this.generateDeploymentName(chainName, `${gatewayCodeId}-${verifierCodeId}-${proverCodeId}`);

            const governanceAddress = validateRequired(
                votingVerifierConfig.governanceAddress,
                `${verifierContractName}[${chainName}].governanceAddress`,
            );
            const serviceName = validateRequired(votingVerifierConfig.serviceName, `${verifierContractName}[${chainName}].serviceName`);
            const rewardsAddress = validateRequired(rewardsConfig.address, `Rewards.address`);
            const sourceGatewayAddress = validateRequired(
                votingVerifierConfig.sourceGatewayAddress,
                `${verifierContractName}[${chainName}].sourceGatewayAddress`,
            );
            const votingThreshold = validateThreshold(
                votingVerifierConfig.votingThreshold,
                `${verifierContractName}[${chainName}].votingThreshold`,
            );
            const blockExpiry = validateRequired(votingVerifierConfig.blockExpiry, `${verifierContractName}[${chainName}].blockExpiry`);
            const confirmationHeight = validateRequired(
                votingVerifierConfig.confirmationHeight,
                `${verifierContractName}[${chainName}].confirmationHeight`,
            );
            const msgIdFormat = validateRequired(votingVerifierConfig.msgIdFormat, `${verifierContractName}[${chainName}].msgIdFormat`);
            const addressFormat = validateRequired(
                votingVerifierConfig.addressFormat,
                `${verifierContractName}[${chainName}].addressFormat`,
            );
            const encoder = validateRequired(multisigProverConfig.encoder, `${proverContractName}[${chainName}].encoder`);
            const keyType = validateRequired(multisigProverConfig.keyType, `${proverContractName}[${chainName}].keyType`);

            const routerAddress = validateRequired(routerConfig.address, `Router.address`);
            const domainSeparator = calculateDomainSeparator(chainName, routerAddress, this.configManager.axelar.chainId);
            if (!isKeccak256Hash(domainSeparator)) {
                throw new Error(`Invalid ${proverContractName}[${chainName}].domainSeparator in axelar info`);
            }
            multisigProverConfig.domainSeparator = domainSeparator;

            const verifierContractAdminAddress = admin;
            const multisigContractAdminAddress = admin;
            const gatewayContractAdminAddress = admin;
            votingVerifierConfig.contractAdmin = verifierContractAdminAddress;
            multisigProverConfig.contractAdmin = multisigContractAdminAddress;
            gatewayConfig.contractAdmin = gatewayContractAdminAddress;

            const multisigAdminAddress = validateRequired(
                multisigProverConfig.adminAddress,
                `${proverContractName}[${chainName}].adminAddress`,
            );
            const multisigAddress = validateRequired(multisigConfig.address, `Multisig.address`);
            const verifierSetDiffThreshold = validateRequired(
                multisigProverConfig.verifierSetDiffThreshold,
                `${proverContractName}[${chainName}].verifierSetDiffThreshold`,
            );
            const signingThreshold = validateThreshold(
                multisigProverConfig.signingThreshold,
                `${proverContractName}[${chainName}].signingThreshold`,
            );
            const validSalt = validateRequired(salt, 'CLI option --salt');
            const saltUint8Array = getSalt(validSalt, 'Coordinator', chainName);

            printInfo(`Code IDs - Gateway: ${gatewayCodeId}, Verifier: ${verifierCodeId}, Prover: ${proverCodeId}`);

            return {
                instantiate_chain_contracts: {
                    deployment_name: deploymentName,
                    salt: Buffer.from(saltUint8Array).toString('base64'),
                    params: {
                        manual: {
                            gateway: {
                                code_id: gatewayCodeId,
                                label: `${gatewayContractName}-${chainName}`,
                                msg: null,
                                contract_admin: gatewayContractAdminAddress,
                            },
                            verifier: {
                                code_id: verifierCodeId,
                                label: `${verifierContractName}-${chainName}`,
                                msg: {
                                    governance_address: governanceAddress,
                                    service_name: serviceName,
                                    source_gateway_address: sourceGatewayAddress,
                                    voting_threshold: [votingThreshold[0].toString(), votingThreshold[1].toString()],
                                    block_expiry: String(blockExpiry),
                                    confirmation_height: confirmationHeight,
                                    source_chain: chainConfig.axelarId,
                                    rewards_address: rewardsAddress,
                                    msg_id_format: msgIdFormat,
                                    address_format: addressFormat,
                                },
                                contract_admin: verifierContractAdminAddress,
                            },
                            prover: {
                                code_id: proverCodeId,
                                label: `${proverContractName}-${chainName}`,
                                msg: {
                                    governance_address: governanceAddress,
                                    admin_address: multisigAdminAddress,
                                    multisig_address: multisigAddress,
                                    signing_threshold: [signingThreshold[0].toString(), signingThreshold[1].toString()],
                                    service_name: serviceName,
                                    chain_name: chainConfig.axelarId,
                                    verifier_set_diff_threshold: verifierSetDiffThreshold,
                                    encoder: encoder,
                                    key_type: keyType,
                                    domain_separator: domainSeparator.replace('0x', ''),
                                },
                                contract_admin: multisigContractAdminAddress,
                            },
                        },
                    },
                },
            };
        } catch (error) {
            printError(`Error constructing message: ${error}`);
            throw error;
        }
    }

    private generateDeploymentName(chainName: string, codeId: string): string {
        return `${chainName}-${codeId}`;
    }
}
