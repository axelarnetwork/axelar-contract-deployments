import { CosmWasmClient, SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { AccountData, DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { GasPrice, StdFee } from '@cosmjs/stargate';

import { ConfigManager, FullConfig } from '../common/config';

type Options = {
    env: string;
    contractName: string;
    chainName: string;
    mnemonic: string;
    runAs?: string;
    deposit?: string;
    instantiateAddresses?: string[];
};

type ProcessorFn = (client: ClientManager, config: FullConfig, options: Options, args?: string[], fee?: string | StdFee) => Promise<void>;
type ProcessorQueryFn = (
    client: CosmWasmClient,
    config: FullConfig,
    options: Options,
    args?: string[],
    fee?: string | StdFee,
) => Promise<void>;


function prepareProcessor(options: Options): { configManager: ConfigManager; fee: string | StdFee } {
    const { runAs, deposit, instantiateAddresses, env } = options;
    const configManager = new ConfigManager(env);
    const fee = configManager.getFee();

    options.runAs =
        runAs ||
        (env === 'devnet-amplifier' ? 'axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9' : 'axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj');
    options.deposit = deposit || configManager.getProposalDepositAmount();
    options.instantiateAddresses = instantiateAddresses || configManager.getProposalInstantiateAddresses();

    configManager.initContractConfig(options.contractName, options.chainName);

    return { configManager, fee };
}

export async function mainProcessor(processor: ProcessorFn, options: Options, args?: string[]) {
    const { configManager, fee } = prepareProcessor(options);

    if (!options.mnemonic) {
        throw new Error('Mnemonic is required');
    }

    const client = await ClientManager.prepareClient(
        options.mnemonic,
        configManager.getFullConfig().axelar.rpc,
        GasPrice.fromString(configManager.getFullConfig().axelar.gasPrice),
    );

    await processor(client, configManager.getFullConfig(), options, args, fee);
    configManager.saveConfig();
}

export async function mainQueryProcessor(processor: ProcessorQueryFn, options: Options, args?: string[]) {
    const { configManager, fee } = prepareProcessor(options);
    const client = await CosmWasmClient.connect(configManager.getFullConfig().axelar.rpc);
    await processor(client, configManager.getFullConfig(), options, args, fee);
    configManager.saveConfig();
}

export class ClientManager extends SigningCosmWasmClient {
    private wallet: DirectSecp256k1HdWallet;

    static async prepareClient(mnemonic: string, rpc: string, gasPrice: GasPrice): Promise<ClientManager> {
        try {
            const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, { prefix: 'axelar' });
            const signingClient = await SigningCosmWasmClient.connectWithSigner(rpc, wallet, { gasPrice: gasPrice });
            // Create a proper ClientManager instance by copying properties
            const clientManager = Object.create(ClientManager.prototype);
            Object.assign(clientManager, signingClient);
            clientManager.wallet = wallet;
            return clientManager;
        } catch (error) {
            throw new Error(`Failed to prepare client: ${error instanceof Error ? error.message : String(error)}`);
        }
    }

    public async getAccounts(): Promise<readonly AccountData[]> {
        return await this.wallet.getAccounts();
    }
}
