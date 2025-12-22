import { StdFee } from '@cosmjs/stargate';
import { Command, Option } from 'commander';

import { printInfo, validateParameters } from '../common';
import { ConfigManager } from '../common/config';
import { addCoreOptions } from './cli-utils';
import { ClientManager, Options } from './processor';
import { mainProcessor } from './processor';
import { confirmProposalSubmission, submitProposalAndPrint } from './proposal-utils';
import { encodeAddIBCChain, encodeChainStatusRequest } from './utils';

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
        // TODO: Implement direct execution with custom registry
        // Direct execution requires registering custom Axelar protobuf types
        // (e.g., ActivateChainRequest, DeactivateChainRequest) in the client's registry.
        throw new Error('Direct execution is not yet supported for core operations. Please submit as a governance proposal.');
    }

    const title = options.title || defaultTitle;
    const description = options.description || defaultDescription || defaultTitle;
    validateParameters({ isNonEmptyString: { title, description } });

    if (!confirmProposalSubmission(options, messages)) {
        return;
    }

    await submitProposalAndPrint(client, config, { ...options, title, description }, messages, fee);
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
    const message = encodeChainStatusRequest(args, requestType);

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


const addIBCChain = (
    client: ClientManager,
    config: ConfigManager,
    options: CoreCommandOptions,
    args: string[],
    fee?: string | StdFee,
) => executeCoreOperation(client, config, options, [encodeAddIBCChain(args)], fee);

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

    const addIBCChainCmd = program
        .command('add-ibc-chain')
        .description('Add an IBC chain')
        .argument('<chainName>', 'chain name to add')
        .argument('<chainPrefix>', 'chain prefix to add')
        .argument('<ibcPath>', 'IBC path to add')
        .action((cosmosChain, addrPrefix, ibcPath, options) => mainProcessor(addIBCChain, options, [cosmosChain, addrPrefix, ibcPath]));

    addCoreOptions(addIBCChainCmd);

    program.parse();
};

if (require.main === module) {
    programHandler();
}

export { activateChain, deactivateChain };
