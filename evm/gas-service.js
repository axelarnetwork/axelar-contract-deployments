'use strict';

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    Contract,
    constants: { AddressZero },
} = ethers;
const { Command, Option } = require('commander');
const {
    printInfo,
    printWalletInfo,
    printWarn,
    printError,
    mainProcessor,
    prompt,
    getContractJSON,
    getGasOptions,
    wasEventEmitted,
    isValidAddress,
    validateParameters,
    httpPost,
    toBigNumberString,
} = require('./utils');
const { addBaseOptions } = require('./cli-utils');
const { getWallet } = require('./sign-utils');

let failedChainUpdates = [];

async function getGasUpdates(config, env, chain, destinationChains) {
    const api = config.axelar.axelarscanApi;

    validateParameters({
        isNonEmptyStringArray: { destinationChains },
    });

    let gasUpdates = await Promise.all(
        destinationChains.map(async (destinationChain) => {
            const destinationConfig = config.chains[destinationChain];

            if (!destinationConfig) {
                printError(`Error: chain ${destinationChain} not found in config.`);
                printError(`Skipping ${destinationChain}.`);
                failedChainUpdates.push({ chain: chain.axelarId, destinationChain });
                return null;
            }

            const { axelarId, onchainGasEstimate: { gasEstimationType = 0, blobBaseFee = 0 } = {} } = destinationConfig;

            let data;

            try {
                data = await httpPost(`${api}/gmp/getFees`, {
                    sourceChain: chain.axelarId,
                    destinationChain: axelarId,
                    sourceTokenAddress: AddressZero,
                });
            } catch (e) {
                printError(`Error getting gas info for ${chain.axelarId} -> ${axelarId}`);
                printError(e);
                failedChainUpdates.push({ chain: chain.axelarId, destinationChain: axelarId });
                return null;
            }

            const {
                source_base_fee: sourceBaseFee,
                source_express_fee: { total: sourceExpressFee },
                source_token: {
                    gas_price_in_units: { value: gasPrice },
                    token_price: { usd: srcTokenPrice },
                    decimals,
                },
                destination_native_token: {
                    token_price: { usd: destinationTokenPrice },
                },
                execute_gas_multiplier: multiplier = 1.1,
            } = data.result;

            const axelarBaseFee = parseFloat(sourceBaseFee) * Math.pow(10, decimals);
            const expressFee = parseFloat(sourceExpressFee) * Math.pow(10, decimals);
            const relativeGasPrice = parseFloat(gasPrice) * parseFloat(multiplier);
            const gasPriceRatio = parseFloat(destinationTokenPrice) / parseFloat(srcTokenPrice);
            const relativeBlobBaseFee = blobBaseFee * gasPriceRatio;

            return {
                chain: destinationChain,
                gasInfo: [gasEstimationType, axelarBaseFee, expressFee, relativeGasPrice, relativeBlobBaseFee].map(toBigNumberString),
            };
        }),
    );

    gasUpdates = gasUpdates.filter((update) => update !== null);

    // Adding lowercase chain names for case insensitivity
    gasUpdates.forEach((update) => {
        const { chain: destination, gasInfo } = update;
        const { axelarId, onchainGasEstimate: { chainName } = {} } = config.chains[destination];

        update.chain = axelarId;

        // Adding lowercase chain names for case insensitivity
        if (axelarId.toLowerCase() !== axelarId) {
            gasUpdates.push({
                chain: axelarId.toLowerCase(),
                gasInfo,
            });
        }

        // Adding a duplicate entry for the specified chain name if it is different from axelarId
        // Allows to have `ethereum` entry for `ethereum-sepolia` chain
        if (chainName && chainName !== axelarId) {
            gasUpdates.push({
                chain: chainName,
                gasInfo,
            });

            // Adding lowercase chain names for case insensitivity
            if (chainName.toLowerCase() !== chainName) {
                gasUpdates.push({
                    chain: chainName.toLowerCase(),
                    gasInfo,
                });
            }
        }
    });

    return {
        chainsToUpdate: gasUpdates.map(({ chain }) => chain),
        gasInfoUpdates: gasUpdates.map(({ gasInfo }) => gasInfo),
    };
}

function printFailedChainUpdates() {
    if (failedChainUpdates.length > 0) {
        printError('Failed to update gas info for following chain combinations');

        failedChainUpdates.forEach(({ chain, destinationChain }) => {
            printError(`${chain} -> ${destinationChain}`);
        });
    }

    failedChainUpdates = [];
}

