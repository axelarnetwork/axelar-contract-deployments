import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { AccountData, DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';

import { prepareClient, prepareWallet, submitProposal } from '../utils';
import { ConfigManager } from './config';

export class ProposalManager {
    private configManager: ConfigManager;

    constructor(configManager: ConfigManager) {
        this.configManager = configManager;
    }

    public async submitProposal<T>(proposal: T, mnemonic: string, deposit: string): Promise<string> {
        const wallet = await this.getWallet(mnemonic);
        const client = await this.getClient(mnemonic);
        return await submitProposal(client, wallet, this.configManager.getFullConfig(), { deposit }, proposal);
    }

    public async getClient(mnemonic: string): Promise<SigningCosmWasmClient> {
        const wallet = await this.getWallet(mnemonic);
        return await prepareClient(this.configManager.getFullConfig() as { axelar: { rpc: string; gasPrice: string } }, wallet);
    }

    public async getWallet(mnemonic: string): Promise<DirectSecp256k1HdWallet> {
        return await prepareWallet({ mnemonic });
    }

    public async getAccounts(mnemonic: string): Promise<readonly AccountData[]> {
        const wallet = await this.getWallet(mnemonic);
        return await wallet.getAccounts();
    }
}
