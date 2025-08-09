import type { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import type { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';

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

export interface ConfigFile {
    chains: {
        [chainName: string]: ChainConfig;
    };
}

export interface GatewayParams {
    code_id: number;
    label: string;
    msg: null;
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
}

export interface ProverParams {
    code_id: number;
    label: string;
    msg: {
        governance_address: string;
        multisig_address: string;
        signing_threshold: [string, string];
        service_name: string;
        chain_name: string;
        verifier_set_diff_threshold: number;
        encoder: string;
        key_type: string;
        domain_separator: string;
    };
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

export interface DeploymentOptions {
    artifactDir?: string;
    version?: string;
    salt?: string;
    deploymentName?: string;
}

export interface GovernanceOptions {
    governanceAddress?: string;
    deposit?: string;
    title?: string;
    description?: string;
    yes?: boolean;
}

export interface ContractConfigOptions {
    serviceName?: string;
    rewardsAddress?: string;
    sourceGatewayAddress?: string;
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
}

export interface CLIOptions {
    runAs?: string;
    mnemonic?: string;
    chainName?: string;
}

export interface CoordinatorOptions extends DeploymentOptions, GovernanceOptions, ContractConfigOptions, CLIOptions {}

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
    [key: string]: unknown;
}

export interface ContractInfo {
    gateway?: { address: string; codeId: number };
    verifier?: { address: string; codeId: number };
    prover?: { address: string; codeId: number };
}

export interface WalletAndClient {
    wallet: DirectSecp256k1HdWallet;
    client: SigningCosmWasmClient;
}
