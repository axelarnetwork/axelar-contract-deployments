'use strict';

require('dotenv').config();

const {
    Wallet,
    getDefaultProvider,
    utils: { isAddress, keccak256, Interface, formatEther },
    constants: { AddressZero },
    Contract,
    BigNumber,
} = require('ethers');
const readlineSync = require('readline-sync');
const { Command, Option } = require('commander');
const chalk = require('chalk');
const {
    printInfo,
    printWalletInfo,
    loadConfig,
    isNumber,
    isValidCalldata,
    printWarn,
    printError,
    isStringArray,
    isNumberArray,
} = require('./utils');
const IMultisig = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IMultisig.json');
const IGateway = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IAxelarGateway.json');
const IGovernance = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IAxelarServiceGovernance.json');
const { parseEther } = require('ethers/lib/utils');

async function preExecutionChecks(multisigContract, multisigAction, wallet, target, calldata, nativeValue) {
    const isSigner = await multisigContract.isSigner(wallet.address);

    if (!isSigner) {
        throw new Error(`Caller ${wallet.address} is not an authorized multisig signer.`);
    }

    let topic;

    if (multisigAction === 'withdraw') {
        topic = multisigContract.interface.encodeFunctionData('withdraw', [target, nativeValue]);
    } else if (multisigAction === 'executeMultisigProposal') {
        topic = multisigContract.interface.encodeFunctionData('executeMultisigProposal', [target, calldata, nativeValue]);
    } else {
        topic = multisigContract.interface.encodeFunctionData('executeContract', [target, calldata, nativeValue]);
    }

    const topicHash = keccak256(topic);
    const voteCount = await multisigContract.getSignerVotesCount(topicHash);

    if (voteCount.eq(0)) {
        printWarn(`The vote count for this topic is zero. This action will create a new multisig proposal.`);
        const answer = readlineSync.question(`Proceed with ${multisigAction}?`);
        if (answer !== 'y') return;
    }

    const hasVoted = await multisigContract.hasSignerVoted(wallet.address, topicHash);

    if (hasVoted) {
        throw new Error(`Signer ${wallet.address} has already voted on this proposal.`);
    }

    const threshold = await multisigContract.signerThreshold();

    if (voteCount.eq(threshold.sub(1))) {
        printWarn(`The vote count is one below the threshold. This action will execute the multisig proposal.`);
        const answer = readlineSync.question(`Proceed with ${multisigAction}?`);
        if (answer !== 'y') return 0;
    }
}

