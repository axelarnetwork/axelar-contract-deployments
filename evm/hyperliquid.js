'use strict';

const { Wallet, ethers, getDefaultProvider } = require('ethers');
const { exec } = require('child_process');
const { promisify } = require('util');
const { Command, Option } = require('commander');
const { printInfo, printError, validateParameters, mainProcessor, isHyperliquidChain, getContractJSON, getGasOptions } = require('./utils');
const { addEvmOptions } = require('./cli-utils');
const { handleTx } = require('./its');
const execAsync = promisify(exec);
const msgpack = require('msgpack-lite');
const { keccak256 } = require('ethers/lib/utils');
const { Contract } = require('ethers');
const { AddressZero } = require('ethers');

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

function constructPhantomAgent(hash, isMainnet) {
    return {
        source: isMainnet ? 'a' : 'b',
        connectionId: hash,
    };
}

async function signL1Action(wallet, action, activePool, nonce, isMainnet, chain) {
    const hash = actionHash(action, activePool, nonce);
    const phantomAgent = constructPhantomAgent(hash, isMainnet);

    // Use chain.hypercore.domain if available, otherwise fall back to default
    const domain = chain?.hypercore?.domain;
    const agent = [
        { name: 'source', type: 'string' },
        { name: 'connectionId', type: 'bytes32' },
    ];

    const signature = await wallet._signTypedData(domain, { Agent: agent }, phantomAgent);
    const sig = ethers.utils.splitSignature(signature);

    return { r: sig.r, s: sig.s, v: sig.v };
}

async function sendRequest(action, signature, nonce, chain) {
    const payload = { action, signature, nonce };
    const endpoint = `${chain.hypercore.url}/exchange`;

    const curlCommand = `curl -s -X POST "${endpoint}" \
        -H "Content-Type: application/json" \
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

async function updateBlockSize(privateKey, useBig, network = 'mainnet', chain) {
    validateParameters({ isValidPrivateKey: { privateKey } });

    const wallet = new Wallet(privateKey);
    const isMainnet = network.toLowerCase() === 'mainnet';

    const action = { type: 'evmUserModify', usingBigBlocks: useBig };
    const nonce = Date.now();
    const signature = await signL1Action(wallet, action, null, nonce, isMainnet, chain);
    const result = await sendRequest(action, signature, nonce, chain);

    return result.status === 'ok'
        ? { success: true, data: result }
        : (() => {
              throw new Error(result.response || result);
          })();
}

async function processCommand(config, chain, options) {
    const { privateKey, action } = options;

    validateParameters({
        isNonEmptyString: { privateKey },
    });

    if (!isHyperliquidChain(chain)) {
        throw new Error(`Chain "${chain.name}" is not supported. This script only works on Hyperliquid chains.`);
    }

    if (!action) {
        throw new Error('Action is required. Use --action to specify what operation to perform.');
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);
    const wallet = new Wallet(privateKey, provider);

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
                const result = await updateBlockSize(privateKey, useBig, network, chain);
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

            await getTokenDeployer(config, chain, options, wallet);
            break;
        }
        case 'updateTokenDeployer': {
            const { tokenId, deployer } = options;

            validateParameters({
                isNonEmptyString: { tokenId },
                isValidAddress: { deployer },
            });

            await updateTokenDeployer(config, chain, options, wallet);
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
        const result = await updateBlockSize(options.privateKey, useBigBlocks, network, chain);

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
async function getTokenDeployer(config, chain, options, wallet) {
    const { privateKey, tokenId } = options;

    const contracts = chain.contracts;
    const interchainTokenFactoryAddress = contracts.InterchainTokenFactory?.address;
    const interchainTokenServiceAddress = contracts.InterchainTokenService?.address;

    validateParameters({
        isValidAddress: { interchainTokenFactoryAddress, interchainTokenServiceAddress },
    });

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
async function updateTokenDeployer(config, chain, options, wallet) {
    const { privateKey, tokenId, deployer } = options;

    const contracts = chain.contracts;
    const interchainTokenFactoryAddress = contracts.InterchainTokenFactory?.address;
    const interchainTokenServiceAddress = contracts.InterchainTokenService?.address;

    validateParameters({
        isValidAddress: { interchainTokenFactoryAddress, interchainTokenServiceAddress },
    });

    const IInterchainTokenService = getContractJSON('IInterchainTokenService');
    const interchainTokenService = new Contract(interchainTokenServiceAddress, IInterchainTokenService.abi, wallet);

    const tokenAddress = await interchainTokenService.registeredTokenAddress(tokenId);
    printInfo('Token address', tokenAddress);

    const ServiceContract = getContractJSON('HyperliquidInterchainTokenService');
    const service = new Contract(interchainTokenServiceAddress, ServiceContract.abi, wallet);

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

    program.name('hyperliquid').description('Hyperliquid chain management commands');

    // Update block size command
    const updateBlockSizeCmd = program
        .command('update-block-size')
        .description('Update Hyperliquid block size')
        .addOption(new Option('--block-size <blockSize>', 'block size to switch to').choices(['big', 'small']).makeOptionMandatory(true));

    addEvmOptions(updateBlockSizeCmd, { privateKey: true });

    updateBlockSizeCmd.action((options) => {
        options.action = 'updateBlockSize';
        main(options);
    });

    // Deployer command
    const deployerCmd = program.command('deployer').description('Get deployer address for a Hyperliquid interchain token');

    addEvmOptions(deployerCmd, { privateKey: true });
    deployerCmd.addOption(new Option('--tokenId <tokenId>', 'ID of the token').makeOptionMandatory(true));

    deployerCmd.action((options) => {
        options.action = 'deployer';
        main(options);
    });

    // Update token deployer command
    const updateTokenDeployerCmd = program
        .command('update-token-deployer')
        .description('Update deployer address for a Hyperliquid interchain token');

    addEvmOptions(updateTokenDeployerCmd, { privateKey: true });
    updateTokenDeployerCmd.addOption(new Option('--tokenId <tokenId>', 'ID of the token').makeOptionMandatory(true));
    updateTokenDeployerCmd.addOption(new Option('--deployer <deployer>', 'new deployer address').makeOptionMandatory(true));

    updateTokenDeployerCmd.action((options) => {
        options.action = 'updateTokenDeployer';
        main(options);
    });

    program.parse();
}

module.exports = { updateBlockSize, switchHyperliquidBlockSize, shouldUseBigBlocks, getTokenDeployer, updateTokenDeployer };
