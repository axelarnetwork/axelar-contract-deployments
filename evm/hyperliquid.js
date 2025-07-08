'use strict';

const { Wallet, ethers, getDefaultProvider, Contract, AddressZero, BigNumber } = require('ethers');
const { exec } = require('child_process');
const { promisify } = require('util');
const { Command, Option } = require('commander');
const { printInfo, validateParameters, isHyperliquidChain, getContractJSON, getGasOptions, loadConfig, getChainConfig} = require('./utils');
const { addEvmOptions, addOptionsToCommands } = require('./cli-utils');
const { handleTx } = require('./its');
const execAsync = promisify(exec);
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

function constructPhantomAgent(hash, isMainnet) {
    return {
        source: isMainnet ? 'a' : 'b',
        connectionId: hash,
    };
}

async function signL1Action(wallet, action, activePool, nonce, isMainnet, chain) {
    const hash = actionHash(action, activePool, nonce);
    const phantomAgent = constructPhantomAgent(hash, isMainnet);

    const domain = chain.hypercore.domain;
    if (!domain) {
        throw new Error('hypercore domain information is required');
    }

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

    if (stderr) {
        throw new Error(stderr);
    }

    const result = JSON.parse(stdout);
    return result;
}

async function updateBlockSize(wallet, config, chain, args, options) {
    const [blockSize] = args;
    validateParameters({
        isNonEmptyString: { blockSize },
    });

    const useBig = blockSize === 'big';
    const network = chain.networkType;
    
    printInfo('Block size', blockSize.toUpperCase());
    printInfo('Network', network);

    const action = { type: 'evmUserModify', usingBigBlocks: useBig };
    const nonce = Date.now();
    const signature = await signL1Action(wallet, action, null, nonce, network === 'mainnet', chain);
    const result = await sendRequest(action, signature, nonce, chain);

    if (result.status !== 'ok') {
        throw new Error(result.response || result);
    }

    printInfo('Result', result);
    return result;
}

async function deployer(wallet, config, chain, args) {
    const [tokenId] = args;
    validateParameters({
        isNonEmptyString: { tokenId },
    });

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

async function updateTokenDeployer(wallet, config, chain, args, options) {
    const [tokenId, deployer] = args;
    validateParameters({
        isNonEmptyString: { tokenId },
        isValidAddress: { deployer },
    });

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

async function main(processor, args, options) {
    if (!options.env) {
        throw new Error('Environment was not provided');
    }

    if (!options.chainNames) {
        throw new Error('Chain names were not provided');
    }

    printInfo('Environment', options.env);

    const config = loadConfig(options.env);
    
    // Handle single chain name (take first if comma-separated)
    const chainName = options.chainNames.split(',')[0].trim();
    const chain = getChainConfig(config, chainName);
    
    if (!chain) {
        throw new Error(`Chain "${chainName}" is not defined in the config`);
    }

    if (!isHyperliquidChain(chain)) {
        throw new Error(`Chain "${chain.name}" is not supported. This script only works on Hyperliquid chains.`);
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);
    const wallet = new Wallet(options.privateKey, provider);

    await processor(wallet, config, chain, args, options);
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
        const result = await updateBlockSize(options.privateKey, config, chain, [blockType], options);

        if (result.status === 'ok') {
            if (useBigBlocks && gasOptions.gasLimit) {
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

if (require.main === module) {
    const program = new Command();

    program.name('hyperliquid').description('Hyperliquid chain management commands');

    program
        .command('update-block-size <blockSize>')
        .description('Update Hyperliquid block size')
        .action((blockSize, options) => {
            main(updateBlockSize, [blockSize], options);
        });

    program
        .command('deployer <tokenId>')
        .description('Get deployer address for a Hyperliquid interchain token')
        .action((tokenId, options) => {
            main(deployer, [tokenId], options);
        });

    program
        .command('update-token-deployer <tokenId> <deployer>')
        .description('Update deployer address for a Hyperliquid interchain token')
        .action((tokenId, deployer, options) => {
            main(updateTokenDeployer, [tokenId, deployer], options);
        });

    addOptionsToCommands(program, addEvmOptions, { privateKey: true });

    program.parse();
}

module.exports = { updateBlockSize, switchHyperliquidBlockSize, shouldUseBigBlocks, deployer, updateTokenDeployer };
