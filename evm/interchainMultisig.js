'use strict';

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { arrayify, keccak256, formatEther, formatBytes32String, hashMessage, recoverAddress },
    Contract,
    BigNumber,
} = ethers;
const { sortBy } = require('lodash');
const { Command, Option } = require('commander');
const {
    printInfo,
    printWalletInfo,
    printWarn,
    isValidAddress,
    mainProcessor,
    prompt,
    getGasOptions,
    saveConfig, validateParameters,
} = require('./utils');
const { addBaseOptions } = require('./cli-utils');
const IInterchainMultisig = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IInterchainMultisig.json');
const IGateway = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IAxelarGateway.json');
const IGovernance = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IAxelarServiceGovernance.json');
const IInterchainTokenService = require('@axelar-network/interchain-token-service/interfaces/IInterchainTokenService.json');
const ITokenManager = require('@axelar-network/interchain-token-service/interfaces/ITokenManager.json');
const IOperator = require('@axelar-network/interchain-token-service/interfaces/IOperator.json');
const { parseEther } = require('ethers/lib/utils');
const { getWallet } = require('./sign-utils');
const {
    getWeightedSignersSet,
    sortWeightedSignaturesProof,
    encodeInterchainCallsBatch,
} = require('@axelar-network/axelar-gmp-sdk-solidity/scripts/utils');

async function preExecutionChecks(multisigContract, action, wallet, batchId, calls, signers, weights, threshold, signatures) {
    const signerEpoch = await multisigContract.epoch();
    const signerHash = await multisigContract.signerHashByEpoch(signerEpoch);

    if (signerHash !== keccak256(getWeightedSignersSet(signers, weights, threshold))) {
        throw new Error('Invalid signers: the hash of the signers set does not match the one on the contract');
    }

    validateParameters({ isNonEmptyStringArray: { signatures }});

    const callsBatchData = encodeInterchainCallsBatch(batchId, calls);
    const messageHash = arrayify(hashMessage(arrayify(keccak256(callsBatchData))));

    signers = signers.map((address) => address.toLowerCase());
    const signatureAddresses = signatures.map((signature) => recoverAddress(messageHash, signature).toLowerCase());

    const wrongSignatureAddresses = signatureAddresses.filter((address) => !signers.includes(address));

    if (wrongSignatureAddresses.length > 0) {
        throw new Error(
            'Invalid signatures: some of the signatures are not part of the multisig signers.' +
                'Wrong signature addresses:\n' +
                wrongSignatureAddresses.join('\n'),
        );
    }

    let signaturesThreshold = 0;

    for (const signatureAddress of signatureAddresses) {
        const weight = weights[signers.indexOf(signatureAddress)];
        signaturesThreshold += weight;
    }

    if (signaturesThreshold < threshold) {
        throw new Error(
            `Invalid signatures: the sum of the weights of the signatures (${signaturesThreshold})` +
                ` is less than the threshold (${threshold})`,
        );
    }

    if (await multisigContract.isBatchExecuted(batchId)) {
        throw new Error(`The batch with id ${batchId} has already been executed`);
    }

    const proof = sortWeightedSignaturesProof(callsBatchData, signers, weights, threshold, signatures);

    try {
        await multisigContract.validateProof(messageHash, proof);
    } catch (e) {
        throw new Error(`The proof was rejected by the contract:\n${e.message}`);
    }
}

const signCallsBatch = async (batchId, calls, wallet) => {
    const callsBatchData = encodeInterchainCallsBatch(batchId, calls);
    const hash = arrayify(keccak256(callsBatchData));

    return wallet.signMessage(hash);
};

let calls = [];

