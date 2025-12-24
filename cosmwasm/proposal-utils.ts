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
import { getNexusProtoType, getProtoType, submitProposal } from './utils';

interface ProposalOptions {
    yes?: boolean;
    [key: string]: unknown;
}

// Map of Axelar-specific message types to their proto file and package
const axelarMessageTypes: Record<string, { protoFile: string; packageName: string; typeName: string }> = {
    '/axelar.nexus.v1beta1.ActivateChainRequest': {
        protoFile: 'nexus_chain.proto',
        packageName: 'axelar.nexus.v1beta1',
        typeName: 'ActivateChainRequest',
    },
    '/axelar.nexus.v1beta1.DeactivateChainRequest': {
        protoFile: 'nexus_chain.proto',
        packageName: 'axelar.nexus.v1beta1',
        typeName: 'DeactivateChainRequest',
    },
    '/axelar.nexus.v1beta1.SetTransferRateLimitRequest': {
        protoFile: 'nexus_chain.proto',
        packageName: 'axelar.nexus.v1beta1',
        typeName: 'SetTransferRateLimitRequest',
    },
    '/axelar.nexus.v1beta1.RegisterAssetFeeRequest': {
        protoFile: 'nexus_chain.proto',
        packageName: 'axelar.nexus.v1beta1',
        typeName: 'RegisterAssetFeeRequest',
    },
    '/axelar.permission.v1beta1.RegisterControllerRequest': {
        protoFile: 'permission.proto',
        packageName: 'axelar.permission.v1beta1',
        typeName: 'RegisterControllerRequest',
    },
    '/axelar.permission.v1beta1.DeregisterControllerRequest': {
        protoFile: 'permission.proto',
        packageName: 'axelar.permission.v1beta1',
        typeName: 'DeregisterControllerRequest',
    },
    '/axelar.evm.v1beta1.SetGatewayRequest': {
        protoFile: 'evm.proto',
        packageName: 'axelar.evm.v1beta1',
        typeName: 'SetGatewayRequest',
    },
    '/axelar.evm.v1beta1.CreateTransferOperatorshipRequest': {
        protoFile: 'evm.proto',
        packageName: 'axelar.evm.v1beta1',
        typeName: 'CreateTransferOperatorshipRequest',
    },
    '/axelar.multisig.v1beta1.StartKeygenRequest': {
        protoFile: 'multisig.proto',
        packageName: 'axelar.multisig.v1beta1',
        typeName: 'StartKeygenRequest',
    },
    '/axelar.multisig.v1beta1.RotateKeyRequest': {
        protoFile: 'multisig.proto',
        packageName: 'axelar.multisig.v1beta1',
        typeName: 'RotateKeyRequest',
    },
};

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
        const axelarMsgInfo = axelarMessageTypes[message.typeUrl];

        if (axelarMsgInfo) {
            const MsgType = getProtoType(axelarMsgInfo.protoFile, axelarMsgInfo.packageName, axelarMsgInfo.typeName);
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

const submitProposalAndPrint = async (
    client: ClientManager,
    config: ConfigManager,
    options: ProposalOptions,
    proposal: object[],
    fee?: string | StdFee,
): Promise<string> => {
    const proposalId = await submitProposal(client, config, options, proposal, fee);
    printInfo('Proposal submitted', proposalId);
    return proposalId;
};

export { printProposal, confirmProposalSubmission, submitProposalAndPrint };
