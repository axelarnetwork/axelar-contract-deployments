'use strict';

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    BigNumber,
    FixedNumber,
    Contract,
    constants: { AddressZero },
    utils: { formatEther, parseEther },
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
} = require('./utils');
const { addBaseOptions } = require('./cli-utils');
const { getWallet } = require('./sign-utils');

let failedChainUpdates = [];

async function getGasUpdates(config, env, chain, destinationChains) {
    const api = config.axelar.axelarscanApi;

    validateParameters({
        isNonEmptyStringArray: { destinationChains },
    });

    return Promise.all(
        destinationChains.map(async (destinationChain) => {
            const destinationConfig = config.chains[destinationChain];

            if (!destinationConfig) {
                printError(`Error: chain ${destinationChain} not found in config.`);
                printError(`Skipping ${destinationChain}.`);
                failedChainUpdates.push({ chain: chain.axelarId, destinationChain });
                return null;
            }

            const { axelarId, onchainGasEstimate: { chainName = axelarId, gasEstimationType = 0, blobBaseFee = 0 } = {} } =
                destinationConfig;

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

            // sourceBaseFee * 10 ^ decimals
            const axelarBaseFee = FixedNumber.from(parseFloat(sourceBaseFee).toFixed(10))
                .mulUnsafe(FixedNumber.from(Math.pow(10, decimals).toFixed(10)))
                .round();
            // gasPrice * multiplier
            const relativeGasPrice = FixedNumber.from(parseFloat(gasPrice))
                .mulUnsafe(FixedNumber.from(parseFloat(multiplier).toFixed(10)))
                .round();
            // destinationTokenPrice / srcTokenPrice
            const gasPriceRatio = FixedNumber.from(parseFloat(destinationTokenPrice).toFixed(10)).divUnsafe(
                FixedNumber.from(parseFloat(srcTokenPrice).toFixed(10)),
            );
            // blobBaseFee * gasPriceRatio
            const relativeBlobBaseFee = FixedNumber.from(parseFloat(blobBaseFee)).mulUnsafe(gasPriceRatio);

            return {
                chainName,
                gasInfo: [gasEstimationType, axelarBaseFee, relativeGasPrice, relativeBlobBaseFee],
            };
        }),
    );
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
    const {
        env,
        contractName,
        address,
        action,
        privateKey,

        txHash,
        logIndex,

        receiver,
        token,
        amount,

        chains,

        collectorReceiver,
        collectTokens,
        collectAmounts,

        gasToken,
        gasFeeAmount,
        refundAddress,

        destinationChain,
        destinationAddress,
        executionGasLimit,

        yes,
    } = options;

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

            const gasEstimate = await gasService.estimateGasFee(destinationChain, destinationAddress, payload, executionGasLimit);

            printInfo('Gas Estimate', gasEstimate.toString());

            break;
        }

        case 'updateGasInfo': {
            validateParameters({
                isNonEmptyStringArray: { chains },
            });

            let gasUpdates = await getGasUpdates(config, env, chain, chains);

            gasUpdates = gasUpdates.filter((update) => update !== null);

            // Adding lowercase chain names for case insensitivity
            gasUpdates.forEach(({ chainName, gasInfo }) => {
                if (chainName.toLowerCase() !== chainName) {
                    gasUpdates.push({
                        chainName: chainName.toLowerCase(),
                        gasInfo,
                    });
                }
            });

            const filteredChains = gasUpdates.map(({ chainName }) => chainName);
            const gasInfoUpdates = gasUpdates.map(({ gasInfo }) => gasInfo);

            if (prompt(`Update gas info for following chains ${filteredChains}?`, yes)) {
                return;
            }

            const tx = await gasService.updateGasInfo(filteredChains, gasInfoUpdates, gasOptions);

            printInfo('TX', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = wasEventEmitted(receipt, gasService, 'GasInfoUpdated');

            if (!eventEmitted) {
                printWarn('Event not emitted in receipt.');
            }

            break;
        }

        case 'refund': {
            validateParameters({
                isKeccak256Hash: { txHash },
                isNumber: { logIndex, amount },
                isValidAddress: { receiver, token },
            });

            const refundAmount = parseEther(amount);

            const balance = await provider.getBalance(gasService.address);

            if (balance.lt(refundAmount)) {
                throw new Error(
                    `Contract balance ${formatEther(BigNumber.from(balance))} is less than refund amount: ${formatEther(
                        BigNumber.from(refundAmount),
                    )}`,
                );
            }

            const tx = await gasService.refund(txHash, logIndex, receiver, token, refundAmount, gasOptions);

            printInfo('Call refund', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = wasEventEmitted(receipt, gasService, 'Refunded');

            if (!eventEmitted) {
                printWarn('Event not emitted in receipt.');
            }

            break;
        }

        case 'collectFees': {
            validateParameters({
                isValidAddress: { collectorReceiver },
                isNonEmptyAddressArray: { collectTokens },
                isNonEmptyNumberArray: { collectAmounts },
            });

            const tx = await gasService.collectFees(collectorReceiver, collectTokens, collectAmounts, gasOptions);

            printInfo('Call collectFees', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = wasEventEmitted(receipt, gasService, 'FeesCollected');

            if (!eventEmitted) {
                printWarn('Event not emitted in receipt.');
            }

            break;
        }

        case 'addGas': {
            validateParameters({
                isKeccak256Hash: { txHash },
                isNumber: { logIndex, gasFeeAmount },
                isValidAddress: { gasToken, refundAddress },
            });

            const tx = await gasService.addGas(txHash, logIndex, gasToken, gasFeeAmount, refundAddress, gasOptions);

            printInfo('Call addGas', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = wasEventEmitted(receipt, gasService, 'GasAdded');

            if (!eventEmitted) {
                printWarn('Event not emitted in receipt.');
            }

            break;
        }

        case 'addNativeGas': {
            validateParameters({
                isKeccak256Hash: { txHash },
                isNumber: { logIndex },
                isValidAddress: { refundAddress },
            });

            const tx = await gasService.addNativeGas(txHash, logIndex, refundAddress, { ...gasOptions, value: amount });

            printInfo('Call addNativeGas', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = wasEventEmitted(receipt, gasService, 'NativeGasAdded');

            if (!eventEmitted) {
                printWarn('Event not emitted in receipt.');
            }

            break;
        }

        case 'addExpressGas': {
            validateParameters({
                isKeccak256Hash: { txHash },
                isNumber: { logIndex, gasFeeAmount },
                isValidAddress: { gasToken, refundAddress },
            });

            const tx = await gasService.addExpressGas(txHash, logIndex, gasToken, gasFeeAmount, refundAddress, gasOptions);

            printInfo('Call addExpressGas', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = wasEventEmitted(receipt, gasService, 'ExpressGasAdded');

            if (!eventEmitted) {
                printWarn('Event not emitted in receipt.');
            }

            break;
        }

        case 'addNativeExpressGas': {
            validateParameters({
                isKeccak256Hash: { txHash },
                isNumber: { logIndex },
                isValidAddress: { refundAddress },
            });

            const tx = await gasService.addNativeExpressGas(txHash, logIndex, refundAddress, { ...gasOptions, value: amount });

            printInfo('Call addNativeExpressGas', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = wasEventEmitted(receipt, gasService, 'NativeExpressGasAdded');

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
        new Option('--action <action>', 'GasService action')
            .choices([
                'estimateGasFee',
                'updateGasInfo',
                'refund',
                'collectFees',
                'addGas',
                'addNativeGas',
                'addExpressGas',
                'addNativeExpressGas',
            ])
            .makeOptionMandatory(true),
    );
    program.addOption(new Option('--offline', 'run script in offline mode'));
    program.addOption(new Option('--nonceOffset <nonceOffset>', 'The value to add in local nonce if it deviates from actual wallet nonce'));

    // common options
    program.addOption(new Option('--txHash <txHash>', 'Transaction hash').makeOptionMandatory(false));
    program.addOption(new Option('--logIndex <logIndex>', 'Log index').makeOptionMandatory(false));
    program.addOption(new Option('--receiver <receiver>', 'Receiver address').makeOptionMandatory(false));

    // options for estimateGasFee
    program.addOption(new Option('--destinationChain <destinationChain>', 'Destination chain name'));
    program.addOption(new Option('--destinationAddress <destinationAddress>', 'Destination contract address'));
    program.addOption(new Option('--payload <payload>', 'Payload for the contract call').env('PAYLOAD'));
    program.addOption(new Option('--executionGasLimit <executionGasLimit>', 'Execution gas limit'));

    // options for updateGasInfo
    program.addOption(new Option('--chains <chains...>', 'Chain names'));

    // options for refund
    program.addOption(new Option('--token <token>', 'Refund token address').makeOptionMandatory(false));
    program.addOption(new Option('--amount <amount>', 'Refund amount').makeOptionMandatory(false));

    // options for collectFees
    program.addOption(new Option('--collectorReceiver <collectorReceiver>', 'Collector receiver address').makeOptionMandatory(false));
    program.addOption(new Option('--collectTokens <collectTokens...>', 'Tokens to collect').makeOptionMandatory(false));
    program.addOption(new Option('--collectAmounts <collectAmounts...>', 'Amounts to collect').makeOptionMandatory(false));

    // options for adding gas
    program.addOption(new Option('--gasToken <gasToken>', 'Gas token address').makeOptionMandatory(false));
    program.addOption(new Option('--gasFeeAmount <gasFeeAmount>', 'Gas fee amount').makeOptionMandatory(false));
    program.addOption(new Option('--refundAddress <refundAddress>', 'Refund address').makeOptionMandatory(false));

    program.action((options) => {
        main(options);
    });

    program.parse();
}

exports.getGasUpdates = getGasUpdates;
exports.printFailedChainUpdates = printFailedChainUpdates;
