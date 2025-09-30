import { calculateDomainSeparator, isKeccak256Hash, printError, printInfo } from '../common';
import { ConfigManager } from '../common/config';
import { getSalt } from './utils';

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
}

export interface GatewayChainConfig {
    deploymentName?: string;
    proposalId?: string;
    salt?: string;
    contractAdmin?: string;
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
            const votingVerifierConfig =
                (this.configManager.getContractConfig('VotingVerifier')[chainName] as VotingVerifierChainConfig) || {};
            const multisigProverConfig =
                (this.configManager.getContractConfig('MultisigProver')[chainName] as MultisigProverChainConfig) || {};
            const gatewayConfig = (this.configManager.getContractConfig('Gateway')[chainName] as GatewayChainConfig) || {};

            const validateRequired = <T>(value: T | undefined | null, configPath: string): T => {
                if (value === undefined || value === null || (Array.isArray(value) && value.length === 0)) {
                    throw new Error(`Missing required configuration for chain ${chainName}. Please configure it in ${configPath}.`);
                }
                return value;
            };

            const validateThreshold = (value: [string, string] | undefined | null, configPath: string): [string, string] => {
                if (!value || !Array.isArray(value) || value.length !== 2) {
                    throw new Error(
                        `Missing or invalid threshold configuration for chain ${chainName}. Please configure it in ${configPath} as [numerator, denominator].`,
                    );
                }
                return value;
            };

            const gatewayCodeId: number = validateRequired(this.configManager.getContractConfig('Gateway').codeId, `Gateway.codeId`);
            const verifierCodeId: number = validateRequired(
                this.configManager.getContractConfig('VotingVerifier').codeId,
                `VotingVerifier.codeId`,
            );
            const proverCodeId: number = validateRequired(
                this.configManager.getContractConfig('MultisigProver').codeId,
                `MultisigProver.codeId`,
            );
            const deploymentName = this.generateDeploymentName(chainName, `${gatewayCodeId}-${verifierCodeId}-${proverCodeId}`);

            const governanceAddress = validateRequired(
                votingVerifierConfig.governanceAddress,
                `VotingVerifier[${chainName}].governanceAddress`,
            );
            const serviceName = validateRequired(votingVerifierConfig.serviceName, `VotingVerifier[${chainName}].serviceName`);
            const rewardsAddress = validateRequired(rewardsConfig.address, `Rewards[${chainName}].address`);
            const sourceGatewayAddress = validateRequired(
                votingVerifierConfig.sourceGatewayAddress,
                `VotingVerifier[${chainName}].sourceGatewayAddress`,
            );
            const votingThreshold = validateThreshold(votingVerifierConfig.votingThreshold, `VotingVerifier[${chainName}].votingThreshold`);
            const blockExpiry = validateRequired(votingVerifierConfig.blockExpiry, `VotingVerifier[${chainName}].blockExpiry`);
            const confirmationHeight = validateRequired(
                votingVerifierConfig.confirmationHeight,
                `VotingVerifier[${chainName}].confirmationHeight`,
            );
            const msgIdFormat = validateRequired(votingVerifierConfig.msgIdFormat, `VotingVerifier[${chainName}].msgIdFormat`);
            const addressFormat = validateRequired(votingVerifierConfig.addressFormat, `VotingVerifier[${chainName}].addressFormat`);
            const encoder = validateRequired(multisigProverConfig.encoder, `MultisigProver[${chainName}].encoder`);
            const keyType = validateRequired(multisigProverConfig.keyType, `MultisigProver[${chainName}].keyType`);

            const domainSeparator = calculateDomainSeparator(chainName, routerConfig.address, this.configManager.axelar.chainId);
            if (!isKeccak256Hash(domainSeparator)) {
                throw new Error(`Invalid MultisigProver[${chainName}].domainSeparator in axelar info`);
            }
            multisigProverConfig.domainSeparator = domainSeparator;

            const verifierContractAdminAddress = admin;
            const multisigContractAdminAddress = admin;
            const gatewayContractAdminAddress = admin;
            const multisigAdminAddress = validateRequired(multisigProverConfig.adminAddress, `MultisigProver[${chainName}].adminAddress`);
            const multisigAddress = validateRequired(multisigConfig.address, `Multisig[${chainName}].address`);
            const verifierSetDiffThreshold = validateRequired(
                multisigProverConfig.verifierSetDiffThreshold,
                `MultisigProver[${chainName}].verifierSetDiffThreshold`,
            );
            const signingThreshold = validateThreshold(
                multisigProverConfig.signingThreshold,
                `MultisigProver[${chainName}].signingThreshold`,
            );
            const validSalt = validateRequired(salt, 'CLI option --salt');
            const saltUint8Array = getSalt(validSalt, chainName, chainConfig.axelarId);

            printInfo(`Code IDs - Gateway: ${gatewayCodeId}, Verifier: ${verifierCodeId}, Prover: ${proverCodeId}`);

            return {
                instantiate_chain_contracts: {
                    deployment_name: deploymentName,
                    salt: Buffer.from(saltUint8Array).toString('base64'),
                    params: {
                        manual: {
                            gateway: {
                                code_id: gatewayCodeId,
                                label: `Gateway-${chainName}`,
                                msg: null,
                                contract_admin: gatewayContractAdminAddress,
                            },
                            verifier: {
                                code_id: verifierCodeId,
                                label: `Verifier-${chainName}`,
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
                                label: `Prover-${chainName}`,
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
