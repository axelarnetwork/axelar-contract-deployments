import { StdFee } from '@cosmjs/stargate';
import {
    MsgExecuteContract,
    MsgInstantiateContract,
    MsgInstantiateContract2,
    MsgMigrateContract,
    MsgStoreAndInstantiateContract,
    MsgStoreCode,
    MsgUpdateInstantiateConfig,
} from 'cosmjs-types/cosmwasm/wasm/v1/tx';

import { getChainConfig, printInfo, prompt } from '../common';
import { ConfigManager } from '../common/config';
import { ClientManager } from './processor';
import {
    GOVERNANCE_MODULE_ADDRESS,
    encodeExecuteContract,
    encodeMigrate,
    encodeSubmitProposal,
    getAmplifierContractConfig,
    getCodeId,
    getNexusProtoType,
    signAndBroadcastWithRetry,
    toArray,
} from './utils';

interface ProposalOptions {
    yes?: boolean;
    [key: string]: unknown;
}

const printProposal = (proposalData: object[]): void => {
    proposalData.forEach((msg: unknown) => {
        const message = msg as { typeUrl: string; value: Uint8Array };
        const typeMap: Record<string, unknown> = {
            '/cosmwasm.wasm.v1.MsgStoreCode': MsgStoreCode,
            '/cosmwasm.wasm.v1.MsgExecuteContract': MsgExecuteContract,
            '/cosmwasm.wasm.v1.MsgInstantiateContract': MsgInstantiateContract,
            '/cosmwasm.wasm.v1.MsgInstantiateContract2': MsgInstantiateContract2,
            '/cosmwasm.wasm.v1.MsgMigrateContract': MsgMigrateContract,
            '/cosmwasm.wasm.v1.MsgStoreAndInstantiateContract': MsgStoreAndInstantiateContract,
            '/cosmwasm.wasm.v1.MsgUpdateInstantiateConfig': MsgUpdateInstantiateConfig,
        };

        const MessageType = typeMap[message.typeUrl];

        if (
            message.typeUrl === '/axelar.nexus.v1beta1.ActivateChainRequest' ||
            message.typeUrl === '/axelar.nexus.v1beta1.DeactivateChainRequest'
        ) {
            const typeName = message.typeUrl.includes('Deactivate') ? 'DeactivateChainRequest' : 'ActivateChainRequest';
            const MsgType = getNexusProtoType(typeName);
            const decoded = MsgType.decode(message.value);
            printInfo(`Encoded ${message.typeUrl}`, JSON.stringify(decoded, null, 2));
        } else if (MessageType) {
            const decoded = (MessageType as { decode: (value: Uint8Array) => Record<string, unknown> }).decode(message.value);
            if (decoded.codeId) {
                decoded.codeId = decoded.codeId.toString();
            }
            if (
                (message.typeUrl === '/cosmwasm.wasm.v1.MsgExecuteContract' ||
                    message.typeUrl === '/cosmwasm.wasm.v1.MsgInstantiateContract' ||
                    message.typeUrl === '/cosmwasm.wasm.v1.MsgInstantiateContract2' ||
                    message.typeUrl === '/cosmwasm.wasm.v1.MsgMigrateContract' ||
                    message.typeUrl === '/cosmwasm.wasm.v1.MsgStoreAndInstantiateContract') &&
                decoded.msg
            ) {
                decoded.msg = JSON.parse(Buffer.from(decoded.msg as Uint8Array).toString());
            }
            if (decoded.wasmByteCode) {
                decoded.wasmByteCode = `<${(decoded.wasmByteCode as Uint8Array).length} bytes>`;
            }
            printInfo(`Encoded ${message.typeUrl}`, JSON.stringify(decoded, null, 2));
        } else {
            printInfo(`Unknown message type: ${message.typeUrl}`, '<Unable to decode>');
        }
    });
};

const confirmProposalSubmission = (options: ProposalOptions, proposalData: object[]): boolean => {
    printProposal(proposalData);
    if (prompt(`Proceed with proposal submission?`, options.yes)) {
        return false;
    }
    return true;
};

