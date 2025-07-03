'use strict';

const { Wallet } = require('ethers');
const { exec } = require('child_process');
const { promisify } = require('util');
const { Command, Option } = require('commander');
const { printInfo, printError, printWarn, validateParameters, mainProcessor, isHyperliquidChain } = require('./utils');
const { addEvmOptions } = require('./cli-utils');
const execAsync = promisify(exec);
const msgpack = require('msgpack-lite');

function addressToBytes(address) {
    return Buffer.from(address.replace('0x', ''), 'hex');
}

function actionHash(action, vaultAddress, nonce) {
    const actionData = msgpack.encode(action);
    const nonceBuffer = Buffer.alloc(8);
    nonceBuffer.writeBigUInt64BE(BigInt(nonce));

    let vaultBuffer;
    if (vaultAddress === null || vaultAddress === undefined) {
        vaultBuffer = Buffer.from([0x00]);
    } else {
        const addressBytes = addressToBytes(vaultAddress);
        vaultBuffer = Buffer.concat([Buffer.from([0x01]), addressBytes]);
    }

    const data = Buffer.concat([actionData, nonceBuffer, vaultBuffer]);
    const { keccak256 } = require('ethers/lib/utils');
    return keccak256(data);
}

function constructPhantomAgent(hash, isMainnet) {
    return {
        source: isMainnet ? 'a' : 'b',
        connectionId: hash,
    };
}

async function signL1Action(wallet, action, activePool, nonce, isMainnet) {
    const hash = actionHash(action, activePool, nonce);
    const phantomAgent = constructPhantomAgent(hash, isMainnet);

    const domain = {
        chainId: 1337,
        name: 'Exchange',
        verifyingContract: '0x0000000000000000000000000000000000000000',
        version: '1',
    };

    const types = {
        Agent: [
            { name: 'source', type: 'string' },
            { name: 'connectionId', type: 'bytes32' },
        ],
        EIP712Domain: [
            { name: 'name', type: 'string' },
            { name: 'version', type: 'string' },
            { name: 'chainId', type: 'uint256' },
            { name: 'verifyingContract', type: 'address' },
        ],
    };

    const signature = await wallet._signTypedData(domain, { Agent: types.Agent }, phantomAgent);
    const { ethers } = require('ethers');
    const sig = ethers.utils.splitSignature(signature);

    return { r: sig.r, s: sig.s, v: sig.v };
}

async function sendRequest(action, signature, nonce, isMainnet) {
    const payload = { action, signature, nonce };
    const endpoint = isMainnet ? 'https://api.hyperliquid.xyz/exchange' : 'https://api.hyperliquid-testnet.xyz/exchange';

    const curlCommand = `curl -s -X POST "${endpoint}" \
        -H "Content-Type: application/json" \
        -H "User-Agent: Mozilla/5.0 (compatible; Hyperliquid-Block-Helper/1.0)" \
        -d '${JSON.stringify(payload)}' \
        --connect-timeout 15 \
        --max-time 30`;

    const { stdout, stderr } = await execAsync(curlCommand);

    if (stderr && !stderr.includes('curl')) {
        throw new Error(`curl stderr: ${stderr}`);
    }

    const result = JSON.parse(stdout);
    return result;
}

async function switchBlockSize(privateKey, useBig, network = 'mainnet') {
    validateParameters({ isNonEmptyString: { privateKey } });

    if (!privateKey.startsWith('0x')) {
        privateKey = '0x' + privateKey;
    }

    if (privateKey.length !== 66) {
        throw new Error(`Invalid private key length: ${privateKey.length}`);
    }

    const wallet = new Wallet(privateKey);
    const isMainnet = network.toLowerCase() === 'mainnet';

    const action = { type: 'evmUserModify', usingBigBlocks: useBig };
    const nonce = Date.now();
    const signature = await signL1Action(wallet, action, null, nonce, isMainnet);
    const result = await sendRequest(action, signature, nonce, isMainnet);

    if (result.status === 'ok') {
        return result;
    } else {
        throw new Error(`API error: ${result.response}`);
    }
}

async function processCommand(config, chain, options) {
    const { privateKey, blockSize } = options;

    validateParameters({
        isNonEmptyString: { privateKey },
        isNonEmptyString: { blockSize },
    });

    const isHyperliquid = isHyperliquidChain(chain);

    if (!isHyperliquid) {
        throw new Error(`Chain "${chain.name}" is not supported. This script only works on Hyperliquid chains.`);
    }

    const network = options.env;
    const isMainnet = network === 'mainnet';
    const useBig = blockSize === 'big';

    printInfo('Block size', blockSize.toUpperCase());
    printInfo('Network', network);

    try {
        const result = await switchBlockSize(privateKey, useBig, network);
        printInfo('Result', result);
        return result;
    } catch (error) {
        if (error.message.includes('does not exist')) {
            printWarn('Result', 'Account not found, continuing without block size switch');
            return { status: 'ok', message: 'Account not found, continuing without block size switch' };
        }
        if (error.message.includes('curl request failed')) {
            printError('Result', 'Block size switch failed, continuing with deployment');
            return { status: 'ok', message: 'Block size switch failed, continuing with deployment' };
        }
        throw error;
    }
}

/**
 * Switches Hyperliquid block size and adjusts gas options accordingly
 * @param {Object} options - Deployment options
 * @param {Object} gasOptions - Gas options to modify
 * @param {boolean} useBigBlocks - Whether to switch to big blocks
 * @param {string} deploymentName - Name of the deployment for logging
 * @returns {Promise<boolean>} - Whether the switch was successful
 */
async function switchHyperliquidBlockSize(options, gasOptions, useBigBlocks, deploymentName) {
    const network = options.env;
    const isMainnet = network === 'mainnet';
    const blockType = useBigBlocks ? 'BIG' : 'SMALL';

    try {
        await switchBlockSize(options.privateKey, useBigBlocks, network);

        if (useBigBlocks && gasOptions.gasLimit) {
            const { BigNumber } = require('ethers');
            gasOptions.gasLimit = BigNumber.from(gasOptions.gasLimit).mul(2);
        }

        return true;
    } catch (error) {
        printWarn(`Failed to switch to ${blockType} blocks, continuing with deployment:`, error.message);
        return false;
    }
}

/**
 * Determines if a deployment should use big blocks on Hyperliquid
 * @param {string} key - Deployment key
 * @param {boolean} isHyperliquidChain - Whether this is a Hyperliquid chain
 * @returns {boolean} - Whether big blocks should be used
 */
function shouldUseBigBlocks(key, isHyperliquidChain) {
    if (!isHyperliquidChain) return false;

    // Use big blocks for large contract deployments
    return key === 'implementation' || key === 'interchainTokenFactoryImplementation';
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('HyperliquidBlockHelper').description('Script to switch Hyperliquid block sizes');

    addEvmOptions(program, { privateKey: true });

    program.addOption(new Option('--blockSize <blockSize>', 'block size to switch to').choices(['big', 'small']).makeOptionMandatory(true));

    program.action((options) => {
        main(options);
    });

    program.parse();
}

module.exports = { switchBlockSize, switchHyperliquidBlockSize, shouldUseBigBlocks };
