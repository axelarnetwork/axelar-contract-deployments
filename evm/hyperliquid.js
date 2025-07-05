'use strict';

const { Wallet, ethers } = require('ethers');
const { exec } = require('child_process');
const { promisify } = require('util');
const { Command, Option } = require('commander');
const {
    printInfo,
    printError,
    validateParameters,
    mainProcessor,
    isHyperliquidChain,
    getContractJSON,
    getGasOptions,
    getDefaultProvider,
} = require('./utils');
const { addEvmOptions } = require('./cli-utils');
const { handleTx } = require('./its');
const execAsync = promisify(exec);
const msgpack = require('msgpack-lite');
const { keccak256 } = require('ethers/lib/utils');
const { Contract } = require('ethers');
const { AddressZero } = require('ethers');

const HYPERLIQUID_CONFIG = {
    domain: {
        chainId: 1337,
        name: 'Exchange',
        verifyingContract: '0x0000000000000000000000000000000000000000',
        version: '1',
    },
    types: {
        Agent: [
            { name: 'source', type: 'string' },
            { name: 'connectionId', type: 'bytes32' },
        ],
    },
    endpoints: {
        mainnet: 'https://api.hyperliquid.xyz/exchange',
        testnet: 'https://api.hyperliquid-testnet.xyz/exchange',
    },
    userAgent: 'Mozilla/5.0 (compatible; Hyperliquid-Block-Helper/1.0)',
};

function addressToBytes(address) {
    return Buffer.from(address.replace('0x', ''), 'hex');
}

function actionHash(action, activePool, nonce) {
    const actionData = msgpack.encode(action);
    const nonceBuffer = Buffer.alloc(8);
    nonceBuffer.writeBigUInt64BE(BigInt(nonce));

    let vaultBuffer;
    if (activePool === null || activePool === undefined) {
        vaultBuffer = Buffer.from([0x00]);
    } else {
        const addressBytes = addressToBytes(activePool);
        vaultBuffer = Buffer.concat([Buffer.from([0x01]), addressBytes]);
    }

    const data = Buffer.concat([actionData, nonceBuffer, vaultBuffer]);
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

    const signature = await wallet._signTypedData(HYPERLIQUID_CONFIG.domain, { Agent: HYPERLIQUID_CONFIG.types.Agent }, phantomAgent);
    const sig = ethers.utils.splitSignature(signature);

    return { r: sig.r, s: sig.s, v: sig.v };
}

