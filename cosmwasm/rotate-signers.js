'use strict';

require('dotenv').config();

const { loadConfig, getCurrentVerifierSet, printInfo } = require('../common');
const { prepareWallet, prepareClient } = require('./utils');

const { Command } = require('commander');
const { addAmplifierOptions } = require('./cli-utils');
const { GasPrice, calculateFee } = require('@cosmjs/stargate');
const { CosmWasmClient } = require('@cosmjs/cosmwasm-stargate');

const executeTransaction = async (client, account, contractAddress, message, fee) => {
    const tx = await client.execute(account.address, contractAddress, message, fee, '');
    return tx.transactionHash;
};

const getNextVerifierSet = async (config, chain) => {
    const client = await CosmWasmClient.connect(config.axelar.rpc);
    return await client.queryContractSmart(config.axelar.contracts.MultisigProver[chain].address, 'next_verifier_set');
};

const getVerifierSetStatus = async (config, chain, verifierStatus) => {
    const client = await CosmWasmClient.connect(config.axelar.rpc);
    return await client.queryContractSmart(config.axelar.contracts.VotingVerifier[chain].address, { verifier_set_status: verifierStatus });
};

const processVerifierRotation = async (config, options, chain) => {
    const wallet = await prepareWallet(options);
    const client = await prepareClient(config, wallet);
    const [account] = await wallet.getAccounts();
    const {
        axelar: { gasPrice, gasLimit },
    } = config;

    const currentVerifierSet = await getCurrentVerifierSet(config, chain);
    printInfo('Current verifier set:', currentVerifierSet);

    const fee = gasLimit === 'auto' ? 'auto' : calculateFee(gasLimit, GasPrice.fromString(gasPrice));

    let tx = await executeTransaction(client, account, config.axelar.contracts.MultisigProver[chain].address, 'update_verifier_set', fee);
    printInfo('Update Verifier set tx:', tx);

    const nextVerifierSet = (await getNextVerifierSet(config, chain)).verifier_set;
    printInfo('Next verifier set:', nextVerifierSet);

    const verificationSet = {
        verify_verifier_set: {
            message_id: 'DJxPt5YpU3q46ZoRyRTRcNdbzbQu7ANFAjpLgVFReXke-0', // TODO: break script into 2 subcommands to avoid taking inputs, this is an example message id
            new_verifier_set: nextVerifierSet,
        },
    };
    tx = await executeTransaction(client, account, config.axelar.contracts.VotingVerifier[chain].address, verificationSet, fee);
    printInfo('Initiate verifier set verification tx', tx);

    await getVerifierSetStatus(config, chain, nextVerifierSet);
    tx = await executeTransaction(client, account, config.axelar.contracts.MultisigProver[chain].address, 'confirm_verifier_set', fee);
    printInfo('Initiate verifier set verification tx', tx);
};

const processCommand = async (options, chain) => {
    const config = loadConfig(options.env);
    await processVerifierRotation(config, options, chain);
};

const programHandler = () => {
    const program = new Command();

    program.name('rotate-signers').description('Rotate signers').argument('<chain>', 'Chain to rotate signers for');

    addAmplifierOptions(program, {});

    program.action((chain, options) => {
        processCommand(options, chain);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
