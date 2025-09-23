import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
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

type ProcessorFn = (
    client: ClientManager,
    config: FullConfig,
    options: Options,
    args?: string[],
    fee?: string | StdFee,
) => Promise<void>;

export async function mainProcessor(processor: ProcessorFn, options: Options, args?: string[]) {
    const { runAs, deposit, instantiateAddresses, env } = options;
    const configManager = new ConfigManager(env);

    options.runAs =
        runAs ||
        (env === 'devnet-amplifier' ? 'axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9' : 'axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj');
    options.deposit = deposit || configManager.getProposalDepositAmount();
    options.instantiateAddresses = instantiateAddresses || configManager.getProposalInstantiateAddresses();

    configManager.initContractConfig(options.contractName, options.chainName);

    const client = await ClientManager.prepareClient(
        options.mnemonic,
        configManager.getFullConfig().axelar.rpc,
        GasPrice.fromString(configManager.getFullConfig().axelar.gasPrice),
    );
    const fee = configManager.getFee();

    await processor(client, configManager.getFullConfig(), options, args, fee);

    configManager.saveConfig();
}

export class ClientManager extends SigningCosmWasmClient {
    private mnemonic: string;

    static async prepareClient(mnemonic: string, rpc: string, gasPrice: GasPrice): Promise<ClientManager> {
        const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, { prefix: 'axelar' });
        const clientManager = (await SigningCosmWasmClient.connectWithSigner(rpc, wallet, { gasPrice: gasPrice })) as ClientManager;
        clientManager.mnemonic = mnemonic;
        return clientManager;
    }

    static async prepareQueryClient(rpc: string, gasPrice: GasPrice): Promise<ClientManager> {
        const dummyMnemonic = 'test test test test test test test test test test test junk';
        const clientManager = await ClientManager.prepareClient(dummyMnemonic, rpc, gasPrice);
        return clientManager;
    }

    public async getAccounts(): Promise<readonly AccountData[]> {
        const wallet = await DirectSecp256k1HdWallet.fromMnemonic(this.mnemonic, { prefix: 'axelar' });
        return await wallet.getAccounts();
    }
}
