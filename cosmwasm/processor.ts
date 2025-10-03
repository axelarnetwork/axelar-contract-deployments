import { CosmWasmClient, SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { AccountData, DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { GasPrice, StdFee } from '@cosmjs/stargate';

import { ConfigManager } from '../common/config';

export type Options = {
    env: string;
    contractName: string;
    chainName: string;
    mnemonic: string;
    runAs?: string;
    deposit?: string;
    instantiateAddresses?: string[];
};

type ProcessorFn = (
    client: ClientManager,
    config: ConfigManager,
    options: Options,
    args?: string[],
    fee?: string | StdFee,
) => Promise<void>;
type ProcessorQueryFn = (
    client: CosmWasmClient,
    config: ConfigManager,
    options: Options,
    args?: string[],
    fee?: string | StdFee,
) => Promise<void>;

export interface ClientManager extends SigningCosmWasmClient {
    accounts: readonly AccountData[];
}

function prepareProcessor(options: Options): { configManager: ConfigManager; fee: string | StdFee } {
    const { runAs, deposit, instantiateAddresses, env } = options;
    const configManager = new ConfigManager(env);
    const fee = configManager.getFee();

    configManager.initContractConfig(options.contractName, options.chainName);

    options.runAs =
        runAs || configManager.axelar?.contracts?.ServiceRegistry?.governanceAccount || 'axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj';
    options.deposit = deposit || configManager.getProposalDepositAmount();
    options.instantiateAddresses = instantiateAddresses || configManager.getProposalInstantiateAddresses();

    return { configManager, fee };
}

export async function mainProcessor(processorFn: ProcessorFn, options: Options, args?: string[]) {
    const { configManager, fee } = prepareProcessor(options);

    if (!options.mnemonic) {
        throw new Error('Mnemonic is required');
    }

    const client = await prepareClient(options.mnemonic, configManager.axelar.rpc, GasPrice.fromString(configManager.axelar.gasPrice));

    await processorFn(client, configManager, options, args, fee);
    configManager.saveConfig();
}

export async function mainQueryProcessor(processorQueryFn: ProcessorQueryFn, options: Options, args?: string[]) {
    const { configManager, fee } = prepareProcessor(options);
    const client = await CosmWasmClient.connect(configManager.axelar.rpc);
    await processorQueryFn(client, configManager, options, args, fee);
    configManager.saveConfig();
}

async function prepareClient(mnemonic: string, rpc: string, gasPrice: GasPrice): Promise<ClientManager> {
    try {
        const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, { prefix: 'axelar' });
        const clientManager = (await SigningCosmWasmClient.connectWithSigner(rpc, wallet, { gasPrice: gasPrice })) as ClientManager;
        clientManager.accounts = await wallet.getAccounts();
        return clientManager;
    } catch (error) {
        throw new Error(`Failed to prepare client: ${error instanceof Error ? error.message : String(error)}`);
    }
}
