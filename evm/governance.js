'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const {
    Wallet,
    getDefaultProvider,
    utils: { isAddress, defaultAbiCoder, keccak256 },
    ContractFactory,
} = ethers;
const readlineSync = require('readline-sync');
const { Command, Option } = require('commander');

const {
    printInfo,
    printWalletInfo,
    loadConfig,
    saveConfig,
    isNumber,
    isValidTimeFormat,
    etaToUnixTimestamp,
    getCurrentTimeInSeconds,
    wasEventEmitted,
} = require('./utils');

async function processCommand(options, chain, config) {
    const { artifactPath, contractName, governanceAction, calldata, nativeValue, eta, privateKey, yes } = options;

    if (contractName !== 'AxelarServiceGovernance' && contractName !== 'InterchainGovernance') {
        throw new Error(`Invalid governance contract: ${contractName}`);
    }

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName];

    if (contractConfig && !contractConfig.address) {
        throw new Error(`Contract ${contractName} is not deployed on ${chain}`);
    }

    const target = chain.contracts.AxelarGateway?.address;

    if (!isAddress(target)) {
        throw new Error(`Missing AxelarGateway address in the chain info.`);
    }

    if (!isNumber(nativeValue)) {
        throw new Error(`Invalid native value: ${nativeValue}`);
    }

    if (!isValidTimeFormat(eta)) {
        throw new Error(`Invalid ETA: ${eta}. Please pass the eta in the format YYYY-MM-DDTHH:mm:ss`);
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = new Wallet(privateKey, provider);
    await printWalletInfo(wallet);

    printInfo('Contract name', contractName);

    const contractPath = artifactPath + contractName + '.sol/' + contractName + '.json';
    printInfo('Contract path', contractPath);

    const contractJson = require(contractPath);
    const governanceFactory = new ContractFactory(contractJson.abi, contractJson.bytecode, wallet);
    const governanceContract = governanceFactory.attach(contractConfig.address);

    const gasOptions = contractConfig.gasOptions || chain.gasOptions || {};
    console.log(`Gas override for chain ${chain.name}: ${JSON.stringify(gasOptions)}`);

    printInfo('Proposal Action', governanceAction);

    const unixEta = etaToUnixTimestamp(eta);

    const types = ['uint256', 'address', 'bytes', 'uint256', 'uint256'];
    const values = [0, target, calldata, nativeValue, unixEta];

    let gmpPayload;

    switch (governanceAction) {
        case 'scheduleTimeLock': {
            if (unixEta < getCurrentTimeInSeconds() + contractConfig.minimumTimeDelay && !yes) {
                console.log(`${eta} is less than the minimum time delay.`);
                const anwser = readlineSync.question(`Proceed with ${governanceAction}?`);
                if (anwser !== 'y') return;
            }

            gmpPayload = defaultAbiCoder.encode(types, values);

            console.log(
                `Destination chain: ${chain.name}\nDestination governance address: ${contractConfig.address}\nGMP payload: ${gmpPayload}`,
            );

            break;
        }

        case 'cancelTimeLock': {
            const commandType = 1;

            if (unixEta < getCurrentTimeInSeconds() + contractConfig.minimumTimeDelay && !yes) {
                console.log(`${eta} is less than the minimum time delay.`);
                const anwser = readlineSync.question(`Proceed with ${governanceAction}?`);
                if (anwser !== 'y') return;
            }

            values[0] = commandType;
            gmpPayload = defaultAbiCoder.encode(types, values);

            console.log(
                `Destination chain: ${chain.name}\nDestination governance address: ${contractConfig.address}\nGMP payload: ${gmpPayload}`,
            );

            break;
        }

        case 'approveMultisig': {
            if (contractName === 'InterchainGovernance') {
                throw new Error(`Invalid governance action for InterchainGovernance: ${governanceAction}`);
            }

            const commandType = 2;

            values[0] = commandType;
            gmpPayload = defaultAbiCoder.encode(types, values);

            console.log(
                `Destination chain: ${chain.name}\nDestination governance address: ${contractConfig.address}\nGMP payload: ${gmpPayload}`,
            );

            break;
        }

        case 'cancelMultisig': {
            if (contractName === 'InterchainGovernance') {
                throw new Error(`Invalid governance action for InterchainGovernance: ${governanceAction}`);
            }

            const commandType = 3;

            values[0] = commandType;
            gmpPayload = defaultAbiCoder.encode(types, values);

            console.log(
                `Destination chain: ${chain.name}\nDestination governance address: ${contractConfig.address}\nGMP payload: ${gmpPayload}`,
            );

            break;
        }

        case 'executeProposal': {
            const proposalHash = keccak256(defaultAbiCoder.encode(['address', 'bytes', 'uint256'], [target, calldata, nativeValue]));
            const minimumEta = await governanceContract.getTimeLock(proposalHash);

            if (minimumEta === 0) {
                throw new Error('Proposal does not exist.');
            }

            if (getCurrentTimeInSeconds() < minimumEta) {
                throw new Error(`TimeLock proposal is not yet eligible for execution.`);
            }

            const tx = await governanceContract.executeProposal(target, calldata, nativeValue, gasOptions);
            const receipt = tx.wait();

            const eventEmitted = wasEventEmitted(receipt, governanceContract, 'ProposalExecuted');

            if (eventEmitted) {
                console.log('Proposal executed');
            } else {
                console.log('Proposal execution failed');
            }

            break;
        }

        case 'executeMultisigProposal': {
            if (contractName === 'InterchainGovernance') {
                throw new Error(`Invalid governance action for InterchainGovernance: ${governanceAction}`);
            }

            const proposalHash = keccak256(defaultAbiCoder.encode(['address', 'bytes', 'uint256'], [target, calldata, nativeValue]));
            const isApproved = await governanceContract.multisigApprovals(proposalHash);

            if (!isApproved) {
                throw new Error('Multisig proposal has not been approved.');
            }

            const isSigner = await governanceContract.isSigner(wallet.address);

            if (!isSigner) {
                throw new Error(`Caller is not a valid signer address: ${wallet.address}`);
            }

            const executeInterface = new ethers.utils.Interface(governanceContract.interface.fragments);
            const executeCalldata = executeInterface.encodeFunctionData('executeMultisigProposal', [target, calldata, nativeValue]);
            const topic = keccak256(executeCalldata);

            const hasSignerVoted = await governanceContract.hasSignerVoted(wallet.address, topic);

            if (hasSignerVoted) {
                throw new Error(`Signer has already voted: ${wallet.address}`);
            }

            const signerVoteCount = await governanceContract.getSignerVotesCount(topic);
            console.log(`${signerVoteCount} signers have already voted.`);

            const tx = await governanceContract.executeMultisigProposal(target, calldata, nativeValue, gasOptions);
            const receipt = await tx.wait();

            const eventEmitted = wasEventEmitted(receipt, governanceContract, 'MultisigExecuted');

            if (eventEmitted) {
                console.log('Multisig proposal executed');
            } else {
                console.log('Multisig proposal execution failed');
            }

            break;
        }

        default: {
            throw new Error(`Unknown governance action ${governanceAction}`);
        }
    }
}

