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

import { printInfo, prompt, writeJSON } from '../common';
import { ConfigManager } from '../common/config';
import { ClientManager } from './processor';
import {
    encodeExecuteContract,
    encodeMigrate,
    encodeSubmitProposal,
    getAmplifierContractConfig,
    getCodeId,
    getNexusProtoType,
    getUnitDenom,
    signAndBroadcastWithRetry,
    toArray,
} from './utils';

interface ProposalOptions {
    yes?: boolean;
    contractName?: string | string[];
    chainName?: string;
    dryRun?: boolean;
    msg?: string | string[];
    title?: string;
    description?: string;
    deposit?: string;
    standardProposal?: boolean;
    generateOnly?: string;
    [key: string]: unknown;
}

interface EncodedMessage {
    typeUrl: string;
    value: Uint8Array;
}

interface ProtoCodec {
    decode: (value: Uint8Array) => unknown;
    toJSON: (value: never) => Record<string, unknown>;
}

const messageTypeMap: Record<string, ProtoCodec> = {
    '/cosmwasm.wasm.v1.MsgStoreCode': MsgStoreCode,
    '/cosmwasm.wasm.v1.MsgExecuteContract': MsgExecuteContract,
    '/cosmwasm.wasm.v1.MsgInstantiateContract': MsgInstantiateContract,
    '/cosmwasm.wasm.v1.MsgInstantiateContract2': MsgInstantiateContract2,
    '/cosmwasm.wasm.v1.MsgMigrateContract': MsgMigrateContract,
    '/cosmwasm.wasm.v1.MsgStoreAndInstantiateContract': MsgStoreAndInstantiateContract,
    '/cosmwasm.wasm.v1.MsgUpdateInstantiateConfig': MsgUpdateInstantiateConfig,
};

const rawContractMessageTypes = new Set([
    '/cosmwasm.wasm.v1.MsgExecuteContract',
    '/cosmwasm.wasm.v1.MsgInstantiateContract',
    '/cosmwasm.wasm.v1.MsgInstantiateContract2',
    '/cosmwasm.wasm.v1.MsgMigrateContract',
    '/cosmwasm.wasm.v1.MsgStoreAndInstantiateContract',
]);

const messageToProtoJson = (message: EncodedMessage): Record<string, unknown> => {
    const MessageType = messageTypeMap[message.typeUrl];
    if (MessageType) {
        const decoded = MessageType.decode(message.value) as Record<string, unknown>;
        const json = MessageType.toJSON(decoded as never);

        // wasmd's RawContractMessage JSON representation is the embedded JSON
        // object, not the protobuf bytes encoded as a base64 string.
        if (rawContractMessageTypes.has(message.typeUrl) && decoded.msg) {
            json.msg = JSON.parse(Buffer.from(decoded.msg as Uint8Array).toString());
        }

        return { '@type': message.typeUrl, ...json };
    }

    if (
        message.typeUrl === '/axelar.nexus.v1beta1.ActivateChainRequest' ||
        message.typeUrl === '/axelar.nexus.v1beta1.DeactivateChainRequest'
    ) {
        const typeName = message.typeUrl.includes('Deactivate') ? 'DeactivateChainRequest' : 'ActivateChainRequest';
        const MessageType = getNexusProtoType(typeName);
        const decoded = MessageType.decode(message.value);
        const json = MessageType.toObject(decoded, { longs: String, enums: String, bytes: String });
        return { '@type': message.typeUrl, ...json };
    }

    throw new Error(`Cannot generate axelard proposal JSON for unsupported message type ${message.typeUrl}`);
};

const createProposalJson = (messages: EncodedMessage[], config: ConfigManager, options: ProposalOptions): Record<string, unknown> => {
    const deposit =
        options.deposit ?? (options.standardProposal ? config.proposalDepositAmount() : config.proposalExpeditedDepositAmount());
    const unitDenom = getUnitDenom(config);

    return {
        messages: messages.map(messageToProtoJson),
        metadata: '',
        deposit: `${deposit}${unitDenom}`,
        title: options.title,
        summary: options.description,
        expedited: !options.standardProposal,
    };
};