async function processCommand(config, chain, options) {
    const { env, contractName, address, action, privateKey, chains, destinationChain, destinationAddress, isExpress, yes } = options;
    const executionGasLimit = parseInt(options.executionGasLimit);

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName];

    let GasServiceAddress;

    if (isValidAddress(address)) {
        GasServiceAddress = address;
    } else {
        if (!contractConfig?.address) {
            throw new Error(`Contract ${contractName} is not deployed on ${chain.name}`);
        }

        GasServiceAddress = contractConfig.address;
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = await getWallet(privateKey, provider, options);
    await printWalletInfo(wallet, options);

    printInfo('Contract name', contractName);
    printInfo('Contract address', GasServiceAddress);

    const gasService = new Contract(GasServiceAddress, getContractJSON('IAxelarGasService').abi, wallet);

    const gasOptions = await getGasOptions(chain, options, contractName);

    printInfo('GasService Action', action);

    if (prompt(`Proceed with action ${action} on chain ${chain.name}?`, yes)) {
        return;
    }

    switch (action) {
        case 'estimateGasFee': {
            validateParameters({
                isNonEmptyString: { destinationChain },
                isValidAddress: { destinationAddress },
                isNumber: { executionGasLimit },
            });

            const payload = options.payload || '0x';

            const api = config.axelar.axelarscanApi;

            printInfo(`Estimating cross-chain gas fee from ${chain.axelarId} to ${destinationChain}`);

            if (api) {
                const estimate = await httpPost(`${api}/gmp/estimateGasFee`, {
                    sourceChain: chain.axelarId,
                    destinationChain,
                    sourceTokenAddress: AddressZero,
                    gasLimit: executionGasLimit,
                    executeData: payload,
                });

                printInfo('AxelarScan estimate ', estimate);
            }

            if (isExpress) {
                printInfo('Estimating express gas fee');
            }

            const gasEstimate = await gasService.estimateGasFee(destinationChain, destinationAddress, payload, executionGasLimit, '0x');

            printInfo('GasService estimate ', gasEstimate.toString());
            printInfo('-'.repeat(50));

            break;
        }

        case 'updateGasInfo': {
            validateParameters({
                isNonEmptyStringArray: { chains },
            });

            const { chainsToUpdate, gasInfoUpdates } = await getGasUpdates(config, env, chain, chains);

            if (prompt(`Update gas info for following chains ${chainsToUpdate.join(', ')}?`, yes)) {
                return;
            }

            const tx = await gasService.updateGasInfo(chainsToUpdate, gasInfoUpdates, gasOptions);

            printInfo('TX', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = wasEventEmitted(receipt, gasService, 'GasInfoUpdated');

            if (!eventEmitted) {
                printWarn('Event not emitted in receipt.');
            }

            break;
        }

        default:
            throw new Error(`Unknown action: ${action}`);
    }
}

async function main(options) {
    await mainProcessor(options, processCommand, false);

    printFailedChainUpdates();
}

if (require.main === module) {
    const program = new Command();

    program.name('GasService').description('Script to manage GasService actions');

    addBaseOptions(program, { address: true });

    program.addOption(new Option('-c, --contractName <contractName>', 'contract name').default('AxelarGasService'));
    program.addOption(
        new Option('--action <action>', 'GasService action').choices(['estimateGasFee', 'updateGasInfo']).makeOptionMandatory(true),
    );
    program.addOption(new Option('--offline', 'run script in offline mode'));
    program.addOption(new Option('--nonceOffset <nonceOffset>', 'The value to add in local nonce if it deviates from actual wallet nonce'));

    // options for estimateGasFee
    program.addOption(new Option('--destinationChain <destinationChain>', 'Destination chain name'));
    program.addOption(new Option('--destinationAddress <destinationAddress>', 'Destination contract address'));
    program.addOption(new Option('--payload <payload>', 'Payload for the contract call').env('PAYLOAD'));
    program.addOption(new Option('--executionGasLimit <executionGasLimit>', 'Execution gas limit'));
    program.addOption(new Option('--isExpress', 'Estimate express gas fee'));

    // options for updateGasInfo
    program.addOption(new Option('--chains <chains...>', 'Chain names'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}

exports.getGasUpdates = getGasUpdates;
exports.printFailedChainUpdates = printFailedChainUpdates;