async function processCommand(_, chain, options) {
    const {
        contractName,
        address,
        action,
        symbols,
        tokenIds,
        limits,
        mintLimiter,
        newMultisig,
        recipient,
        target,
        calldata,
        nativeValue,
        withdrawAmount,
        privateKey,
        yes,
        offline,
    } = options;

    const newSigners = options.newSigners.split(',');
    const newWeights = options.newWeights.split(',').map(Number);
    const newThreshold = Number(options.newThreshold);

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName];

    let multisigAddress;

    if (isValidAddress(address)) {
        multisigAddress = address;
    } else {
        if (!contractConfig?.address) {
            throw new Error(`Contract ${contractName} is not deployed on ${chain.name}`);
        }

        multisigAddress = contractConfig.address;
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = await getWallet(privateKey, null, options);

    printInfo('Contract name', contractName);
    printInfo('Contract address', multisigAddress);

    const multisigContract = new Contract(multisigAddress, IInterchainMultisig.abi, wallet);

    printInfo('InterchainMultisig Action', action);

    if (prompt(`Proceed with action ${action} on chain ${chain.name}?`, yes)) {
        return;
    }

    switch (action) {
        case 'signers': {
            const signerEpoch = await multisigContract.epoch();
            const signerHash = await multisigContract.signerHashByEpoch(signerEpoch);

            printInfo('Signer epoch', signerEpoch);
            printInfo('Signer hash', signerHash);

            return;
        }

        case 'setTokenMintLimits': {
            const symbolsArray = JSON.parse(symbols);
            const limitsArray = JSON.parse(limits);



            if (symbolsArray.length !== limitsArray.length) {
                throw new Error('Token symbols and token limits length mismatch');
            }

            const multisigTarget = chain.contracts.AxelarGateway?.address;

            validateParameters({ isValidAddress: { multisigTarget }, isNumberArray: {limitsArray}, isNonEmptyStringArray: {symbolsArray}});

            const gateway = new Contract(multisigTarget, IGateway.abi, wallet);
            const multisigCalldata = gateway.interface.encodeFunctionData('setTokenMintLimits', [symbolsArray, limitsArray]);

            printInfo('Rate limit tokens', symbolsArray);
            printInfo('Rate limit values', limitsArray);

            if (!offline) {
                // loop over each token
                for (const tokenSymbol of symbolsArray) {
                    const token = await gateway.tokenAddresses(tokenSymbol);
                    const limit = await gateway.tokenMintLimit(tokenSymbol);
                    printInfo(`Token ${tokenSymbol} address`, token);
                    printInfo(`Token ${tokenSymbol} limit`, limit);
                }
            }

            calls.push([chain.axelarId, multisigContract.address, multisigTarget, multisigCalldata, 0]);

            break;
        }

        case 'transferMintLimiter': {
            const multisigTarget = chain.contracts.AxelarGateway?.address;

            validateParameters({ isValidAddress: { mintLimiter, multisigTarget } });

            const gateway = new Contract(multisigTarget, IGateway.abi, wallet);
            const multisigCalldata = gateway.interface.encodeFunctionData('transferMintLimiter', [mintLimiter]);

            calls.push([chain.axelarId, multisigContract.address, multisigTarget, multisigCalldata, 0]);

            break;
        }

        case 'transferMultisig': {
            const multisigTarget = chain.contracts.AxelarServiceGovernance?.address;

            validateParameters({ isValidAddress: { newMultisig, multisigTarget } });

            const governance = new Contract(multisigTarget, IGovernance.abi, wallet);
            const multisigCalldata = governance.interface.encodeFunctionData('transferMultisig', [newMultisig]);

            calls.push([chain.axelarId, multisigContract.address, multisigTarget, multisigCalldata, 0]);

            break;
        }

        case 'rotateSigners': {
            validateParameters({ isAddressArray: { newSigners }, isNumberArray: {newWeights}, isNumber: {newThreshold} });


            if (newSigners.length !== newWeights.length) {
                throw new Error('New signers and new weights length mismatch');
            }

            if (newWeights.reduce((sum, weight) => sum + weight, 0) < newThreshold) {
                throw new Error('The sum of the new weights is less than the new threshold');
            }

            const multisigTarget = multisigContract.address;

            const signersWithWeights = newSigners.map((address, i) => ({ address, weight: newWeights[i] }));
            const sortedSignersWithWeights = sortBy(signersWithWeights, (signer) => signer.address.toLowerCase());
            const sortedSigners = sortedSignersWithWeights.map(({ address }) => address);
            const sortedWeights = sortedSignersWithWeights.map(({ weight }) => weight);

            const multisig = new Contract(multisigTarget, IInterchainMultisig.abi, wallet);
            const multisigCalldata = multisig.interface.encodeFunctionData('rotateSigners', [[sortedSigners, sortedWeights, newThreshold]]);

            calls.push([chain.axelarId, multisigContract.address, multisigTarget, multisigCalldata, 0]);

            break;
        }

        case 'withdraw': {
            validateParameters({ isValidAddress: { recipient }, isValidDecimal: {withdrawAmount}});

            const amount = parseEther(withdrawAmount);

            if (!offline) {
                const balance = await provider.getBalance(multisigContract.address);

                if (balance.lt(amount)) {
                    throw new Error(
                        `Contract balance ${formatEther(BigNumber.from(balance))} is less than withdraw amount: ${formatEther(
                            BigNumber.from(amount),
                        )}`,
                    );
                }
            }

            const multisigCalldata = multisigContract.interface.encodeFunctionData('withdraw', [recipient, amount]);

            calls.push([chain.axelarId, multisigContract.address, multisigContract.address, multisigCalldata, 0]);

            break;
        }

        case 'executeCalls': {
            if (calldata === '0x') {
                printWarn(`Calldata for execute multisig calls is empty.`);

                if (prompt(`Proceed with ${action}?`, yes)) {
                    return;
                }
            }

            const governance = chain.contracts.AxelarServiceGovernance?.address;

            validateParameters({ isValidAddress: { target, governance }, isValidCalldata: {calldata}, isValidDecimal: {nativeValue}});

            if (!offline) {
                const balance = await provider.getBalance(governance);

                if (balance.lt(nativeValue)) {
                    throw new Error(
                        `AxelarServiceGovernance balance ${formatEther(
                            BigNumber.from(balance),
                        )} is less than native value amount: ${formatEther(BigNumber.from(nativeValue))}`,
                    );
                }
            }

            calls.push([chain.axelarId, multisigContract.address, target, calldata, nativeValue]);

            break;
        }

        case 'setFlowLimits': {
            const tokenIdsArray = JSON.parse(tokenIds);
            const limitsArray = JSON.parse(limits);

            if (tokenIdsArray.length !== limitsArray.length) {
                throw new Error('Token ids and token flow limits length mismatch');
            }

            const multisigTarget = chain.contracts.InterchainTokenService?.address;

            validateParameters({ isValidAddress:{multisigTarget }, isBytes32Array: { tokenIdsArray }, isNumberArray: {limitsArray}});

            const its = new Contract(multisigTarget, IInterchainTokenService.abi, wallet);
            const multisigCalldata = its.interface.encodeFunctionData('setFlowLimits', [tokenIdsArray, limitsArray]);

            printInfo('Token Ids', tokenIdsArray);
            printInfo('FLow limit values', limitsArray);

            if (!offline) {
                const operatable = new Contract(multisigTarget, IOperator.abi, wallet);
                const hasOperatorRole = await operatable.isOperator(multisigAddress);

                if (!hasOperatorRole) {
                    throw new Error('Missing Operator role for the used multisig address.');
                }

                // loop over each token
                for (const tokenId of tokenIdsArray) {
                    const tokenManagerAddress = await its.validTokenManagerAddress(tokenId);
                    const tokenManager = new Contract(tokenManagerAddress, ITokenManager.abi, wallet);
                    const currentFlowLimit = await tokenManager.flowLimit();
                    printInfo(`TokenManager address`, tokenManagerAddress);
                    printInfo(`TokenManager current flowLimit`, currentFlowLimit);
                }
            }

            calls.push([chain.axelarId, multisigContract.address, multisigTarget, multisigCalldata, 0]);

            break;
        }

        case 'voidBatch': {
            const multisigCalldata = multisigContract.interface.encodeFunctionData('voidBatch', []);

            calls.push([chain.axelarId, multisigContract.address, multisigContract.address, multisigCalldata, 0]);

            break;
        }
    }
}

