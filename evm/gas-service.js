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
    timeout,
} = require('./utils');
const { addBaseOptions } = require('./cli-utils');
const { getWallet } = require('./sign-utils');

let failedChainUpdates = [];

function addFailedChainUpdate(chain, destinationChain) {
    failedChainUpdates.push({ chain, destinationChain });
}

function printFailedChainUpdates() {
    if (failedChainUpdates.length > 0) {
        printError('Failed to update gas info for following chain combinations');

        failedChainUpdates.forEach(({ chain, destinationChain }) => {
            printError(`${chain} -> ${destinationChain}`);
        });

        failedChainUpdates = [];

        throw new Error('Failed to update gas info for the chain combinations above');
    }
}

const feesCache = {};

async function getFeeData(api, sourceChain, destinationChain) {
    const key = `${sourceChain}-${destinationChain}`;

    if (!feesCache[key]) {
        feesCache[key] = timeout(
            httpPost(`${api}/gmp/getFees`, {
                sourceChain,
                destinationChain,
                sourceTokenAddress: AddressZero,
            }),
            10000,
            new Error(`Timeout fetching fees for ${sourceChain} -> ${destinationChain}`),
        )
            .then(({ result }) => (feesCache[key] = result))
            .catch((e) => {
                delete feesCache[key];
                throw e;
            });
    }

    return feesCache[key];
}

async function getGasUpdates(config, env, chain, destinationChains) {
    const api = config.axelar.axelarscanApi;

    validateParameters({
        isNonEmptyStringArray: { destinationChains },
    });

    if (destinationChains.includes('all')) {
        destinationChains = Object.keys(config.chains);
    }

    const referenceChain = Object.values(config.chains)[0];

    let gasUpdates = await Promise.all(
        destinationChains.map(async (destinationChain) => {
            const destinationConfig = config.chains[destinationChain];

            if (!destinationConfig) {
                printError(`Error: chain ${destinationChain} not found in config.`);
                printError(`Skipping ${destinationChain}.`);
                addFailedChainUpdate(chain.axelarId, destinationChain);
                return null;
            }

            const {
                axelarId: destinationAxelarId,
                onchainGasEstimate: {
                    gasEstimationType = 0,
                    blobBaseFee = 0,
                    multiplier: onchainGasVolatilityMultiplier = 2,
                    l1FeeScalar = 0,
                } = {},
            } = destinationConfig;

            let destinationFeeData;
            let sourceFeeData;

            try {
                [sourceFeeData, destinationFeeData] = await Promise.all([
                    getFeeData(api, referenceChain.axelarId, chain.axelarId),
                    getFeeData(api, referenceChain.axelarId, destinationAxelarId),
                ]);
            } catch (e) {
                printError(`Error getting gas info for ${chain.axelarId} -> ${destinationAxelarId}`);
                printError(e);
                addFailedChainUpdate(chain.axelarId, destinationAxelarId);
                return null;
            }

            let {
                destination_native_token: {
                    symbol: srcSymbol,
                    token_price: { usd: srcTokenPrice },
                    decimals: srcTokenDecimals,
                },
                ethereum_token: {
                    token_price: { usd: ethPrice },
                },
            } = sourceFeeData;

            let {
                base_fee_usd: baseFeeUsd,
                destination_express_fee: { total_usd: expressFeeUsd },
                destination_native_token: {
                    symbol: destSymbol,
                    token_price: { usd: destTokenPrice },
                    gas_price_in_units: { value: gasPriceInDestToken },
                },
                execute_gas_multiplier: executeGasMultiplier = 1.1,
            } = destinationFeeData;

            if (srcSymbol === 'ETH' && srcTokenPrice === 100) {
                srcTokenPrice = ethPrice;
            }

            if (destSymbol === 'ETH' && destTokenPrice === 100) {
                destTokenPrice = ethPrice;
            }

            const axelarBaseFee = (parseFloat(baseFeeUsd) / parseFloat(srcTokenPrice)) * Math.pow(10, srcTokenDecimals);
            const expressFee = (parseFloat(expressFeeUsd) / parseFloat(srcTokenPrice)) * Math.pow(10, srcTokenDecimals);
            const gasPriceRatio = parseFloat(destTokenPrice) / parseFloat(srcTokenPrice);
            const relativeGasPrice =
                parseFloat(executeGasMultiplier) *
                parseFloat(onchainGasVolatilityMultiplier) *
                (parseFloat(gasPriceInDestToken) / gasPriceRatio);
            const relativeBlobBaseFee = blobBaseFee * gasPriceRatio;

            const gasInfo = {
                gasEstimationType,
                l1FeeScalar,
                axelarBaseFee,
                relativeGasPrice,
                relativeBlobBaseFee,
                expressFee,
            };
            Object.keys(gasInfo).forEach((key) => {
                gasInfo[key] = toBigNumberString(gasInfo[key]);
            });

            return {
                chain: destinationChain,
                gasInfo,
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

            if (chainsToUpdate.length === 0) {
                printWarn('No gas info updates found.');
                return;
            }

            printInfo('Collected gas info for the following chain names', chainsToUpdate.join(', '));

            if (prompt(`Update gas info?`, yes)) {
                return;
            }

            try {
                const tx = await timeout(
                    gasService.updateGasInfo(chainsToUpdate, gasInfoUpdates, gasOptions),
                    chain.timeout || 60000,
                    new Error(`Timeout updating gas info for ${chain.name}`),
                );

                printInfo('TX', tx.hash);

                const receipt = await tx.wait(chain.confirmations);

                const eventEmitted = wasEventEmitted(receipt, gasService, 'GasInfoUpdated');

                if (!eventEmitted) {
                    printWarn('Event not emitted in receipt.');
                }
            } catch (error) {
                for (let i = 0; i < chainsToUpdate.length; i++) {
                    addFailedChainUpdate(chain.name, chainsToUpdate[i]);
                }

                printError(error);
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
exports.addFailedChainUpdate = addFailedChainUpdate;
exports.printFailedChainUpdates = printFailedChainUpdates;
