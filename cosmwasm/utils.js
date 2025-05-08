'use strict';

const zlib = require('zlib');
const { createHash } = require('crypto');
const { calculateFee, GasPrice } = require('@cosmjs/stargate');
const { SigningCosmWasmClient } = require('@cosmjs/cosmwasm-stargate');
const { DirectSecp256k1HdWallet } = require('@cosmjs/proto-signing');
const { MsgSubmitProposal } = require('cosmjs-types/cosmos/gov/v1beta1/tx');
const {
    StoreCodeProposal,
    StoreAndInstantiateContractProposal,
    InstantiateContractProposal,
    InstantiateContract2Proposal,
    ExecuteContractProposal,
    MigrateContractProposal,
} = require('cosmjs-types/cosmwasm/wasm/v1/proposal');
const { ParameterChangeProposal } = require('cosmjs-types/cosmos/params/v1beta1/params');
const { AccessType } = require('cosmjs-types/cosmwasm/wasm/v1/types');
const {
    printInfo,
    printWarn,
    isString,
    isStringArray,
    isKeccak256Hash,
    isNumber,
    toBigNumberString,
    getChainConfig,
    getSaltFromKey,
    calculateDomainSeparator,
    validateParameters,
} = require('../common');
const {
    pascalToSnake,
    pascalToKebab,
    downloadContractCode,
    readContractCode,
    VERSION_REGEX,
    SHORT_COMMIT_HASH_REGEX,
} = require('../common/utils');
const { normalizeBech32 } = require('@cosmjs/encoding');

const { XRPLClient } = require('../xrpl/utils');

const DEFAULT_MAX_UINT_BITS_EVM = 256;
const DEFAULT_MAX_DECIMALS_WHEN_TRUNCATING_EVM = 255;

const CONTRACT_SCOPE_GLOBAL = 'global';
const CONTRACT_SCOPE_CHAIN = 'chain';

const governanceAddress = 'axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj';

const AXELAR_R2_BASE_URL = 'https://static.axelar.network';

const DUMMY_MNEMONIC = 'test test test test test test test test test test test junk';

const prepareWallet = async ({ mnemonic }) => await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, { prefix: 'axelar' });

const prepareDummyWallet = async () => {
    return await DirectSecp256k1HdWallet.fromMnemonic(DUMMY_MNEMONIC, { prefix: 'axelar' });
};

const prepareClient = async ({ axelar: { rpc, gasPrice } }, wallet) =>
    await SigningCosmWasmClient.connectWithSigner(rpc, wallet, { gasPrice });

const isValidCosmosAddress = (str) => {
    try {
        normalizeBech32(str);

        return true;
    } catch (error) {
        return false;
    }
};

const fromHex = (str) => new Uint8Array(Buffer.from(str.replace('0x', ''), 'hex'));

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
            const usedSalt = salt || contractName.concat(chainNames);
            const address = instantiate2
                ? instantiate2Address(
                      fromHex(checksum),
                      account.address,
                      fromHex(getSaltFromKey(usedSalt)),
                      'axelar',
                  )
                : null;

            return { codeId, address, usedSalt };
        });
};

const instantiateContract = (client, wallet, initMsg, config, { contractName, instantiate2, admin }) => {
    return wallet
        .getAccounts()
        .then(([account]) => {
            const contractConfig = config.axelar.contracts[contractName];

    if (!contractBaseConfig) {
        throw new Error(`Contract ${contractName} not found in config`);
    }

            return instantiate2
                ? client.instantiate2(
                      account.address,
                      contractConfig.codeId,
                      contractConfig.salt,
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

module.exports = {
    CONTRACT_SCOPE_CHAIN,
    CONTRACT_SCOPE_GLOBAL,
    CONTRACTS,
    governanceAddress,
    prepareWallet,
    prepareDummyWallet,
    prepareClient,
    fromHex,
    getSalt,
    calculateDomainSeparator,
    initContractConfig,
    getAmplifierBaseContractConfig,
    getAmplifierContractConfig,
    getCodeId,
    uploadContract,
    instantiateContract,
    migrateContract,
    fetchCodeIdFromCodeHash,
    fetchCodeIdFromContract,
    addDefaultInstantiateAddresses,
    getChainTruncationParams,
    decodeProposalAttributes,
    encodeStoreCodeProposal,
    encodeStoreInstantiateProposal,
    encodeInstantiateProposal,
    encodeInstantiate2Proposal,
    encodeExecuteContractProposal,
    encodeParameterChangeProposal,
    encodeMigrateContractProposal,
    submitProposal,
    isValidCosmosAddress,
    getContractCodePath,
};