const getSingleContractName = (contractName: string | string[] | undefined, operation: string): string => {
    if (Array.isArray(contractName)) {
        if (contractName.length > 1) {
            throw new Error(`${operation} only supports single contract at a time`);
        }
        return contractName[0];
    }
    return contractName!;
};

const printProposal = (proposalData: object[]): void => {
    proposalData.forEach((msg: unknown) => {
        const message = msg as { typeUrl: string; value: Uint8Array };
        const MessageType = messageTypeMap[message.typeUrl];

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
            if (rawContractMessageTypes.has(message.typeUrl) && decoded.msg) {
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
    if (options.generateOnly) {
        return true;
    }
    if (prompt(`Proceed with proposal submission?`, options.yes)) {
        return false;
    }
    return true;
};

const submitProposal = async (
    client: ClientManager,
    config: ConfigManager,
    options: ProposalOptions,
    proposal: object | object[],
    fee?: string | StdFee,
): Promise<string | undefined> => {
    const deposit =
        options.deposit ?? (options.standardProposal ? config.proposalDepositAmount() : config.proposalExpeditedDepositAmount());
    const proposalOptions = { ...options, deposit };

    const messages = toArray(proposal) as EncodedMessage[];

    if (options.generateOnly) {
        const proposalJson = createProposalJson(messages, config, proposalOptions);
        writeJSON(proposalJson, options.generateOnly);
        printInfo('Axelard proposal JSON written to file', options.generateOnly);
        return;
    }

    const [account] = client.accounts;
    printInfo('Proposer address', account.address);

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
    options: ProposalOptions,
    messages: object | object[],
    fee?: string | StdFee,
): Promise<string | undefined> => {
    const messagesArray = toArray(messages);

    if (!confirmProposalSubmission(options, messagesArray)) {
        return;
    }

    const proposalId = await submitProposal(client, config, options, messagesArray, fee);
    if (proposalId) {
        printInfo('Proposal submitted', proposalId);
    }
    return proposalId;
};

const executeByGovernance = async (
    client: ClientManager,
    config: ConfigManager,
    options: ProposalOptions,
    _args?: string[],
    fee?: string | StdFee,
): Promise<string | undefined> => {
    const { chainName, dryRun } = options;
    const singleContractName = getSingleContractName(options.contractName, 'execute');

    const { msg } = options;
    const msgs = toArray(msg);

    const messages = msgs.map((msgJson) => {
        const msgOptions = { ...options, contractName: singleContractName, msg: msgJson };
        return encodeExecuteContract(config, msgOptions, chainName);
    });

    if (dryRun) {
        printProposal(messages);
        return;
    }

    if (!confirmProposalSubmission(options, messages)) {
        return;
    }

    const proposalId = await submitProposal(client, config, options, messages, fee);
    if (proposalId) {
        printInfo('Proposal submitted', proposalId);
    }
    return proposalId;
};

const migrate = async (
    client: ClientManager,
    config: ConfigManager,
    options: ProposalOptions,
    _args?: string[],
    fee?: string | StdFee,
): Promise<string | undefined> => {
    const contractName = getSingleContractName(options.contractName, 'migrate');
    const optionsWithContractName = { ...options, contractName };
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const { contractConfig } = getAmplifierContractConfig(config, optionsWithContractName as any);
    contractConfig.codeId = await getCodeId(client, config, optionsWithContractName);

    const proposal = encodeMigrate(config, optionsWithContractName);

    if (!confirmProposalSubmission(options, [proposal])) {
        return;
    }

    const proposalId = await submitProposal(client, config, options, proposal, fee);
    if (proposalId) {
        printInfo('Proposal submitted', proposalId);
    }
    return proposalId;
};

export {
    createProposalJson,
    messageToProtoJson,
    printProposal,
    confirmProposalSubmission,
    submitProposal,
    submitMessagesAsProposal,
    executeByGovernance,
    migrate,
};
