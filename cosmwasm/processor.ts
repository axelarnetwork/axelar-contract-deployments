import { CosmWasmClient, SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { AccountData, DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { GasPrice, StdFee } from '@cosmjs/stargate';

import { ConfigManager } from '../common/config';

export type Options = {
    env: string;
    contractName: string;
    chainName: string;
    mnemonic: string;
    deposit?: string;
    standardProposal?: boolean;
    instantiateAddresses?: string[];
    rpc?: string;
};

type ProcessorFn = (
    client: ClientManager,
    config: ConfigManager,
    options: Options,
    args?: string[],
    fee?: string | StdFee,
) => Promise<void>;
type ProcessorQueryFn = (client: CosmWasmClient, config: ConfigManager, options: Options, args?: string[]) => Promise<void>;

export interface ClientManager extends SigningCosmWasmClient {
    accounts: readonly AccountData[];
    wallet: DirectSecp256k1HdWallet;
}

function prepareQueryProcessor(options: Options): { configManager: ConfigManager } {
    const { env, contractName, chainName } = options;
    const configManager = new ConfigManager(env);

    if (contractName) {
        configManager.initContractConfig(contractName, chainName);
    }

    return { configManager };
}

function prepareProcessor(options: Options): { configManager: ConfigManager; fee: string | StdFee } {
    const { instantiateAddresses, env } = options;
    const configManager = new ConfigManager(env);
    const fee = configManager.getFee();

    options.instantiateAddresses = instantiateAddresses || configManager.proposalInstantiateAddresses();

    configManager.initContractConfig(options.contractName, options.chainName);

    return { configManager, fee };
}

export async function mainProcessor(processorFn: ProcessorFn, options: Options, args?: string[]) {
    const { rpc: axelarNode } = options;
    const { configManager, fee } = prepareProcessor(options);

    const axelarNodeFromConfig = configManager.axelar.rpc;

    if (axelarNode) {
        configManager.axelar.rpc = axelarNode;
    }

    if (!options.mnemonic) {
        throw new Error('Mnemonic is required');
    }

    const client = await prepareClient(options.mnemonic, configManager.axelar.rpc, GasPrice.fromString(configManager.axelar.gasPrice));

    await processorFn(client, configManager, options, args, fee);

    configManager.axelar.rpc = axelarNodeFromConfig;
    configManager.saveConfig();
}

export async function mainQueryProcessor(processorQueryFn: ProcessorQueryFn, options: Options, args?: string[]) {
    const { rpc: axelarNode } = options;
    const { configManager } = prepareQueryProcessor(options);
    const axelarNodeFromConfig = configManager.axelar.rpc;

    if (axelarNode) {
        configManager.axelar.rpc = axelarNode;
    }

    const client = await CosmWasmClient.connect(configManager.axelar.rpc);
    const res = await processorQueryFn(client, configManager, options, args);

    configManager.axelar.rpc = axelarNodeFromConfig;
    configManager.saveConfig();

    return res;
}

async function prepareClient(mnemonic: string, rpc: string, gasPrice: GasPrice): Promise<ClientManager> {
    try {
        const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, { prefix: 'axelar' });
        const clientManager = (await SigningCosmWasmClient.connectWithSigner(rpc, wallet, { gasPrice: gasPrice })) as ClientManager;
        clientManager.accounts = await wallet.getAccounts();
        clientManager.wallet = wallet;
        return clientManager;
    } catch (error) {
        throw new Error(`Failed to prepare client: ${error instanceof Error ? error.message : String(error)}`);
    }
}
