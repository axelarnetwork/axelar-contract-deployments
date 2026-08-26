const { expect } = require('chai');
const { MsgExecuteContract } = require('cosmjs-types/cosmwasm/wasm/v1/tx');

const { createAxelarnetGatewayMessages } = require('../governance');
const { encodeExecuteContract, GOVERNANCE_MODULE_ADDRESS } = require('../../cosmwasm/utils');

describe('createAxelarnetGatewayMessages', () => {
    it('routes edge-chain governance calls through AxelarnetGateway.call_contract', () => {
        const messages = createAxelarnetGatewayMessages({
            title: 'Interchain Token Service Governance Proposal',
            description: 'Interchain Token Service Governance Proposal',
            contract_calls: [
                {
                    chain: 'Polygon',
                    contract_address: '0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525',
                    payload: 'AQID',
                },
            ],
        });

        expect(messages).to.deep.equal([
            {
                call_contract: {
                    destination_chain: 'Polygon',
                    destination_address: '0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525',
                    payload: '010203',
                },
            },
        ]);
    });

    it('rejects empty contract-call proposals', () => {
        expect(() => createAxelarnetGatewayMessages({ title: 'title', description: 'description', contract_calls: [] })).to.throw(
            'at least one contract call',
        );
    });

    it('encodes governance as the authenticated AxelarnetGateway caller', () => {
        const gateway = 'axelar18vsne7lns36uvm8gv2cv5jl2lghts0xm7dvzpqzn70dl56gk9hvsgu9sqg';
        const msg = JSON.stringify({
            call_contract: {
                destination_chain: 'Polygon',
                destination_address: '0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525',
                payload: '010203',
            },
        });
        const encoded = encodeExecuteContract(
            { axelar: { contracts: { AxelarnetGateway: { address: gateway } } } },
            { contractName: 'AxelarnetGateway', msg },
        );
        const decoded = MsgExecuteContract.decode(encoded.value);

        expect(encoded.typeUrl).to.equal('/cosmwasm.wasm.v1.MsgExecuteContract');
        expect(decoded.sender).to.equal(GOVERNANCE_MODULE_ADDRESS);
        expect(decoded.contract).to.equal(gateway);
        expect(Buffer.from(decoded.msg).toString()).to.equal(msg);
        expect(decoded.funds).to.deep.equal([]);
    });
});
