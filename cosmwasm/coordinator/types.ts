import type { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import type { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';

export const AMPLIFIER_CONTRACTS_TO_HANDLE = ['VotingVerifier', 'MultisigProver', 'Gateway'];

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

export interface RegisterDeploymentMsg {
    register_deployment: {
        deployment_name: string;
    };
}

export interface ProposalOptions {
    deposit?: string;
    yes?: boolean;
    runAs?: string;
    mnemonic: string;
    env: string;
}

export interface DeployContractsOptions extends ProposalOptions {
    artifactDir?: string;
    version?: string;
    deploymentName?: string;
}

export interface ConfigureChainOptions {
    serviceName?: string;
    rewardsAddress?: string;
    sourceGatewayAddress?: string;
    governanceAddress?: string;
    votingThreshold?: [string, string] | string;
    signingThreshold?: [string, string] | string;
    blockExpiry?: string;
    confirmationHeight?: number | string;
    msgIdFormat?: string;
    addressFormat?: string;
    verifierSetDiffThreshold?: number | string;
    encoder?: string;
    keyType?: string;
    domainSeparator?: string;
    chainName: string;
    contractAdmin: string;
    multisigAdmin: string;
    salt: string;
}

// eslint-disable-next-line @typescript-eslint/no-empty-object-type
export interface RegisterProtocolOptions extends ProposalOptions {}

export interface InstantiateChainOptions extends ProposalOptions {
    chainName: string;
}

export interface RegisterDeploymentOptions extends ProposalOptions {
    chainName: string;
}

export interface CoordinatorOptions
    extends DeployContractsOptions,
        ConfigureChainOptions,
        RegisterProtocolOptions,
        InstantiateChainOptions,
        RegisterDeploymentOptions {}

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

export interface WalletAndClient {
    wallet: DirectSecp256k1HdWallet;
    client: SigningCosmWasmClient;
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
