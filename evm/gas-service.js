'use strict';

const { ethers } = require('hardhat');
const { getDefaultProvider, Contract } = ethers;
const { Command, Option } = require('commander');
const {
    printInfo,
    printWalletInfo,
    printWarn,
    mainProcessor,
    prompt,
    getContractJSON,
    getGasOptions,
    wasEventEmitted,
    isValidAddress,
    validateParameters,
    httpPost,
    loadConfig,
} = require('./utils');
const { addBaseOptions } = require('./cli-utils');
const { getWallet } = require('./sign-utils');

async function getGasUpdates(env, chain, destinationChains) {
    const config = loadConfig(env);
    const api = config.axelar.api;

    return Promise.all(
        destinationChains.map(async (destinationChain) => {
            const destinationConfig = config.chains[destinationChain];

            if (!destinationConfig) {
                printInfo(`Error: chain ${destinationChain} not found in config.`);
                printInfo(`Skipping ${destinationChain}.`);
                return null;
            }

            const { gasEstimationType = 0, blobBaseFee = 0 } = destinationConfig;

            const data = await httpPost(`${api}/gmp/getFees`, {
                sourceChain: chain.axelarId,
                destinationChain,
                sourceTokenAddress: '0x0000000000000000000000000000000000000000',
            });

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

            return [gasEstimationType, axelarBaseFee, relativeGasPrice, relativeBlobBaseFee];
        }),
    );
}

async function processCommand(_, chain, options) {
    const {
        env,
        contractName,
        address,
        action,
        privateKey,

        chains,

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

            let gasUpdates = await getGasUpdates(env, chain, chains);

            const filteredChains = chains.filter((chain, i) => gasUpdates[i] !== null);
            gasUpdates = gasUpdates.filter((update) => update !== null);

            // Adding lowercase chain names for case insensitivity
            filteredChains.forEach((destination, i) => {
                if (destination.toLowerCase() !== destination) {
                    filteredChains.push(destination.toLowerCase());
                    gasUpdates.push(gasUpdates[i]);
                }
            });

            const tx = await gasService.updateGasInfo(filteredChains, gasUpdates, gasOptions);

            printInfo('Call updateGasInfo with following chains', filteredChains);
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