async function processCommand(options, chain) {
    const {
        contractName,
        address,
        multisigAction,
        symbols,
        limits,
        mintLimiterAddress,
        recipientAddress,
        target,
        calldata,
        nativeValue,
        privateKey,
        yes,
    } = options;

    let withdrawAmount = options.withdrawAmount;
    const contracts = chain.contracts;
    const contractConfig = contracts[contractName];

    let multisigAddress;

    if (isAddress(address) && address !== AddressZero) {
        multisigAddress = address;
    } else {
        if (!contractConfig?.address) {
            throw new Error(`Contract ${contractName} is not deployed on ${chain.name}`);
        }

        multisigAddress = contractConfig.address;
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = new Wallet(privateKey, provider);
    await printWalletInfo(wallet);

    printInfo('Contract name', contractName);

    const multisigContract = new Contract(multisigAddress, IMultisig.abi, wallet);

    const gasOptions = contractConfig?.gasOptions || chain?.gasOptions || {};
    console.log(`Gas override for chain ${chain.name}: ${JSON.stringify(gasOptions)}`);

    printInfo('Multisig Action', multisigAction);

    switch (multisigAction) {
        case 'setTokenMintLimits': {
            const symbolsArray = JSON.parse(symbols);
            const limitsArray = JSON.parse(limits);

            if (!isStringArray(symbolsArray)) {
                throw new Error(`Invalid token symbols: ${symbols})}`);
            }

            if (!isNumberArray(limitsArray)) {
                throw new Error(`Invalid token limits: ${limits}`);
            }

            if (symbolsArray.length !== limitsArray.length) {
                throw new Error('Token symbols and token limits length mismatch');
            }

            const multisigTarget = chain.contracts.AxelarGateway?.address;

            if (!isAddress(multisigTarget) && multisigTarget !== AddressZero) {
                throw new Error(`Missing AxelarGateway address in the chain info.`);
            }

            const gatewayContract = new Contract(multisigTarget, IGateway.abi, wallet);
            const targetInterface = new Interface(gatewayContract.interface.fragments);
            const multisigCalldata = targetInterface.encodeFunctionData('setTokenMintLimits', [symbolsArray, limitsArray]);

            if (!yes) {
                const answer = readlineSync.question(`Proceed with multiSigAction without pre-execution checks ${chalk.green('(y/n)')} `);

                if (answer !== 'y') {
                    await preExecutionChecks(multisigContract, multisigAction, wallet, multisigTarget, multisigCalldata, 0);
                }
            }

            const tx = await multisigContract.executeContract(multisigTarget, multisigCalldata, 0, gasOptions);
            await tx.wait();
            break;
        }

        case 'transferMintLimiter': {
            if (!isAddress(mintLimiterAddress) && mintLimiterAddress !== AddressZero) {
                throw new Error(`Invalid new mint limiter address: ${mintLimiterAddress}`);
            }

            const multisigTarget = chain.contracts.AxelarGateway?.address;

            if (!isAddress(multisigTarget) && multisigTarget !== AddressZero) {
                throw new Error(`Missing AxelarGateway address in the chain info.`);
            }

            const gatewayContract = new Contract(multisigTarget, IGateway.abi, wallet);
            const targetInterface = new Interface(gatewayContract.interface.fragments);
            const multisigCalldata = targetInterface.encodeFunctionData('transferMintLimiter', [mintLimiterAddress]);

            if (!yes) {
                const answer = readlineSync.question(`Proceed with multiSigAction without pre-execution checks ${chalk.green('(y/n)')} `);

                if (answer !== 'y') {
                    await preExecutionChecks(multisigContract, multisigAction, wallet, multisigTarget, multisigCalldata, 0);
                }
            }

            const tx = await multisigContract.executeContract(multisigTarget, multisigCalldata, 0);
            await tx.wait();
            break;
        }

        case 'withdraw': {
            if (!isAddress(recipientAddress) && recipientAddress !== AddressZero) {
                throw new Error(`Invalid recipient address: ${recipientAddress}`);
            }

            if (!isNumber(parseFloat(withdrawAmount)) || parseFloat(withdrawAmount) === 0) {
                throw new Error(`Invalid withdraw amount: ${withdrawAmount}`);
            }

            withdrawAmount = parseEther(withdrawAmount);

            if (!yes) {
                const answer = readlineSync.question(`Proceed with multiSigAction without pre-execution checks ${chalk.green('(y/n)')} `);

                if (answer !== 'y') {
                    await preExecutionChecks(multisigContract, multisigAction, wallet, recipientAddress, '0x', withdrawAmount);
                }
            }

            const balance = await provider.getBalance(multisigContract.address);

            if (balance.lt(withdrawAmount)) {
                throw new Error(
                    `Contract balance ${formatEther(BigNumber.from(balance))} is less than withdraw amount: ${formatEther(
                        BigNumber.from(withdrawAmount),
                    )}`,
                );
            }

            const tx = await multisigContract.withdraw(recipientAddress, withdrawAmount);
            await tx.wait();
            break;
        }

        case 'executeMultisigProposal': {
            if (!isAddress(target) && target !== AddressZero) {
                throw new Error(`Invalid target for execute multisig proposal: ${target}`);
            }

            if (!isValidCalldata(calldata)) {
                throw new Error(`Invalid calldata for execute multisig proposal: ${calldata}`);
            }

            if (calldata === '0x' && !yes) {
                printWarn(`Calldata for execute multisig proposal is empty.`);
                const answer = readlineSync.question(`Proceed with ${multisigAction}?`);
                if (answer !== 'y') return;
            }

            if (!isNumber(parseFloat(nativeValue))) {
                throw new Error(`Invalid native value for execute multisig proposal: ${nativeValue}`);
            }

            const governance = chain.contracts.AxelarServiceGovernance?.address;

            if (!isAddress(governance) && governance !== AddressZero) {
                throw new Error(`Missing AxelarServiceGovernance address in the chain info.`);
            }

            const governanceContract = new Contract(governance, IGovernance.abi, wallet);

            if (!yes) {
                const answer = readlineSync.question(`Proceed with multiSigAction without pre-execution checks ${chalk.green('(y/n)')} `);

                if (answer !== 'y') {
                    await preExecutionChecks(governanceContract, multisigAction, wallet, target, calldata, nativeValue);
                }
            }

            const balance = await provider.getBalance(governance);

            if (balance.lt(nativeValue)) {
                throw new Error(
                    `AxelarServiceGovernance balance ${formatEther(
                        BigNumber.from(balance),
                    )} is less than native value amount: ${formatEther(BigNumber.from(nativeValue))}`,
                );
            }

            const tx = await governanceContract.executeMultisigProposal(target, calldata, nativeValue);
            await tx.wait();
            break;
        }
    }
}

async function main(options) {
    const config = loadConfig(options.env);

    let chains = options.destinationChains.split(',').map((str) => str.trim());

    if (options.destinationChains === 'all') {
        chains = Object.keys(config.chains);
    }

    for (const chain of chains) {
        if (config.chains[chain.toLowerCase()] === undefined) {
            throw new Error(`Destination chain ${chain} is not defined in the info file`);
        }
    }

    for (const chain of chains) {
        try {
            await processCommand(options, config.chains[chain.toLowerCase()]);
        } catch (error) {
            printError(error);
        }
    }
}

const program = new Command();

program.name('multisig-script').description('Script to manage multisig actions');

program.addOption(
    new Option('-e, --env <env>', 'environment')
        .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
        .default('testnet')
        .makeOptionMandatory(true)
        .env('ENV'),
);
program.addOption(new Option('-c, --contractName <contractName>', 'contract name').default('Multisig').makeOptionMandatory(false));
program.addOption(new Option('-a, --address <address>', 'override address').makeOptionMandatory(false));
program.addOption(new Option('-n, --destinationChains <destinationChains>', 'destination chain').makeOptionMandatory(true));
program.addOption(
    new Option('-g, --multisigAction <multisigAction>', 'multisig action')
        .choices(['setTokenMintLimits', 'transferMintLimiter', 'withdraw', 'executeMultisigProposal'])
        .default('setTokenMintLimits'),
);

// options for setTokenMintLimits
program.addOption(new Option('-s, --symbols <symbols>', 'token symbols').makeOptionMandatory(false));
program.addOption(new Option('-l, --limits <limits>', 'token limits').makeOptionMandatory(false));

// option for transferMintLimiter
program.addOption(new Option('-m, --mintLimiter <mintLimiter>', 'new mint limiter address').makeOptionMandatory(false));

// options for withdraw
program.addOption(new Option('-r, --recipient <recipient>', 'withdraw recipient address').makeOptionMandatory(false));
program.addOption(new Option('-w, --withdrawAmount <withdrawAmount>', 'withdraw amount').makeOptionMandatory(false));

// options for executeMultisigProposal
program.addOption(new Option('-t, --target <target>', 'execute multisig proposal target').makeOptionMandatory(false));
program.addOption(new Option('-d, --calldata <calldata>', 'execute multisig proposal calldata').makeOptionMandatory(false));
program.addOption(
    new Option('-v, --nativeValue <nativeValue>', 'execute multisig proposal nativeValue').makeOptionMandatory(false).default(0),
);

program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));

program.action((options) => {
    main(options);
});

program.parse();
