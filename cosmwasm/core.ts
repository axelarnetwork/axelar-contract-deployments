import { StdFee } from '@cosmjs/stargate';
import { Command, Option } from 'commander';

import { printInfo, validateParameters } from '../common';
import { ConfigManager } from '../common/config';
import { addCoreOptions } from './cli-utils';
import { ClientManager, Options } from './processor';
import { mainProcessor } from './processor';
import { confirmProposalSubmission, submitProposalAndPrint } from './proposal-utils';
import { GOVERNANCE_MODULE_ADDRESS, encodeChainStatusRequest, getNexusProtoType, signAndBroadcastWithRetry } from './utils';

interface CoreCommandOptions extends Options {
    yes?: boolean;
    title?: string;
    description?: string;
    chains?: string[];
    action?: 'activate' | 'deactivate';
    direct?: boolean;
    [key: string]: unknown;
}

/**
 * Execute a core protocol operation either directly (via EOA) or through governance proposal.
 * Default is governance proposal unless --direct flag is set.
 */
const executeCoreOperation = async (
    client: ClientManager,
    config: ConfigManager,
    options: CoreCommandOptions,
    messages: object[],
    fee?: string | StdFee,
): Promise<void> => {
    if (options.direct) {
        // Direct execution by EOA
        const [account] = (client as any).accounts || (await (client as any).signer.getAccounts());

        printInfo('Executing directly', `${messages.length} message(s)`);
        await signAndBroadcastWithRetry(client, account.address, messages, fee, '');
        printInfo('Transaction successful');
    } else {
        // Governance proposal (default)
        validateParameters({ isNonEmptyString: { title: options.title, description: options.description } });

        if (!confirmProposalSubmission(options, messages)) {
            return;
        }

        return submitProposalAndPrint(client, config, options, messages, fee);
    }
};

const nexusChainState = async (
    client: ClientManager,
    config: ConfigManager,
    options: CoreCommandOptions,
    _args: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const { chains, action, direct } = options;
    const requestType = action === 'activate' ? 'ActivateChainRequest' : 'DeactivateChainRequest';

    let message: object;

    if (direct) {
        // For direct execution, encode with EOA as sender
        const [account] = (client as any).accounts || (await (client as any).signer.getAccounts());

        const RequestType = getNexusProtoType(requestType);
        const request = RequestType.create({
            sender: account.address,
            chains: chains,
        });

        const errMsg = RequestType.verify(request);
        if (errMsg) {
            throw new Error(`Invalid ${requestType}: ${errMsg}`);
        }

        message = {
            typeUrl: `/axelar.nexus.v1beta1.${requestType}`,
            value: RequestType.encode(request).finish(),
        };
    } else {
        // For governance, encode with GOVERNANCE_MODULE_ADDRESS as sender
        message = encodeChainStatusRequest(chains, requestType);
    }

    return executeCoreOperation(client, config, options, [message], fee);
};

const programHandler = () => {
    const program = new Command();

    program.name('core').description('Execute core Axelar protocol operations');

    const nexusChainStateCmd = program
        .command('nexus-chain-state')
        .description(
            'Activate or deactivate chain(s) on Nexus module (via governance proposal by default, or direct execution with --direct)',
        )
        .requiredOption('--chains <chains...>', 'Chain name(s) to activate/deactivate')
        .addOption(new Option('--action <action>', 'Action to perform').choices(['activate', 'deactivate']).makeOptionMandatory())
        .option('-t, --title <title>', 'Proposal title (required for governance proposals)')
        .option('-d, --description <description>', 'Proposal description (required for governance proposals)')
        .action((options) => mainProcessor(nexusChainState, options));

    addCoreOptions(nexusChainStateCmd);

    program.parse();
};

if (require.main === module) {
    programHandler();
}

export { nexusChainState };