async function main(options) {
    const config = loadConfig(options.env);

    const chain = options.destinationChain;

    if (config.chains[chain.toLowerCase()] === undefined) {
        throw new Error(`Destination chain ${chain} is not defined in the info file`);
    }

    await processCommand(options, config.chains[chain.toLowerCase()], config);
    saveConfig(config, options.env);
}

const program = new Command();

program.name('deploy-contract').description('Deploy contracts using create, create2, or create3');

program.addOption(
    new Option('-e, --env <env>', 'environment')
        .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
        .default('testnet')
        .makeOptionMandatory(true)
        .env('ENV'),
);
program.addOption(new Option('-a, --artifactPath <artifactPath>', 'artifact path').makeOptionMandatory(true));
program.addOption(new Option('-c, --contractName <contractName>', 'contract name').makeOptionMandatory(true));
program.addOption(new Option('-n, --destinationChain <destinationChain>', 'destination chain').makeOptionMandatory(true));
program.addOption(
    new Option('-g, --governanceAction <governanceAction>', 'governance action')
        .choices(['scheduleTimeLock', 'cancelTimeLock', 'approveMultisig', 'cancelMultisig', 'executeProposal', 'executeMultisigProposal'])
        .default('scheduleTimeLock'),
);
program.addOption(new Option('-d, --calldata <calldata>', 'calldata').makeOptionMandatory(true));
program.addOption(new Option('-v, --nativeValue <nativeValue>', 'nativeValue').makeOptionMandatory(false).default(0));
program.addOption(new Option('-t, --eta <eta>', 'calldata').makeOptionMandatory(true));
program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));

program.action((options) => {
    main(options);
});

program.parse();