const submitProposal = async (
    client: ClientManager,
    config: ConfigManager,
    options: ProposalOptions,
    proposal: object[],
    fee?: string | StdFee,
): Promise<string> => {
    const deposit =
        options.deposit ?? (options.standardProposal ? config.proposalDepositAmount() : config.proposalExpeditedDepositAmount());
    const proposalOptions = { ...options, deposit };

    const [account] = client.accounts;

    printInfo('Proposer address', account.address);

    const messages = toArray(proposal);

    const submitProposalMsg = encodeSubmitProposal(messages, config, proposalOptions, account.address);

    const result = await signAndBroadcastWithRetry(client, account.address, [submitProposalMsg], fee, '');
    const { events } = result;

    const proposalEvent = events.find(({ type }) => type === 'proposal_submitted' || type === 'submit_proposal');
    if (!proposalEvent) {
        throw new Error('Proposal submission event not found');
    }

    const proposalId = proposalEvent.attributes.find(({ key }) => key === 'proposal_id')?.value;
    if (!proposalId) {
        throw new Error('Proposal ID not found in events');
    }

    return proposalId;
};

const submitMessagesAsProposal = async (
    client: ClientManager,
    config: ConfigManager,
    options: ProposalOptions & { title?: string; description?: string },
    messages: object[],
    fee?: string | StdFee,
): Promise<string> => {
    if (!confirmProposalSubmission(options, messages)) {
        throw new Error('Proposal submission cancelled');
    }

    const proposalId = await submitProposal(client, config, options, messages, fee);
    printInfo('Proposal submitted', proposalId);
    return proposalId;
};

const executeByGovernance = async (
    client: ClientManager,
    config: ConfigManager,
    options: ProposalOptions & { contractName?: string | string[]; msg?: string | string[]; chainName?: string; dryRun?: boolean },
    _args?: string[],
    fee?: string | StdFee,
): Promise<string | undefined> => {
    const { chainName, dryRun } = options;
    let contractName = options.contractName;

    if (!Array.isArray(contractName)) {
        contractName = [contractName as string];
    }

    const singleContractName = contractName[0];
    if (contractName.length > 1) {
        throw new Error(
            'Execute command only supports one contract at a time. Use multiple --msg flags for multiple messages to the same contract.',
        );
    }

    const { msg } = options;
    const msgs = toArray(msg);

    const messages = msgs.map((msgJson) => {
        const msgOptions = { ...options, contractName: singleContractName, msg: msgJson };
        return encodeExecuteContract(config, msgOptions, chainName);
    });

    if (dryRun) {
        const contractConfig = config.axelar.contracts[singleContractName];
        const chainConfig = chainName ? getChainConfig(config.chains, chainName) : null;
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const contractAddress = ((contractConfig as any)[chainConfig?.axelarId]?.address || contractConfig.address) as string;

        const dryRunOutput = messages.map((message, index) => ({
            '@type': '/cosmwasm.wasm.v1.MsgExecuteContract',
            sender: GOVERNANCE_MODULE_ADDRESS,
            contract: contractAddress,
            msg: JSON.parse(msgs[index]),
            funds: [],
        }));

        console.log(JSON.stringify(dryRunOutput, null, 2));
        return;
    }

    if (!confirmProposalSubmission(options, messages)) {
        return;
    }

    const proposalId = await submitProposal(client, config, options, messages, fee);
    printInfo('Proposal submitted', proposalId);
    return proposalId;
};

const migrate = async (
    client: ClientManager,
    config: ConfigManager,
    options: ProposalOptions & { contractName?: string | string[]; chainName?: string },
    _args?: string[],
    fee?: string | StdFee,
): Promise<string | undefined> => {
    let contractName: string;

    if (Array.isArray(options.contractName)) {
        if (options.contractName.length > 1) {
            throw new Error('migrate only supports a single contract at a time');
        }
        contractName = options.contractName[0];
    } else {
        contractName = options.contractName!;
    }

    const optionsWithContractName = { ...options, contractName };
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const { contractConfig } = getAmplifierContractConfig(config, optionsWithContractName as any);
    contractConfig.codeId = await getCodeId(client, config, optionsWithContractName);

    const proposal = encodeMigrate(config, optionsWithContractName);

    if (!confirmProposalSubmission(options, [proposal])) {
        return;
    }

    const proposalId = await submitProposal(client, config, options, [proposal], fee);
    printInfo('Proposal submitted', proposalId);
    return proposalId;
};

export { printProposal, confirmProposalSubmission, submitProposal, submitMessagesAsProposal, executeByGovernance, migrate };
