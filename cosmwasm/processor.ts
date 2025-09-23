import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { StdFee } from '@cosmjs/stargate';

import { ConfigManager, FullConfig } from '../common/config';
import { prepareClient, prepareDummyWallet, prepareWallet } from './utils';

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
    client: SigningCosmWasmClient,
    wallet: DirectSecp256k1HdWallet,
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

    const wallet = options.mnemonic ? await prepareWallet(options) : await prepareDummyWallet();
    const client = await prepareClient(configManager.getFullConfig(), wallet);

    const fee = configManager.getFee();

    await processor(client, wallet, configManager.getFullConfig(), options, args, fee);

    configManager.saveConfig();
}
