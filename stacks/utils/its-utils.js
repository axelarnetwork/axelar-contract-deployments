const { Cl, BytesReader, deserializeTransaction } = require('@stacks/transactions');
const { intToHex } = require('@stacks/common');
const { bytesToHex, hexToBytes } = require("@noble/hashes/utils");
const { sha512_256 } = require("@noble/hashes/sha512");

/**
 * Utils for constructing verification proof for Stacks
 */

function tagged_sha512_256(tag, data) {
    return sha512_256(new Uint8Array([...tag, ...data]));
}

class MerkleTree {
    static MERKLE_PATH_LEAF_TAG = new Uint8Array([0x00]);
    static MERKLE_PATH_NODE_TAG = new Uint8Array([0x01]);

    nodes;

    constructor(nodes = []) {
        this.nodes = nodes;
    }

    static empty() {
        return new MerkleTree();
    }

    static new(data) {
        if (data.length === 0) {
            return new MerkleTree();
        }

        let leaf_hashes = data.map((buf) =>
            MerkleTree.get_leaf_hash(buf),
        );

        // force even number
        if (leaf_hashes.length % 2 !== 0) {
            const dup = leaf_hashes[leaf_hashes.length - 1];
            leaf_hashes.push(dup);
        }

        let nodes = [leaf_hashes];

        while (true) {
            const current_level = nodes[nodes.length - 1];
            const next_level = [];

            for (let i = 0; i < current_level.length; i += 2) {
                if (i + 1 < current_level.length) {
                    next_level.push(
                        MerkleTree.get_node_hash(current_level[i], current_level[i + 1]),
                    );
                } else {
                    next_level.push(current_level[i]);
                }
            }

            // at root
            if (next_level.length === 1) {
                nodes.push(next_level);
                break;
            }

            // force even number
            if (next_level.length % 2 !== 0) {
                const dup = next_level[next_level.length - 1];
                next_level.push(dup);
            }

            nodes.push(next_level);
        }

        return new MerkleTree(nodes);
    }

    static get_leaf_hash(leaf_data) {
        return tagged_sha512_256(MerkleTree.MERKLE_PATH_LEAF_TAG, leaf_data);
    }

    static get_node_hash(left, right) {
        return tagged_sha512_256(
            MerkleTree.MERKLE_PATH_NODE_TAG,
            new Uint8Array([...left, ...right]),
        );
    }

    proof(index) {
        if (this.nodes.length === 0) {
            return [];
        }
        if (index > this.nodes[0].length - 1) {
            throw new Error("Index out of bounds");
        }
        const depth = this.nodes.length - 1;
        const path = Math.pow(2, depth) + index;

        let proof = [];
        let position = index;
        for (let level = 0; level < depth; ++level) {
            const left = ((1 << level) & path) > 0;
            proof.push(this.nodes[level][position + (left ? -1 : 1)]);
            position = ~~(position / 2);
        }

        return proof;
    }

    root() {
        if (this.nodes.length === 0) {
            return new Uint8Array(32);
        }
        return this.nodes[this.nodes.length - 1][0];
    }

    pretty_print() {
        let str = "";
        for (let level = this.nodes.length - 1; level >= 0; --level) {
            const whitespace = " ".repeat((this.nodes.length - level - 1) * 2);
            str += this.nodes[level]
                .map((node) => whitespace + bytesToHex(node) + "\n")
                .join("");
        }
        return str;
    }
}

async function getRawTx({ txId }, rpc) {
    const txRawRes = await fetch(
        `${rpc}/extended/v1/tx/${txId}/raw`,
    );
    const txRawData = await txRawRes.json();
    return txRawData.raw_tx;
}

async function getTxInfo({ txId }, rpc) {
    const txInfoRes = await fetch(
        `${rpc}/extended/v1/tx/${txId}`,
    );

    return await txInfoRes.json();
}

function deserializeTransactionCustom(bytesReader) {
    const transaction = deserializeTransaction(bytesReader);
    return { transaction, reader: bytesReader };
}

function deserializeRawBlockTxs(txs, processedTxs = []) {
    const { transaction, reader } = deserializeTransactionCustom(
        txs instanceof BytesReader ? txs : new BytesReader(txs),
    );

    processedTxs = processedTxs.concat(transaction.txid());

    if (reader.consumed === reader.source.length) {
        return processedTxs;
    }
    return deserializeRawBlockTxs(reader, processedTxs);
}

function proof_path_to_cv(
    tx_index,
    hashes,
    tree_depth,
) {
    return Cl.tuple({
        "tx-index": Cl.uint(tx_index),
        hashes: Cl.list(hashes.map(Cl.buffer)),
        "tree-depth": Cl.uint(tree_depth),
    });
}

async function getVerificationParams(txId, rpc) {
    const txRaw = await getRawTx({ txId }, rpc);
    const txInfoData = await getTxInfo({ txId }, rpc);

    const txIndex = txInfoData.tx_index;
    const blockHeight = txInfoData.block_height;
    const block = new Uint8Array(
        await (
            await fetch(`${rpc}/v3/blocks/height/${blockHeight}`)
        ).arrayBuffer(),
    );

    const block_version = block.slice(0, 1);
    const chain_length = block.slice(1, 9);
    const burn_spent = block.slice(9, 17);
    const consensus_hash = block.slice(17, 37);
    const parent_block_id = block.slice(37, 69);
    const tx_merkle_root = block.slice(69, 101);
    const state_root = block.slice(101, 133);
    const timestamp = block.slice(133, 141);
    const miner_signature = block.slice(141, 206);
    const signatureCount = Number("0x" + bytesToHex(block.slice(206, 210)));
    const pastSignatures = 210 + signatureCount * 65;
    // const signerBitVecLen = Number("0x" + bytesToHex(block.slice(pastSignatures, pastSignatures + 2)))
    const signerBitVecByteLen = Number(
        "0x" + bytesToHex(block.slice(pastSignatures + 2, pastSignatures + 6)),
    );
    const signer_bitvec = block.slice(
        pastSignatures,
        pastSignatures + 6 + signerBitVecByteLen,
    );

    const txs = block.slice(pastSignatures + 10 + signerBitVecByteLen);
    const txids = deserializeRawBlockTxs(txs);
    const tx_merkle_tree = MerkleTree.new(txids.map(hexToBytes));

    const blockHeader = new Uint8Array([
        ...block_version,
        ...chain_length,
        ...burn_spent,
        ...consensus_hash,
        ...parent_block_id,
        ...tx_merkle_root,
        ...state_root,
        ...timestamp,
        ...miner_signature,
        ...signer_bitvec,
    ]);

    const proof = tx_merkle_tree.proof(txIndex);

    const tx = deserializeTransaction(txRaw);

    return Cl.tuple({
        nonce: Cl.bufferFromHex(intToHex(txInfoData.nonce, 8)),
        "fee-rate": Cl.bufferFromHex(intToHex(txInfoData.fee_rate, 8)),
        signature: Cl.bufferFromHex(
            tx.auth.spendingCondition.signature.data,
        ),
        proof: proof_path_to_cv(txIndex, proof, proof.length),
        "tx-block-height": Cl.uint(txInfoData.block_height),
        "block-header-without-signer-signatures": Cl.buffer(blockHeader),
    });
}

async function getTokenTxId(contract, rpc) {
    const res = await fetch(
        `${rpc}/extended/v1/contract/${contract}`,
    );

    const json = await res.json();

    return json.tx_id;
}

module.exports = {
    getVerificationParams,
    getTokenTxId,
};
