'use strict';

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { arrayify, keccak256, formatEther, defaultAbiCoder },
    Contract,
    BigNumber,
} = ethers;
const { sortBy } = require('lodash');
const { Command, Option } = require('commander');
const {
    printInfo,
    printWalletInfo,
    isNumber,
    isValidCalldata,
    printWarn,
    isNonEmptyStringArray,
    isNumberArray,
    isValidAddress,
    mainProcessor,
    isValidDecimal,
    prompt,
    isBytes32Array,
    getGasOptions,
    isAddressArray,
} = require('./utils');
const { addBaseOptions } = require('./cli-utils');
const IMultisig = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IMultisig.json');
const IGateway = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IAxelarGateway.json');
const IGovernance = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IAxelarServiceGovernance.json');
const IInterchainTokenService = require('@axelar-network/interchain-token-service/interfaces/IInterchainTokenService.json');
const ITokenManager = require('@axelar-network/interchain-token-service/interfaces/ITokenManager.json');
const IOperator = require('@axelar-network/interchain-token-service/interfaces/IOperator.json');
const { parseEther } = require('ethers/lib/utils');
const { getWallet, signTransaction, storeSignedTx } = require('./sign-utils');

const getAddresses = (wallets) => wallets.map(({ address }) => address);
const encodeInterchainCallsBatch = (batchId, calls) =>
    defaultAbiCoder.encode(['uint256', 'tuple(string, address, address, bytes, uint256)[]'], [batchId, calls]);

const getWeightedSignaturesProof = async (data, accounts, weights, threshold, signers) => {
    const hash = arrayify(keccak256(data));
    const signatures = await Promise.all(
        sortBy(signers, (wallet) => wallet.address.toLowerCase()).map((wallet) => wallet.signMessage(hash)),
    );
    return defaultAbiCoder.encode(
        ['address[]', 'uint256[]', 'uint256', 'bytes[]'],
        [getAddresses(accounts), weights, threshold, signatures],
    );
};

async function preExecutionChecks(multisigContract, action, wallet, target, calldata, nativeValue, yes) {
    const address = await wallet.getAddress();

    // TODO hash the calls batch and validate signatures
}

