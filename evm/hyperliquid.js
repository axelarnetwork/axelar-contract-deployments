'use strict';

const { Wallet, ethers, getDefaultProvider, Contract } = require('ethers');
const { Command, Argument } = require('commander');
const {
    printInfo,
    validateParameters,
    getContractJSON,
    getGasOptions,
    mainProcessor,
    isHyperliquidChain,
    printWalletInfo,
} = require('./utils');
const { addEvmOptions, addOptionsToCommands } = require('./cli-utils');
const { httpPost } = require('../common/utils');
const { handleTx } = require('./its');
const msgpack = require('msgpack-lite');
const { keccak256 } = require('ethers/lib/utils');

function addressToBytes(address) {
    return Buffer.from(address.replace('0x', ''), 'hex');
}

function actionHash(action, activePool, nonce) {
    const actionData = msgpack.encode(action);
    const nonceBuffer = Buffer.alloc(8);
    nonceBuffer.writeBigUInt64BE(BigInt(nonce));

    const vaultBuffer =
        activePool === null || activePool === undefined
            ? Buffer.from([0x00])
            : Buffer.concat([Buffer.from([0x01]), addressToBytes(activePool)]);

    const data = Buffer.concat([actionData, nonceBuffer, vaultBuffer]);
    return keccak256(data);
}

function constructPhantomAgent(hash, source) {
    return {
        source: source,
        connectionId: hash,
    };
}

async function signL1Action(wallet, action, activePool, nonce, chain) {
    const hash = actionHash(action, activePool, nonce);
    const phantomAgent = constructPhantomAgent(hash, chain.hypercore?.source);
    const domain = chain.hypercore?.domain;

    const agent = [
        { name: 'source', type: 'string' },
        { name: 'connectionId', type: 'bytes32' },
    ];

    const signature = await wallet._signTypedData(domain, { Agent: agent }, phantomAgent);
    const sig = ethers.utils.splitSignature(signature);

    return { r: sig.r, s: sig.s, v: sig.v };
}

async function updateBlockSize(wallet, chain, useBigBlocks) {
    const action = { type: 'evmUserModify', usingBigBlocks: useBigBlocks };
    const nonce = Date.now();
    const signature = await signL1Action(wallet, action, null, nonce, chain);
    const payload = { action, signature, nonce };
    const endpoint = `${chain.hypercore.url}/exchange`;
    const result = await httpPost(endpoint, payload);

    if (!result || result.status !== 'ok') {
        throw new Error(`Failed to update block size: ${result}`);
    }

    return result;
}

async function deployer(wallet, chain, args, _options) {
    const [tokenId] = args;
    validateParameters({
        isNonEmptyString: { tokenId },
    });

    const IInterchainTokenService = getContractJSON('IInterchainTokenService');

    const interchainTokenService = new Contract(chain.contracts.InterchainTokenService?.address, IInterchainTokenService.abi, wallet);

    const tokenAddress = await interchainTokenService.registeredTokenAddress(tokenId);
    printInfo('Token address', tokenAddress);

    try {
        const HyperliquidInterchainToken = getContractJSON('HyperliquidInterchainToken');
        const token = new Contract(tokenAddress, HyperliquidInterchainToken.abi, wallet);

        const currentDeployer = await token.deployer();
        printInfo('Current deployer', currentDeployer);
    } catch (error) {
        throw error;
    }
}

async function updateTokenDeployer(wallet, chain, args, options) {
    const [tokenId, deployer] = args;
    validateParameters({
        isNonEmptyString: { tokenId },
        isValidAddress: { deployer },
    });

    const interchainTokenServiceAddress = chain.contracts.InterchainTokenService?.address;

    validateParameters({
        isValidAddress: { interchainTokenServiceAddress },
    });

    const IInterchainTokenService = getContractJSON('IInterchainTokenService');
    const interchainTokenService = new Contract(interchainTokenServiceAddress, IInterchainTokenService.abi, wallet);

    const tokenAddress = await interchainTokenService.registeredTokenAddress(tokenId);
    printInfo('Token address', tokenAddress);

    const InterchainTokenService = getContractJSON('HyperliquidInterchainTokenService');
    const service = new Contract(interchainTokenServiceAddress, InterchainTokenService.abi, wallet);

    const HyperliquidInterchainToken = getContractJSON('HyperliquidInterchainToken');
    const token = new Contract(tokenAddress, HyperliquidInterchainToken.abi, wallet);

    const currentDeployer = await token.deployer();
    printInfo('Current deployer', currentDeployer);
    printInfo('New deployer', deployer);

    const serviceOwner = await service.owner();
    const isOperator = await service.isOperator(wallet.address);

    if (wallet.address.toLowerCase() !== serviceOwner.toLowerCase() && !isOperator) {
        throw new Error('Wallet does not have permission to update deployers. Must be service owner or operator.');
    }

    const gasOptions = await getGasOptions(chain, options, 'HyperliquidInterchainTokenService');

    const tx = await service.updateTokenDeployer(tokenId, deployer, gasOptions);
    await handleTx(tx, chain, service, 'updateTokenDeployer');

    const updatedDeployer = await token.deployer();
    printInfo('Updated deployer', updatedDeployer);
}

async function main(processor, args, options) {
    return mainProcessor(options, async (_config, chain, _chains, options) => {
        if (!isHyperliquidChain(chain)) {
            throw new Error(`Chain "${chain.name}" is not supported. This script only works on Hyperliquid chains.`);
        }

        const rpc = chain.rpc;
        const provider = getDefaultProvider(rpc);
        const wallet = new Wallet(options.privateKey, provider);

        await printWalletInfo(wallet);

        return await processor(wallet, chain, args, options);
    });
}

async function switchHyperliquidBlockSize(wallet, chain, args, options) {
    const [blockType] = args;
    const useBigBlocks = blockType === 'big';

    printInfo('Block size', blockType);

    const result = await updateBlockSize(wallet, chain, useBigBlocks);

    printInfo('Block size updated', result);
}

if (require.main === module) {
    const program = new Command();

    program.name('hyperliquid').description('Hyperliquid chain management commands');

    program
        .command('update-block-size')
        .addArgument(new Argument('<block-size>', 'block size to use').choices(['big', 'small']))
        .description('Update Hyperliquid block size')
        .action((blockSize, options) => {
            main(switchHyperliquidBlockSize, [blockSize], options);
        });

    program
        .command('deployer <token-id>')
        .description('Get deployer address for a Hyperliquid interchain token')
        .action((tokenId, options) => {
            main(deployer, [tokenId], options);
        });

    program
        .command('update-token-deployer <token-id> <deployer>')
        .description('Update deployer address for a Hyperliquid interchain token')
        .action((tokenId, deployer, options) => {
            main(updateTokenDeployer, [tokenId, deployer], options);
        });

    addOptionsToCommands(program, addEvmOptions);

    program.parse();
}

module.exports = { updateBlockSize };