async function sendRequest(action, signature, nonce, isMainnet) {
    const payload = { action, signature, nonce };
    const endpoint = isMainnet ? HYPERLIQUID_CONFIG.endpoints.mainnet : HYPERLIQUID_CONFIG.endpoints.testnet;

    const curlCommand = `curl -s -X POST "${endpoint}" \
        -H "Content-Type: application/json" \
        -H "User-Agent: ${HYPERLIQUID_CONFIG.userAgent}" \
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

async function updateBlockSize(privateKey, useBig, network = 'mainnet') {
    validateParameters({ isValidPrivateKey: { privateKey } });

    const wallet = new Wallet(privateKey);
    const isMainnet = network.toLowerCase() === 'mainnet';

    const action = { type: 'evmUserModify', usingBigBlocks: useBig };
    const nonce = Date.now();
    const signature = await signL1Action(wallet, action, null, nonce, isMainnet);
    const result = await sendRequest(action, signature, nonce, isMainnet);

    if (result.status === 'ok') {
        return { success: true, data: result };
    } else {
        if (result.response && result.response.includes('does not exist')) {
            printWarn('API Response', 'Account not found, continuing without block size switch');
            return { success: false, error: result.response || result };
        } else {
            printError('API Response Error', result.response || result);
            throw new Error(`Block size switch failed: ${result.response || result}`);
        }
    }
}

async function processCommand(config, chain, options) {
    const { privateKey, action } = options;

    validateParameters({
        isNonEmptyString: { privateKey },
    });

    const isHyperliquid = isHyperliquidChain(chain);

    if (!isHyperliquid) {
        throw new Error(`Chain "${chain.name}" is not supported. This script only works on Hyperliquid chains.`);
    }

    if (!action) {
        throw new Error('Action is required. Use --action to specify what operation to perform.');
    }

    const network = chain.networkType;

    switch (action) {
        case 'updateBlockSize': {
            const { blockSize } = options;
            validateParameters({
                isNonEmptyString: { blockSize },
            });

            const useBig = blockSize === 'big';
            printInfo('Block size', blockSize.toUpperCase());
            printInfo('Network', network);

            try {
                const result = await updateBlockSize(privateKey, useBig, network);
                if (result.success) {
                    printInfo('Result', result.data);
                    return result.data;
                } else {
                    throw new Error(`Block size switch failed: ${result.error}`);
                }
            } catch (error) {
                throw error;
            }
        }
        case 'deployer': {
            const { tokenId } = options;

            validateParameters({
                isNonEmptyString: { tokenId },
            });

            printInfo('Switching to big blocks for deployer query');
            await updateBlockSize(privateKey, true, network);

            try {
                await getTokenDeployer(config, chain, options);
            } finally {
                printInfo('Switching back to small blocks');
                await updateBlockSize(privateKey, false, network);
            }
            break;
        }
        case 'updateTokenDeployer': {
            const { tokenId, deployer } = options;

            validateParameters({
                isNonEmptyString: { tokenId },
                isValidAddress: { deployer },
            });

            printInfo('Switching to big blocks for deployer update');
            await updateBlockSize(privateKey, true, network);

            try {
                await updateTokenDeployer(config, chain, options);
            } finally {
                printInfo('Switching back to small blocks');
                await updateBlockSize(privateKey, false, network);
            }
            break;
        }
        default: {
            throw new Error(`Unknown action: ${action}`);
        }
    }
}

/**
 * Switches Hyperliquid block size and adjusts gas options accordingly
 * @param {Object} options - Deployment options
 * @param {Object} gasOptions - Gas options to modify
 * @param {boolean} useBigBlocks - Whether to switch to big blocks
 * @param {Object} chain - Chain configuration object
 * @returns {Promise<boolean>} - Whether the switch was successful
 */
async function switchHyperliquidBlockSize(options, gasOptions, useBigBlocks, chain) {
    const network = chain.networkType;
    const blockType = useBigBlocks ? 'BIG' : 'SMALL';

    try {
        const result = await updateBlockSize(options.privateKey, useBigBlocks, network);

        if (result.success) {
            if (useBigBlocks && gasOptions.gasLimit) {
                const { BigNumber } = require('ethers');
                gasOptions.gasLimit = BigNumber.from(gasOptions.gasLimit).mul(2);
            }
            return true;
        } else {
            throw new Error(`Failed to switch to ${blockType} blocks: ${result.error}`);
        }
    } catch (error) {
        throw error;
    }
}

/**
 * Determines if a deployment should use big blocks on Hyperliquid
 * @param {string} key - Deployment key
 * @returns {boolean} - Whether big blocks should be used
 */
function shouldUseBigBlocks(key) {
    return key === 'implementation' || key === 'interchainTokenFactoryImplementation';
}

/**
 * Gets the deployer address for a Hyperliquid interchain token
 * @param {Object} config - Configuration object
 * @param {Object} chain - Chain configuration
 * @param {Object} options - Command options
 * @returns {Promise<void>}
 */
async function getTokenDeployer(config, chain, options) {
    const { privateKey, address, tokenId } = options;

    const contracts = chain.contracts;
    const interchainTokenFactoryAddress = address || contracts.InterchainTokenFactory?.address;
    const interchainTokenServiceAddress = contracts.InterchainTokenService?.address;

    validateParameters({
        isValidAddress: { interchainTokenFactoryAddress, interchainTokenServiceAddress },
    });

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);
    const wallet = new Wallet(privateKey, provider);

    const IInterchainTokenFactory = getContractJSON('IInterchainTokenFactory');
    const IInterchainTokenService = getContractJSON('IInterchainTokenService');

    const interchainTokenFactory = new Contract(interchainTokenFactoryAddress, IInterchainTokenFactory.abi, wallet);
    const interchainTokenService = new Contract(interchainTokenServiceAddress, IInterchainTokenService.abi, wallet);

    const tokenAddress = await interchainTokenService.registeredTokenAddress(tokenId);
    printInfo('Token address', tokenAddress);

    try {
        const TokenContract = getContractJSON('HyperliquidInterchainToken');
        const token = new Contract(tokenAddress, TokenContract.abi, wallet);

        const currentDeployer = await token.deployer();
        printInfo('Current deployer', currentDeployer);
    } catch (error) {
        if (error.message.includes('deployer is not a function') || error.message.includes('execution reverted')) {
            const factoryDeployer = await interchainTokenFactory.getTokenDeployer(tokenId);
            if (factoryDeployer !== AddressZero) {
                printInfo('Factory deployer', factoryDeployer);
            } else {
                throw new Error('Token does not support deployer retrieval and no factory record found');
            }
        } else {
            throw error;
        }
    }
}

/**
 * Updates the deployer address for a Hyperliquid interchain token
 * @param {Object} config - Configuration object
 * @param {Object} chain - Chain configuration
 * @param {Object} options - Command options
 * @returns {Promise<void>}
 */
async function updateTokenDeployer(config, chain, options) {
    const { privateKey, address, tokenId, deployer } = options;

    const contracts = chain.contracts;
    const interchainTokenFactoryAddress = address || contracts.InterchainTokenFactory?.address;
    const interchainTokenServiceAddress = contracts.InterchainTokenService?.address;

    validateParameters({
        isValidAddress: { interchainTokenFactoryAddress, interchainTokenServiceAddress },
    });

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);
    const wallet = new Wallet(privateKey, provider);

    const IInterchainTokenService = getContractJSON('IInterchainTokenService');
    const interchainTokenService = new Contract(interchainTokenServiceAddress, IInterchainTokenService.abi, wallet);

    const tokenAddress = await interchainTokenService.registeredTokenAddress(tokenId);
    printInfo('Token address', tokenAddress);

    const ServiceContract = getContractJSON('HyperliquidInterchainTokenService');
    const service = new Contract(interchainTokenServiceAddress, ServiceContract.abi, wallet);

    const hasUpdateFunction = serviceContract.interface.functions.hasOwnProperty('updateTokenDeployer');
    if (!hasUpdateFunction) {
        printError('Service contract does not support updateTokenDeployer');
    }

    const TokenContract = getContractJSON('HyperliquidInterchainToken');
    const token = new Contract(tokenAddress, TokenContract.abi, wallet);

    const currentDeployer = await token.deployer();
    printInfo('Current deployer', currentDeployer);
    printInfo('New deployer', deployer);

    const serviceOwner = await service.owner();
    const isOperator = await service.isOperator(wallet.address);

    if (wallet.address.toLowerCase() !== serviceOwner.toLowerCase() && !isOperator) {
        throw new Error('Wallet does not have permission to update deployers. Must be service owner or operator.');
    }

    const gasOptions = await getGasOptions(chain, options, 'InterchainTokenService');
    const tx = await service.updateTokenDeployer(tokenId, deployer, gasOptions);
    await handleTx(tx, chain, service, 'updateTokenDeployer');

    const updatedDeployer = await token.deployer();
    printInfo('Updated deployer', updatedDeployer);
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('HyperliquidBlockHelper').description('Script to manage Hyperliquid specific actions');

    addEvmOptions(program, { privateKey: true });

    program.addOption(
        new Option('--action <action>', 'action to perform')
            .choices(['updateBlockSize', 'deployer', 'updateTokenDeployer'])
            .makeOptionMandatory(true),
    );
    program.addOption(
        new Option('--blockSize <blockSize>', 'block size to switch to (required for updateBlockSize action)').choices(['big', 'small']),
    );
    program.addOption(new Option('--address <address>', 'contract address'));
    program.addOption(new Option('--tokenId <tokenId>', 'ID of the token'));
    program.addOption(new Option('--deployer <deployer>', 'deployer address'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}

module.exports = { updateBlockSize, switchHyperliquidBlockSize, shouldUseBigBlocks, getTokenDeployer, updateTokenDeployer };