async function processCommand(_, chain, options) {
    const {
        env,
        contractName,
        address,
        action,
        batchId,
        symbols,
        tokenIds,
        limits,
        mintLimiter,
        newMultisig,
        signers,
        weights,
        threshold,
        recipient,
        target,
        calldata,
        nativeValue,
        withdrawAmount,
        privateKey,
        yes,
        offline,
    } = options;

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

    printInfo('Chain', chain.name);

    const wallet = await getWallet(privateKey, provider, options);
    const { address: walletAddress } = await printWalletInfo(wallet, options);

    printInfo('Contract name', contractName);
    printInfo('Contract address', multisigAddress);

    const multisigContract = new Contract(multisigAddress, IMultisig.abi, wallet);

    const gasOptions = await getGasOptions(chain, options, contractName);

    printInfo('InterchainMultisig Action', action);

    if (prompt(`Proceed with action ${action} on chain ${chain.name}?`, yes)) {
        return;
    }

    let tx;

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

            if (!isNonEmptyStringArray(symbolsArray)) {
                throw new Error(`Invalid token symbols: ${symbols})}`);
            }

            if (!isNumberArray(limitsArray)) {
                throw new Error(`Invalid token limits: ${limits}`);
            }

            if (symbolsArray.length !== limitsArray.length) {
                throw new Error('Token symbols and token limits length mismatch');
            }

            const multisigTarget = chain.contracts.AxelarGateway?.address;

            if (!isValidAddress(multisigTarget)) {
                throw new Error(`Missing AxelarGateway address in the chain info.`);
            }

            const gateway = new Contract(multisigTarget, IGateway.abi, wallet);
            const multisigCalldata = gateway.interface.encodeFunctionData('setTokenMintLimits', [symbolsArray, limitsArray]);

            printInfo('Rate limit tokens', symbolsArray);
            printInfo('Rate limit values', limitsArray);

            if (!offline) {
                await preExecutionChecks(multisigContract, action, wallet, multisigTarget, multisigCalldata, 0, yes);

                // loop over each token
                for (let i = 0; i < symbolsArray.length; i++) {
                    const token = await gateway.tokenAddresses(symbolsArray[i]);
                    const limit = await gateway.tokenMintLimit(symbolsArray[i]);
                    printInfo(`Token ${symbolsArray[i]} address`, token);
                    printInfo(`Token ${symbolsArray[i]} limit`, limit);
                }
            }

            const calls = [[chain.axelarId, multisigContract.address, multisigTarget, multisigCalldata, 0]];
            // TODO add signing process
            const proof = getWeightedSignaturesProof(encodeInterchainCallsBatch(batchId, calls), [wallet.address], [1], 1, [wallet]);

            tx = await multisigContract.populateTransaction.executeCalls(batchId, calls, proof, gasOptions);
            break;
        }

        case 'transferMintLimiter': {
            if (!isValidAddress(mintLimiter)) {
                throw new Error(`Invalid new mint limiter address: ${mintLimiter}`);
            }

            const multisigTarget = chain.contracts.AxelarGateway?.address;

            if (!isValidAddress(multisigTarget)) {
                throw new Error(`Missing AxelarGateway address in the chain info.`);
            }

            const gateway = new Contract(multisigTarget, IGateway.abi, wallet);
            const multisigCalldata = gateway.interface.encodeFunctionData('transferMintLimiter', [mintLimiter]);

            if (!offline) {
                await preExecutionChecks(multisigContract, action, wallet, multisigTarget, multisigCalldata, 0, yes);
            }

            const calls = [[chain.axelarId, multisigContract.address, multisigTarget, multisigCalldata, 0]];
            // TODO add signing process
            const proof = getWeightedSignaturesProof(encodeInterchainCallsBatch(batchId, calls), [wallet.address], [1], 1, [wallet]);

            tx = await multisigContract.populateTransaction.executeCalls(batchId, calls, proof, gasOptions);
            break;
        }

        case 'transferMultisig': {
            if (!isValidAddress(newMultisig)) {
                throw new Error(`Invalid new mint limiter address: ${newMultisig}`);
            }

            const multisigTarget = chain.contracts.AxelarServiceGovernance?.address;

            if (!isValidAddress(multisigTarget)) {
                throw new Error(`Missing AxelarServiceGovernance address in the chain info.`);
            }

            const governance = new Contract(multisigTarget, IGovernance.abi, wallet);
            const multisigCalldata = governance.interface.encodeFunctionData('transferMultisig', [newMultisig]);

            if (!offline) {
                await preExecutionChecks(multisigContract, action, wallet, multisigTarget, multisigCalldata, 0, yes);
            }

            const calls = [[chain.axelarId, multisigContract.address, multisigTarget, multisigCalldata, 0]];
            // TODO add signing process
            const proof = getWeightedSignaturesProof(encodeInterchainCallsBatch(batchId, calls), [wallet.address], [1], 1, [wallet]);

            tx = await multisigContract.populateTransaction.executeCalls(batchId, calls, proof, gasOptions);
            break;
        }

        case 'rotateSigners': {
            if (!isAddressArray(signers)) {
                throw new Error(`Invalid new signers: ${signers}`);
            }

            if (!isNumberArray(weights)) {
                throw new Error(`Invalid new weights: ${weights}`);
            }

            if (!isNumber(threshold)) {
                throw new Error(`Invalid new threshold: ${threshold}`);
            }

            const multisigTarget = multisigContract.address;

            const multisig = new Contract(multisigTarget, IMultisig.abi, wallet);
            const multisigCalldata = multisig.interface.encodeFunctionData('rotateSigners', [[signers, weights, threshold]]);

            if (!offline) {
                await preExecutionChecks(multisigContract, action, wallet, multisigTarget, multisigCalldata, 0, yes);
            }

            const calls = [[chain.axelarId, multisigContract.address, multisigTarget, multisigCalldata, 0]];
            // TODO add signing process
            const proof = getWeightedSignaturesProof(encodeInterchainCallsBatch(batchId, calls), [wallet.address], [1], 1, [wallet]);

            tx = await multisigContract.populateTransaction.executeCalls(batchId, calls, proof, gasOptions);
            break;
        }

        case 'withdraw': {
            if (!isValidAddress(recipient)) {
                throw new Error(`Invalid recipient address: ${recipient}`);
            }

            if (!isValidDecimal(withdrawAmount)) {
                throw new Error(`Invalid withdraw amount: ${withdrawAmount}`);
            }

            const amount = parseEther(withdrawAmount);

            if (!offline) {
                await preExecutionChecks(multisigContract, action, wallet, recipient, '0x', amount, yes);

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

            const calls = [[chain.axelarId, multisigContract.address, multisigContract.address, multisigCalldata, 0]];
            // TODO add signing process
            const proof = getWeightedSignaturesProof(encodeInterchainCallsBatch(batchId, calls), [wallet.address], [1], 1, [wallet]);

            tx = await multisigContract.populateTransaction.executeCalls(batchId, calls, proof, gasOptions);
            break;
        }

        case 'executeCalls': {
            if (!isValidAddress(target)) {
                throw new Error(`Invalid target for execute multisig calls: ${target}`);
            }

            if (!isValidCalldata(calldata)) {
                throw new Error(`Invalid calldata for execute multisig calls: ${calldata}`);
            }

            if (calldata === '0x') {
                printWarn(`Calldata for execute multisig calls is empty.`);

                if (prompt(`Proceed with ${action}?`, yes)) {
                    return;
                }
            }

            if (!isNumber(parseFloat(nativeValue))) {
                throw new Error(`Invalid native value for execute multisig proposal: ${nativeValue}`);
            }

            const governance = chain.contracts.AxelarServiceGovernance?.address;

            if (!isValidAddress(governance)) {
                throw new Error(`Missing AxelarServiceGovernance address in the chain info.`);
            }

            const governanceContract = new Contract(governance, IGovernance.abi, wallet);

            if (!offline) {
                await preExecutionChecks(governanceContract, action, wallet, target, calldata, nativeValue, yes);

                const balance = await provider.getBalance(governance);

                if (balance.lt(nativeValue)) {
                    throw new Error(
                        `AxelarServiceGovernance balance ${formatEther(
                            BigNumber.from(balance),
                        )} is less than native value amount: ${formatEther(BigNumber.from(nativeValue))}`,
                    );
                }
            }

            const calls = [[chain.axelarId, multisigContract.address, target, calldata, nativeValue]];
            // TODO add signing process
            const proof = getWeightedSignaturesProof(encodeInterchainCallsBatch(batchId, calls), [wallet.address], [1], 1, [wallet]);

            tx = await multisigContract.populateTransaction.executeCalls(batchId, calls, proof, gasOptions);
            break;
        }

        case 'setFlowLimits': {
            const tokenIdsArray = JSON.parse(tokenIds);
            const limitsArray = JSON.parse(limits);

            if (!isBytes32Array(tokenIdsArray)) {
                throw new Error(`Invalid token symbols: ${tokenIds}`);
            }

            if (!isNumberArray(limitsArray)) {
                throw new Error(`Invalid token limits: ${limits}`);
            }

            if (tokenIdsArray.length !== limitsArray.length) {
                throw new Error('Token ids and token flow limits length mismatch');
            }

            const multisigTarget = chain.contracts.InterchainTokenService?.address;

            if (!isValidAddress(multisigTarget)) {
                throw new Error(`Missing InterchainTokenService address in the chain info.`);
            }

            const its = new Contract(multisigTarget, IInterchainTokenService.abi, wallet);
            const multisigCalldata = its.interface.encodeFunctionData('setFlowLimits', [tokenIdsArray, limitsArray]);

            printInfo('Token Ids', tokenIdsArray);
            printInfo('FLow limit values', limitsArray);

            if (!offline) {
                await preExecutionChecks(multisigContract, action, wallet, multisigTarget, multisigCalldata, 0, yes);
                const operatable = new Contract(multisigTarget, IOperator.abi, wallet);
                const hasOperatorRole = await operatable.isOperator(multisigAddress);

                if (!hasOperatorRole) {
                    throw new Error('Missing Operator role for the used multisig address.');
                }

                // loop over each token
                for (let i = 0; i < tokenIdsArray.length; ++i) {
                    const tokenManagerAddress = await its.validTokenManagerAddress(tokenIdsArray[i]);
                    const tokenManager = new Contract(tokenManagerAddress, ITokenManager.abi, wallet);
                    const currentFlowLimit = await tokenManager.flowLimit();
                    printInfo(`TokenManager address`, tokenManagerAddress);
                    printInfo(`TokenManager current flowLimit`, currentFlowLimit);
                }
            }

            const calls = [[chain.axelarId, multisigContract.address, multisigTarget, multisigCalldata, 0]];
            // TODO add signing process
            const proof = getWeightedSignaturesProof(encodeInterchainCallsBatch(batchId, calls), [wallet.address], [1], 1, [wallet]);

            tx = await multisigContract.populateTransaction.executeCalls(batchId, calls, proof, gasOptions);
            break;
        }

        case 'voidBatch': {
            if (!offline) {
                await preExecutionChecks(multisigContract, action, wallet, recipient, '0x', 0, yes);
            }

            const multisigCalldata = multisigContract.interface.encodeFunctionData('voidBatch', []);

            const calls = [[chain.axelarId, multisigContract.address, multisigContract.address, multisigCalldata, 0]];
            // TODO add signing process
            const proof = getWeightedSignaturesProof(encodeInterchainCallsBatch(batchId, calls), [wallet.address], [1], 1, [wallet]);

            tx = await multisigContract.populateTransaction.executeCalls(batchId, calls, proof, gasOptions);
            break;
        }
    }

    const { baseTx, signedTx } = await signTransaction(wallet, chain, tx, options);

    if (offline) {
        const filePath = `./tx/signed-tx-${env}-multisig-${action}-${chain.name.toLowerCase()}-address-${walletAddress}-nonce-${
            baseTx.nonce
        }.json`;
        printInfo(`Storing signed Tx offline in file ${filePath}`);

        // Storing the fields in the data that will be stored in file
        const data = {
            msg: `This transaction will perform multisig action ${action} on chain ${chain.name}`,
            unsignedTx: baseTx,
            signedTx,
            status: 'PENDING',
        };

        storeSignedTx(filePath, data);
    }
}

