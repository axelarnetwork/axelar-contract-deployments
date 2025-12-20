import { StdFee } from '@cosmjs/stargate';
import { Command, Option } from 'commander';

import { printInfo, validateParameters } from '../common';
import { ConfigManager } from '../common/config';
import { addCoreOptions } from './cli-utils';
import { ClientManager, Options } from './processor';
import { mainProcessor } from './processor';
import { confirmProposalSubmission, submitProposalAndPrint } from './proposal-utils';
import { encodeChainStatusRequest, getNexusProtoType, signAndBroadcastWithRetry } from './utils';

interface CoreCommandOptions extends Options {
    yes?: boolean;
    title?: string;
    description?: string;
    direct?: boolean;
    [key: string]: unknown;
}

const executeCoreOperation = async (
    client: ClientManager,
    config: ConfigManager,
    options: CoreCommandOptions,
    messages: object[],
    fee?: string | StdFee,
    defaultTitle?: string,
    defaultDescription?: string,
): Promise<void> => {
    if (options.direct) {
        const [account] = client.accounts;

        printInfo('Executing directly', `${messages.length} message(s)`);
        await signAndBroadcastWithRetry(client, account.address, messages, fee);
        printInfo('Transaction successful');
    } else {
        const title = options.title || defaultTitle;
        const description = options.description || defaultDescription || defaultTitle;
        validateParameters({ isNonEmptyString: { title, description } });

        if (!confirmProposalSubmission(options, messages)) {
            return;
        }

        await submitProposalAndPrint(client, config, { ...options, title, description }, messages, fee);
    }
};

const nexusChainState = async (
    action: 'activate' | 'deactivate',
    client: ClientManager,
    config: ConfigManager,
    options: CoreCommandOptions,
    args: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const requestType = action === 'activate' ? 'ActivateChainRequest' : 'DeactivateChainRequest';

    let message: object;

    if (options.direct) {
        const [account] = client.accounts;

        const RequestType = getNexusProtoType(requestType);
        const request = RequestType.create({
            sender: account.address,
            chains: args,
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
        message = encodeChainStatusRequest(args, requestType);
    }

    const actionText = action.charAt(0).toUpperCase() + action.slice(1);
    const defaultTitle = `${actionText} ${args.join(', ')} on Nexus`;

    return executeCoreOperation(client, config, options, [message], fee, defaultTitle);
};

const activateChain = (client: ClientManager, config: ConfigManager, options: CoreCommandOptions, args: string[], fee?: string | StdFee) =>
    nexusChainState('activate', client, config, options, args, fee);

const deactivateChain = (
    client: ClientManager,
    config: ConfigManager,
    options: CoreCommandOptions,
    args: string[],
    fee?: string | StdFee,
) => nexusChainState('deactivate', client, config, options, args, fee);

const programHandler = () => {
    const program = new Command();

    program.name('core').description('Execute core Axelar protocol operations');

    const activateChainCmd = program
        .command('activate-chain')
        .description('Activate chain(s) on Nexus module')
        .argument('<chains...>', 'chain name(s) to activate')
        .action((chains, options) => mainProcessor(activateChain, options, chains));

    addCoreOptions(activateChainCmd);

    const deactivateChainCmd = program
        .command('deactivate-chain')
        .description('Deactivate chain(s) on Nexus module')
        .argument('<chains...>', 'chain name(s) to deactivate')
        .action((chains, options) => mainProcessor(deactivateChain, options, chains));

    addCoreOptions(deactivateChainCmd);

    program.parse();
};

if (require.main === module) {
    programHandler();
}

export { activateChain, deactivateChain };
