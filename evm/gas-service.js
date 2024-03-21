'use strict';

const { ethers } = require('hardhat');
const { getDefaultProvider, Contract } = ethers;
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

            const axelarBaseFee = Math.ceil(parseFloat(sourceBaseFee) * Math.pow(10, decimals));
            const relativeGasPrice = Math.ceil(parseFloat(gasPrice) * parseFloat(multiplier));
            const gasPriceRatio = parseFloat(destinationTokenPrice) / parseFloat(srcTokenPrice);
            const relativeBlobBaseFee = Math.ceil(blobBaseFee * gasPriceRatio);

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

async function processCommand(_config, chain, options) {
    const { env, contractName, address, action, privateKey, chains, destinationChain, destinationAddress, executionGasLimit, yes } =
        options;

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

    program.addOption(
        new Option('-c, --contractName <contractName>', 'contract name').default('AxelarGasService').makeOptionMandatory(false),
    );
    program.addOption(
        new Option('--action <action>', 'GasService action').choices(['estimateGasFee', 'updateGasInfo']).makeOptionMandatory(true),
    );
    program.addOption(new Option('--offline', 'run script in offline mode'));
    program.addOption(new Option('--nonceOffset <nonceOffset>', 'The value to add in local nonce if it deviates from actual wallet nonce'));

    // options for estimateGasFee
    program.addOption(new Option('--destinationChain <destinationChain>', 'Destination chain name').makeOptionMandatory(false));
    program.addOption(new Option('--destinationAddress <destinationAddress>', 'Destination contract address').makeOptionMandatory(false));
    program.addOption(new Option('--payload <payload>', 'Payload for the contract call').makeOptionMandatory(false));
    program.addOption(new Option('--executionGasLimit <executionGasLimit>', 'Execution gas limit').makeOptionMandatory(false));

    // options for updateGasInfo
    program.addOption(new Option('--chains <chains...>', 'Chain names').makeOptionMandatory(false));

    program.action((options) => {
        main(options);
    });

    program.parse();
}

exports.getGasUpdates = getGasUpdates;
exports.printFailedChainUpdates = printFailedChainUpdates;
