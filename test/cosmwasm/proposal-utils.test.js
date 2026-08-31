const { expect } = require('chai');
const { MsgExecuteContract, MsgMigrateContract } = require('cosmjs-types/cosmwasm/wasm/v1/tx');

const { ConfigManager } = require('../../common/config');
const { createProposalJson, messageToProtoJson } = require('../../cosmwasm/proposal-utils');
const { encodeChainStatusRequest, encodeSubmitProposal } = require('../../cosmwasm/utils');

const encodeMessage = (typeUrl, codec, message) => ({
    typeUrl,
    value: codec.encode(message).finish(),
});

describe('axelard proposal JSON', () => {
    it('converts CosmWasm messages to proto JSON', () => {
        const message = MsgExecuteContract.fromPartial({
            sender: 'axelar1sender',
            contract: 'axelar1contract',
            msg: Buffer.from('{"freeze":{}}'),
            funds: [],
        });

        expect(messageToProtoJson(encodeMessage(MsgExecuteContract.typeUrl, MsgExecuteContract, message))).to.deep.equal({
            '@type': '/cosmwasm.wasm.v1.MsgExecuteContract',
            sender: 'axelar1sender',
            contract: 'axelar1contract',
            msg: { freeze: {} },
            funds: [],
        });
    });

    it('preserves uint64 fields as strings', () => {
        const message = MsgMigrateContract.fromPartial({
            sender: 'axelar1sender',
            contract: 'axelar1contract',
            codeId: 42n,
            msg: Buffer.from('{}'),
        });

        expect(messageToProtoJson(encodeMessage(MsgMigrateContract.typeUrl, MsgMigrateContract, message))).to.deep.include({
            '@type': '/cosmwasm.wasm.v1.MsgMigrateContract',
            codeId: '42',
            msg: {},
        });
    });

    it('converts Axelar Nexus messages to proto JSON', () => {
        const message = encodeChainStatusRequest(['ethereum', 'avalanche'], 'ActivateChainRequest');

        expect(messageToProtoJson(message)).to.deep.equal({
            '@type': '/axelar.nexus.v1beta1.ActivateChainRequest',
            chains: ['ethereum', 'avalanche'],
            sender: 'axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj',
        });
    });

    it('builds the proposal file consumed by axelard', () => {
        const config = new ConfigManager('testnet');
        const message = MsgExecuteContract.fromPartial({
            sender: 'axelar1sender',
            contract: 'axelar1contract',
            msg: Buffer.from('{}'),
        });

        const proposal = createProposalJson([encodeMessage(MsgExecuteContract.typeUrl, MsgExecuteContract, message)], config, {
            title: 'Test proposal',
            description: 'Test summary',
        });

        expect(proposal).to.deep.include({
            metadata: '',
            deposit: '3000000000uaxl',
            title: 'Test proposal',
            summary: 'Test summary',
            expedited: true,
        });
        expect(proposal.messages).to.have.length(1);
    });

    it('generates byte-equivalent online and offline contract messages', () => {
        const config = new ConfigManager('mainnet');
        const options = {
            title: 'Equivalent proposal',
            description: 'Equivalent proposal summary',
            deposit: '400000000000',
            standardProposal: false,
        };
        const proposer = 'axelar1proposer';
        const message = MsgExecuteContract.fromPartial({
            sender: 'axelar1sender',
            contract: 'axelar1contract',
            msg: Buffer.from('{"call_contract":{"destination_chain":"Polygon"}}'),
            funds: [],
        });
        const encodedMessage = encodeMessage(MsgExecuteContract.typeUrl, MsgExecuteContract, message);

        const online = encodeSubmitProposal([encodedMessage], config, options, proposer).value;
        const offline = createProposalJson([encodedMessage], config, options);
        const offlineMessage = offline.messages[0];
        const reconstructedOfflineMessage = MsgExecuteContract.fromPartial({
            sender: offlineMessage.sender,
            contract: offlineMessage.contract,
            msg: Buffer.from(JSON.stringify(offlineMessage.msg)),
            funds: offlineMessage.funds,
        });

        expect(MsgExecuteContract.encode(reconstructedOfflineMessage).finish()).to.deep.equal(online.messages[0].value);
        expect({
            metadata: offline.metadata,
            title: offline.title,
            summary: offline.summary,
            expedited: offline.expedited,
            deposit: offline.deposit,
            proposer,
        }).to.deep.equal({
            metadata: online.metadata,
            title: online.title,
            summary: online.summary,
            expedited: online.expedited,
            deposit: `${online.initialDeposit[0].amount}${online.initialDeposit[0].denom}`,
            proposer: online.proposer,
        });
    });

    it('uses the current mainnet governance deposits', () => {
        const config = new ConfigManager('mainnet');
        const message = MsgExecuteContract.fromPartial({
            sender: 'axelar1sender',
            contract: 'axelar1contract',
            msg: Buffer.from('{}'),
        });
        const messages = [encodeMessage(MsgExecuteContract.typeUrl, MsgExecuteContract, message)];

        const expedited = createProposalJson(messages, config, { title: 'Expedited', description: 'Expedited' });
        const standard = createProposalJson(messages, config, {
            title: 'Standard',
            description: 'Standard',
            standardProposal: true,
        });

        expect(expedited.deposit).to.equal('400000000000uaxl');
        expect(standard.deposit).to.equal('150000000000uaxl');
    });

    it('rejects message types axelard cannot decode', () => {
        expect(() => messageToProtoJson({ typeUrl: '/unknown.Msg', value: new Uint8Array() })).to.throw(
            'unsupported message type /unknown.Msg',
        );
    });
});
