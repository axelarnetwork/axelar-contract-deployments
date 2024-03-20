'use strict';

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    Contract,
} = ethers;
const { Command, Option } = require('commander');
const {
    printInfo,
    printWalletInfo,
    printWarn,
    mainProcessor,
    prompt,
    getGasOptions,
    wasEventEmitted,
    isValidAddress,
    validateParameters,
} = require('./utils');
const { addBaseOptions } = require('./cli-utils');
const IAxelarGasService = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IAxelarGasService.json');
const { getWallet } = require('./sign-utils');

async function getGasInfo(env, chain, destinationChains) {
    const apiUrl = env === 'testnet' ? 'https://testnet.api.axelarscan.io/gmp/getFees' : 'https://api.axelarscan.io/gmp/getFees';

    return Promise.all(
        destinationChains.map((destinationChain) => {
            let feeType = 0;
            let blobBaseFee = 0;

            switch (destinationChain.toLowerCase()) {
                case 'ethereum-sepolia':

                // eslint-disable-next-line no-fallthrough
                case 'ethereum': {
                    feeType = 0;
                    blobBaseFee = env === 'testnet' ? 50187563959 : 1;
                    break;
                }

                case 'base':

                // eslint-disable-next-line no-fallthrough
                case 'optimism': {
                    feeType = 1; // Optimism Ecotone
                    break;
                }
            }

            return fetch(apiUrl, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    sourceChain: chain.axelarId,
                    destinationChain,
                    sourceTokenAddress: '0x0000000000000000000000000000000000000000',
                }),
            })
                .then((res) => res.json())
                .then(({ result }) => {
                    const {
                        source_base_fee: sourceBaseFee,
                        source_token: {
                            gas_price_in_units: { value: gasPrice },
                            decimals,
                        },
                        execute_gas_multiplier: multiplier = 1.1,
                    } = result;

                    const axelarBaseFee = Math.ceil(parseFloat(sourceBaseFee) * Math.pow(10, decimals));
                    const relativeGasPrice = Math.ceil(parseFloat(gasPrice) * parseFloat(multiplier));

                    return [feeType, axelarBaseFee, relativeGasPrice, blobBaseFee * relativeGasPrice];
                });
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
    const { address: walletAddress } = await printWalletInfo(wallet, options);

    printInfo('Contract name', contractName);
    printInfo('Contract address', GasServiceAddress);

    const gasService = new Contract(GasServiceAddress, IAxelarGasService.abi, wallet);

    const gasOptions = await getGasOptions(chain, options, contractName);

    printInfo('GasService Action', action);
    printInfo('Wallet', walletAddress);

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

            const gasUpdates = await getGasInfo(env, chain, chains);

            // Adding lowecase chain names for case insensitivity
            chains.forEach((destination, i) => {
                if (destination.toLowerCase() !== destination) {
                    chains.push(destination.toLowerCase());
                    gasUpdates.push(gasUpdates[i]);
                }
            });

            console.log({ chains, gasUpdates });

            const tx = await gasService.updateGasInfo(chains, gasUpdates, gasOptions);

            printInfo('Call updateGasInfo', tx.hash);

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
