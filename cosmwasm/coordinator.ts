import { calculateDomainSeparator, isKeccak256Hash, printError, printInfo } from '../common';
import { ConfigManager, GATEWAY_CONTRACT_NAME, VERIFIER_CONTRACT_NAME } from '../common/config';
import { getSalt } from './utils';

export interface RegisterDeploymentMsg {
    register_deployment: {
        deployment_name: string;
    };
}

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
        chain_codec_address: string;
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
        key_type: string;
        domain_separator: string;
        notify_signing_session: boolean;
        expect_full_message_payloads: boolean;
        sig_verifier_address: string | null;
        chain_codec_address: string;
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
            const multisigAddress = this.configManager.validateRequired(multisigConfig.address, `Multisig.address`);

            const proverContractName = this.configManager.getMultisigProverContractForChainType(chainConfig.chainType);
            const verifierContractName = this.configManager.getVotingVerifierContractForChainType(chainConfig.chainType);
            const gatewayContractName = this.configManager.getGatewayContractForChainType(chainConfig.chainType);

            const votingVerifierConfig = this.configManager.getVotingVerifierContract(chainName);
            const multisigProverConfig = this.configManager.getMultisigProverContract(chainName);
            const gatewayConfig = this.configManager.getGatewayContract(chainName);
            const gatewayCodeId = this.configManager.validateRequired(gatewayConfig.codeId, `Gateway.${chainName}.codeId`);
            const verifierCodeId = this.configManager.validateRequired(votingVerifierConfig.codeId, `VotingVerifier.${chainName}.codeId`);
            const proverCodeId = this.configManager.validateRequired(
                multisigProverConfig.codeId,
                `${proverContractName}.${chainName}.codeId`,
            );
            const deploymentName = this.generateDeploymentName(chainName, `${gatewayCodeId}-${verifierCodeId}-${proverCodeId}`);
            const rewardsAddress = this.configManager.validateRequired(rewardsConfig.address, `Rewards.address`);
            const routerAddress = this.configManager.validateRequired(routerConfig.address, `Router.address`);
            const domainSeparator = calculateDomainSeparator(chainName, routerAddress, this.configManager.axelar.chainId);
            if (!isKeccak256Hash(domainSeparator)) {
                throw new Error(`Invalid ${proverContractName}[${chainName}].domainSeparator in axelar info`);
            }
            multisigProverConfig.domainSeparator = domainSeparator;
            votingVerifierConfig.contractAdmin = admin;
            multisigProverConfig.contractAdmin = admin;
            gatewayConfig.contractAdmin = admin;
            const validSalt = this.configManager.validateRequired(salt, 'CLI option --salt');
            const saltUint8Array = getSalt(validSalt, 'Coordinator', chainName);

            printInfo(`Code IDs - Gateway: ${gatewayCodeId}, Verifier: ${verifierCodeId}, Prover: ${proverCodeId}`);

            // Note: These are required for standard chains, but not for XRPL chains
            this.configManager.validateRequired(
                votingVerifierConfig.sourceGatewayAddress,
                `${verifierContractName}[${chainName}].sourceGatewayAddress`,
            );
            this.configManager.validateRequired(votingVerifierConfig.msgIdFormat, `${verifierContractName}[${chainName}].msgIdFormat`);
            this.configManager.validateRequired(multisigProverConfig.keyType, `${proverContractName}[${chainName}].keyType`);

            // Get chain codec address
            const chainCodecAddress = this.configManager.getChainCodecAddress(chainConfig.chainType);

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
                                contract_admin: gatewayConfig.contractAdmin,
                            },
                            verifier: {
                                code_id: verifierCodeId,
                                label: `${verifierContractName}-${chainName}`,
                                msg: {
                                    governance_address: votingVerifierConfig.governanceAddress,
                                    service_name: votingVerifierConfig.serviceName,
                                    source_gateway_address: votingVerifierConfig.sourceGatewayAddress,
                                    voting_threshold: [
                                        String(votingVerifierConfig.votingThreshold[0]),
                                        String(votingVerifierConfig.votingThreshold[1]),
                                    ],
                                    block_expiry: String(votingVerifierConfig.blockExpiry),
                                    confirmation_height: votingVerifierConfig.confirmationHeight,
                                    source_chain: chainConfig.axelarId,
                                    rewards_address: rewardsAddress,
                                    msg_id_format: votingVerifierConfig.msgIdFormat,
                                    chain_codec_address: chainCodecAddress,
                                },
                                contract_admin: votingVerifierConfig.contractAdmin,
                            },
                            prover: {
                                code_id: proverCodeId,
                                label: `${proverContractName}-${chainName}`,
                                msg: {
                                    governance_address: multisigProverConfig.governanceAddress,
                                    admin_address: multisigProverConfig.adminAddress,
                                    multisig_address: multisigAddress,
                                    signing_threshold: [
                                        String(multisigProverConfig.signingThreshold[0]),
                                        String(multisigProverConfig.signingThreshold[1]),
                                    ],
                                    service_name: votingVerifierConfig.serviceName,
                                    chain_name: chainConfig.axelarId,
                                    verifier_set_diff_threshold: multisigProverConfig.verifierSetDiffThreshold,
                                    key_type: multisigProverConfig.keyType,
                                    domain_separator: domainSeparator.replace('0x', ''),
                                    notify_signing_session: Boolean(multisigProverConfig.notifySigningSession) || false,
                                    expect_full_message_payloads: Boolean(multisigProverConfig.expectFullMessagePayloads) || false,
                                    sig_verifier_address: multisigProverConfig.sigVerifierAddress || null,
                                    chain_codec_address: chainCodecAddress,
                                },
                                contract_admin: multisigProverConfig.contractAdmin,
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

    public constructRegisterDeploymentMessage(chainName: string): RegisterDeploymentMsg {
        const coordinatorConfig = this.configManager.getContractConfig('Coordinator');
        const deploymentName = coordinatorConfig.deployments?.[chainName]?.deploymentName;
        if (!deploymentName) {
            throw new Error(`Deployment name not found for chain ${chainName}`);
        }
        return {
            register_deployment: {
                deployment_name: deploymentName,
            },
        };
    }

    private generateDeploymentName(chainName: string, codeId: string): string {
        return `${chainName}-${codeId}`;
    }
}
