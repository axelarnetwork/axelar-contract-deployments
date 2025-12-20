import { StdFee } from '@cosmjs/stargate';
import { Command, Option } from 'commander';

import { printInfo, prompt, validateParameters } from '../common';
import { ConfigManager } from '../common/config';
import { addAmplifierOptions } from './cli-utils';
import { ClientManager, Options } from './processor';
import { mainProcessor } from './processor';
import { encodeChainStatusRequest, submitProposal } from './utils';

interface CoreCommandOptions extends Options {
    yes?: boolean;
    title?: string;
    description?: string;
    chains?: string[];
    action?: 'activate' | 'deactivate';
    [key: string]: unknown;
}

const confirmProposalSubmission = (options: CoreCommandOptions, proposalData: object[]): boolean => {
    printInfo('Proposal messages', JSON.stringify(proposalData, null, 2));
    return !prompt(`Proceed with proposal submission?`, options.yes);
};

const callSubmitProposal = async (
    client: ClientManager,
    config: ConfigManager,
    options: CoreCommandOptions,
    proposal: object[],
    fee?: string | StdFee,
): Promise<void> => {
    const proposalId = await submitProposal(client, config, options, proposal, fee);
    printInfo('Proposal submitted', proposalId);
};

const nexusChainState = async (
    client: ClientManager,
    config: ConfigManager,
    options: CoreCommandOptions,
    _args: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const { chains, action, title, description } = options;

    validateParameters({ isNonEmptyString: { title, description } });

    const requestType = action === 'activate' ? 'ActivateChainRequest' : 'DeactivateChainRequest';
    const proposal = encodeChainStatusRequest(chains, requestType);

    if (!confirmProposalSubmission(options, [proposal])) {
        return;
    }

    return callSubmitProposal(client, config, options, [proposal], fee);
};

const programHandler = () => {
    const program = new Command();

    program.name('core').description('Execute core Axelar protocol operations');

    const nexusChainStateCmd = program
        .command('nexus-chain-state')
        .description('Submit a proposal to activate or deactivate chain(s) on Nexus module')
        .requiredOption('--chains <chains...>', 'Chain name(s) to activate/deactivate')
        .addOption(new Option('--action <action>', 'Action to perform').choices(['activate', 'deactivate']).makeOptionMandatory())
        .requiredOption('-t, --title <title>', 'Proposal title')
        .requiredOption('-d, --description <description>', 'Proposal description')
        .action((options) => mainProcessor(nexusChainState, options));

    addAmplifierOptions(nexusChainStateCmd, {
        proposalOptions: true,
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}

export { nexusChainState };