async function submitTransactions(config, chain, options) {
    const { address, contractName, action, batchId, privateKey, yes } = options;
    const signatures = options.signatures.split(',');

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName];

    let multisigAddress;

    if (isValidAddress(address)) {
        multisigAddress = address;
    } else {
        if (!contractConfig?.address) {
            throw new Error(`Contract ${contractName} is not deployed on ${chain.name}`);
        }

        multisigAddress = contractConfig.address;
    }

    const { signers, weights, threshold } = contractConfig;

    if (prompt(`Proceed with submitting the calls batch with id ${batchId} on ${chain.name}?`, yes)) {
        return;
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);
    const wallet = await getWallet(privateKey, provider, options);
    await printWalletInfo(wallet, options);

    const multisigContract = new Contract(multisigAddress, IInterchainMultisig.abi, wallet);
    const gasOptions = await getGasOptions(chain, options, contractName);

    await preExecutionChecks(multisigContract, action, wallet, batchId, calls, signers, weights, threshold, signatures);

    const proof = sortWeightedSignaturesProof(encodeInterchainCallsBatch(batchId, calls), signers, weights, threshold, signatures);

    await multisigContract.executeCalls(formatBytes32String(batchId), calls, proof, gasOptions);

    if (action === 'rotateSigners') {
        const newSigners = options.newSigners.split(',');
        const newWeights = options.newWeights.split(',').map(Number);
        const newThreshold = Number(options.newThreshold);

        contractConfig.signers = newSigners;
        contractConfig.weights = newWeights;
        contractConfig.threshold = newThreshold;

        saveConfig(config, options.env);
    }

    printInfo(`Batch with id ${batchId} successfully executed.`);
}

