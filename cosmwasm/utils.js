'use strict';

const { ethers } = require('hardhat');
const {
    utils: { keccak256 },
} = ethers;
const { createHash } = require('crypto');

const { readFileSync } = require('fs');
const { calculateFee, GasPrice } = require('@cosmjs/stargate');
const { instantiate2Address, SigningCosmWasmClient } = require('@cosmjs/cosmwasm-stargate');
const { DirectSecp256k1HdWallet } = require('@cosmjs/proto-signing');
const { MsgSubmitProposal } = require('cosmjs-types/cosmos/gov/v1beta1/tx');
const { StoreCodeProposal } = require('cosmjs-types/cosmwasm/wasm/v1/proposal');
const { getSaltFromKey } = require('../evm/utils');
const { normalizeBech32 } = require('@cosmjs/encoding');

const governanceAddress = 'axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj';

const prepareWallet = ({ mnemonic }) => DirectSecp256k1HdWallet.fromMnemonic(mnemonic, { prefix: 'axelar' });

const prepareClient = ({ axelar: { rpc, gasPrice } }, wallet) =>
    SigningCosmWasmClient.connectWithSigner(rpc, wallet, { gasPrice }).then((client) => {
        return { wallet, client };
    });

const pascalToSnake = (str) => str.replace(/([A-Z])/g, (group) => `_${group.toLowerCase()}`).replace(/^_/, '');

const isValidCosmosAddress = (str) => {
    try {
        normalizeBech32(str);

        return true;
    } catch (error) {
        return false;
    }
};

const fromHex = (str) => new Uint8Array(Buffer.from(str.replace('0x', ''), 'hex'));

const calculateDomainSeparator = (chain, router, network) => keccak256(Buffer.from(`${chain}${router}${network}`));

const uploadContract = async (client, wallet, config, options) => {
    const { artifactPath, contractName, instantiate2, salt, aarch64, chainNames } = options;
    return wallet
        .getAccounts()
        .then(([account]) => {
            const wasm = readFileSync(`${artifactPath}/${pascalToSnake(contractName)}${aarch64 ? '-aarch64' : ''}.wasm`);
            const {
                axelar: { gasPrice, gasLimit },
            } = config;
            const uploadFee = gasLimit === 'auto' ? 'auto' : calculateFee(gasLimit, GasPrice.fromString(gasPrice));
            return client.upload(account.address, wasm, uploadFee).then(({ checksum, codeId }) => ({ checksum, codeId, account }));
        })
        .then(({ account, checksum, codeId }) => {
            const address = instantiate2
                ? instantiate2Address(
                      fromHex(checksum),
                      account.address,
                      fromHex(getSaltFromKey(salt || contractName.concat(chainNames))),
                      'axelar',
                  )
                : null;

            return { codeId, address };
        });
};

const instantiateContract = (client, wallet, initMsg, config, { contractName, salt, instantiate2, chainNames, admin }) => {
    return wallet
        .getAccounts()
        .then(([account]) => {
            const contractConfig = config.axelar.contracts[contractName];

            const {
                axelar: { gasPrice, gasLimit },
            } = config;
            const initFee = gasLimit === 'auto' ? 'auto' : calculateFee(gasLimit, GasPrice.fromString(gasPrice));

            return instantiate2
                ? client.instantiate2(
                      account.address,
                      contractConfig.codeId,
                      fromHex(getSaltFromKey(salt || contractName.concat(chainNames))),
                      initMsg,
                      contractName,
                      initFee,
                      { admin },
                  )
                : client.instantiate(account.address, contractConfig.codeId, initMsg, contractName, initFee, {
                      admin,
                  });
        })
        .then(({ contractAddress }) => contractAddress);
};

const encodeStoreCodeProposal = (options) => {
    const { artifactPath, contractName, aarch64, title, description, runAs, source, builder } = options;

    const wasm = readFileSync(`${artifactPath}/${pascalToSnake(contractName)}${aarch64 ? '-aarch64' : ''}.wasm`);

    let codeHash;

    // source, builder and codeHash are optional, but mandatory if one is provided
    if (source && builder) {
        codeHash = createHash('sha256').update(wasm).digest();
    }

    const proposal = StoreCodeProposal.fromPartial({
        title,
        description,
        runAs,
        wasmByteCode: wasm,
        source,
        builder,
        codeHash,
    });

    return {
        typeUrl: '/cosmwasm.wasm.v1.StoreCodeProposal',
        value: Uint8Array.from(StoreCodeProposal.encode(proposal).finish()),
    };
};

const encodeSubmitProposal = (content, config, options, proposer) => {
    const {
        axelar: { tokenSymbol },
    } = config;
    const { deposit } = options;

    return {
        typeUrl: '/cosmos.gov.v1beta1.MsgSubmitProposal',
        value: MsgSubmitProposal.fromPartial({
            content,
            initialDeposit: [{ denom: `u${tokenSymbol.toLowerCase()}`, amount: deposit }],
            proposer,
        }),
    };
};

const submitProposal = (client, wallet, config, options, content) => {
    return wallet
        .getAccounts()
        .then(([account]) => {
            const {
                axelar: { gasPrice, gasLimit },
            } = config;

            const submitProposalMsg = encodeSubmitProposal(content, config, options, account.address);

            const storeFee = gasLimit === 'auto' ? 'auto' : calculateFee(gasLimit, GasPrice.fromString(gasPrice));
            return client.signAndBroadcast(account.address, [submitProposalMsg], storeFee, '');
        })
        .then(
            ({ events }) => events.find(({ type }) => type === 'submit_proposal').attributes.find(({ key }) => key === 'proposal_id').value,
        );
};

const submitStoreCodeProposal = (client, wallet, config, options) => {
    const content = encodeStoreCodeProposal(options);

    return submitProposal(client, wallet, config, options, content);
};

module.exports = {
    governanceAddress,
    prepareWallet,
    prepareClient,
    calculateDomainSeparator,
    uploadContract,
    instantiateContract,
    submitStoreCodeProposal,
    isValidCosmosAddress,
};