async function main(options) {
    await mainProcessor(options, processCommand, false);
}

if (require.main === module) {
    const program = new Command();

    program.name('multisig').description('Script to manage multisig actions');

    addBaseOptions(program, { address: true });

    program.addOption(
        new Option('-c, --contractName <contractName>', 'contract name').default('InterchainMultisig').makeOptionMandatory(false),
    );
    program.addOption(
        new Option('--action <action>', 'multisig action')
            .choices([
                'signers',
                'setTokenMintLimits',
                'transferMintLimiter',
                'transferMultisig',
                'withdraw',
                'executeCalls',
                'setFlowLimits',
                'voidBatch',
            ])
            .makeOptionMandatory(true),
    );
    program.addOption(new Option('--offline', 'run script in offline mode'));
    program.addOption(new Option('--batchId <batchId>', 'The id of the batch to be executed').makeOptionMandatory(true));

    // options for setTokenMintLimits
    program.addOption(new Option('--symbols <symbols>', 'token symbols').makeOptionMandatory(false));
    program.addOption(new Option('--limits <limits>', 'token limits').makeOptionMandatory(false));

    // option for transferMintLimiter
    program.addOption(new Option('--mintLimiter <mintLimiter>', 'new mint limiter address').makeOptionMandatory(false));

    // option for transferMultisig
    program.addOption(new Option('--newMultisig <newMultisig>', 'new mint multisig address').makeOptionMandatory(false));

    // options for rotateSigners
    program.addOption(new Option('--signers <signers>', 'new signers').makeOptionMandatory(false));
    program.addOption(new Option('--weights <weights>', 'new weights').makeOptionMandatory(false));
    program.addOption(new Option('--threshold <threshold>', 'new threshold').makeOptionMandatory(false));

    // options for withdraw
    program.addOption(new Option('--recipient <recipient>', 'withdraw recipient address').makeOptionMandatory(false));
    program.addOption(new Option('--withdrawAmount <withdrawAmount>', 'withdraw amount').makeOptionMandatory(false));

    // options for executeCalls
    program.addOption(new Option('--target <target>', 'execute multisig proposal target').makeOptionMandatory(false));
    program.addOption(new Option('--calldata <calldata>', 'execute multisig proposal calldata').makeOptionMandatory(false));
    program.addOption(
        new Option('--nativeValue <nativeValue>', 'execute multisig proposal nativeValue').makeOptionMandatory(false).default(0),
    );

    // option for setFlowLimit in ITS
    program.addOption(new Option('--tokenIds <tokenIds>', 'token ids'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
