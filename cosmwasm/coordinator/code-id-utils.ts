import type { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';

import { printError, printInfo } from '../../common';
import { fetchCodeIdFromCodeHash } from '../utils';
import type { ConfigManager } from './config';

export class CodeIdUtils {
    public configManager: ConfigManager;

    constructor(configManager: ConfigManager) {
        this.configManager = configManager;
    }

    /**
     * Fetches and updates code IDs from proposals for all contracts that need them
     */
    public async fetchAndUpdateCodeIdsFromProposals(client: SigningCosmWasmClient, contractsToUpdate: string[]): Promise<void> {
        printInfo('Fetching and updating code IDs from proposals...');

        for (const contractName of contractsToUpdate) {
            try {
                const contractConfig = this.configManager.getContractConfig(contractName);

                if (contractConfig.storeCodeProposalId && contractConfig.storeCodeProposalCodeHash) {
                    printInfo(`Found proposal data for ${contractName}, fetching latest code ID from chain...`);

                    const contractBaseConfig = {
                        storeCodeProposalCodeHash: contractConfig.storeCodeProposalCodeHash,
                    };

                    try {
                        const codeId = await fetchCodeIdFromCodeHash(client, contractBaseConfig);
                        printInfo(`Successfully fetched code ID ${codeId} for ${contractName} from chain`);

                        this.configManager.updateContractCodeId(contractName, codeId);
                        printInfo(`Updated ${contractName} code ID in config: ${codeId}`);
                    } catch (error) {
                        printInfo(`Failed to fetch code ID for ${contractName} from chain: ${(error as Error).message}`);
                        if (contractConfig.codeId) {
                            printInfo(`Using existing code ID from config as fallback: ${contractConfig.codeId}`);
                            this.configManager.updateContractCodeId(contractName, contractConfig.codeId);
                        }
                    }
                }
            } catch (error) {
                printError(`Error processing ${contractName}: ${(error as Error).message}`);
            }
        }
    }
}
