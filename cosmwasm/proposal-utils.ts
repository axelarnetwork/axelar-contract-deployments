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

import { printInfo, prompt } from '../common';
import { ConfigManager } from '../common/config';
import { ClientManager } from './processor';
import { getNexusProtoType, submitProposal } from './utils';

interface ProposalOptions {
    yes?: boolean;
    [key: string]: any;
}

const printProposal = (proposalData: object[]): void => {
    proposalData.forEach((message: any) => {
        const typeMap: Record<string, any> = {
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
            const decoded = MessageType.decode(message.value);
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
                decoded.msg = JSON.parse(Buffer.from(decoded.msg).toString());
            }
            if (decoded.wasmByteCode) {
                decoded.wasmByteCode = `<${decoded.wasmByteCode.length} bytes>`;
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

const submitProposalAndPrint = async (
    client: ClientManager,
    config: ConfigManager,
    options: ProposalOptions,
    proposal: object[],
    fee?: string | StdFee,
): Promise<void> => {
    const proposalId = await submitProposal(client, config, options, proposal, fee);
    printInfo('Proposal submitted', proposalId);
};

export { printProposal, confirmProposalSubmission, submitProposalAndPrint };