async function main(options) {
    calls = [];

    await mainProcessor(options, processCommand, false);

    printInfo(`Interchain calls`, JSON.stringify(calls, null, 2));

    if (options.offline) {
        const { batchId, privateKey, yes } = options;

        const wallet = await getWallet(privateKey, null, options);
        const signature = await signCallsBatch(batchId, calls, wallet);

        if (prompt(`Proceed with signing the calls batch with id ${batchId}?`, yes)) {
            return;
        }

        printInfo(`Wallet address`, wallet.address);
        printInfo(`Signature`, signature);
    } else {
        await mainProcessor(options, submitTransactions, false);
    }
}

if (require.main === module) {
    const program = new Command();

    program.name('interchain-multisig').description('Script to manage interchain multisig actions');

    addBaseOptions(program, { address: true });

    program.addOption(new Option('-c, --contractName <contractName>', 'contract name').default('InterchainMultisig'));
    program.addOption(
        new Option('--action <action>', 'interchain multisig action')
            .choices([
                'signers',
                'setTokenMintLimits',
                'transferMintLimiter',
                'transferMultisig',
                'rotateSigners',
                'withdraw',
                'executeCalls',
                'setFlowLimits',
                'voidBatch',
            ])
            .makeOptionMandatory(true),
    );
    program.addOption(new Option('--offline', 'run script in offline mode'));
    program.addOption(new Option('--batchId <batchId>', 'The id of the batch to be executed').makeOptionMandatory(true));
    program.addOption(new Option('--signatures <signatures>', 'Signatures to ').env('SIGNATURES'));

    // options for setTokenMintLimits
    program.addOption(new Option('--symbols <symbols>', 'token symbols'));
    program.addOption(new Option('--limits <limits>', 'token limits'));

    // option for transferMintLimiter
    program.addOption(new Option('--mintLimiter <mintLimiter>', 'new mint limiter address'));

    // option for transferMultisig
    program.addOption(new Option('--newMultisig <newMultisig>', 'new mint multisig address'));

    // options for rotateSigners
    program.addOption(new Option('--newSigners <newSigners>', 'new signers'));
    program.addOption(new Option('--newWeights <newWeights>', 'new weights'));
    program.addOption(new Option('--newThreshold <newThreshold>', 'new threshold'));

    // options for withdraw
    program.addOption(new Option('--recipient <recipient>', 'withdraw recipient address'));
    program.addOption(new Option('--withdrawAmount <withdrawAmount>', 'withdraw amount'));

    // options for executeCalls
    program.addOption(new Option('--target <target>', 'execute multisig proposal target'));
    program.addOption(new Option('--calldata <calldata>', 'execute multisig proposal calldata'));
    program.addOption(new Option('--nativeValue <nativeValue>', 'execute multisig proposal nativeValue').default(0));

    // option for setFlowLimit in ITS
    program.addOption(new Option('--tokenIds <tokenIds>', 'token ids'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
