const { Command } = require('commander');
const { loadConfig, saveConfig, getChainConfig, printInfo } = require('../common/utils');
const { addBaseOptions, addOptionsToCommands, getWallet } = require('./utils');
const { Cl } = require('@stacks/transactions');
const { sendContractCallTransaction } = require('./utils/sign-utils');

async function setOwner(wallet, chain, args) {
    const [contract, governanceAddress] = args;

    const contracts = chain.contracts;
    if (!contracts?.[contract]?.address) {
        throw new Error(`Contract ${contract} not yet deployed`);
    }

    const contractAddress = contracts[contract].address;

    printInfo(`Setting owner for contract ${contract}, address ${contractAddress}`);

    const result = await sendContractCallTransaction(contractAddress, 'set-owner', [Cl.address(governanceAddress)], wallet);

    printInfo(`Finished setting owner`, result.txid);
}

async function transferOwnership(wallet, chain, args) {
    const [contract, ownerAddress] = args;

    const contracts = chain.contracts;
    if (!contracts?.[contract]?.address) {
        throw new Error(`Contract ${contract} not yet deployed`);
    }

    const contractAddress = contracts[contract].address;

    printInfo(`Transferring ownership for contract ${contract}, address ${contractAddress}`);

    const result = await sendContractCallTransaction(contractAddress, 'transfer-ownership', [Cl.address(ownerAddress)], wallet);

    printInfo(`Finished transferring ownership`, result.txid);
}

async function processCommand(command, chain, args, options) {
    const wallet = await getWallet(chain, options);

    await command(wallet, chain, args, options);
}

async function mainProcessor(command, options, args, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config.chains, options.chainName);
    await processor(command, chain, args, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();
    program.name('Stacks Commands').description('Stacks scripts');

    const setOwnerCmd = new Command()
        .name('set-owner')
        .description('Set the owner of a contract')
        .command('set-owner <contract> <governance-address>')
        .action((contract, governanceAddress, options) => {
            mainProcessor(setOwner, options, [contract, governanceAddress], processCommand);
        });

    const transferOwnershipCmd = new Command()
        .name('transfer-ownership')
        .description('Transfer the ownership of a contract')
        .command('transfer-ownership <contract> <owner-address>')
        .action((contract, ownerAddress, options) => {
            mainProcessor(transferOwnership, options, [contract, ownerAddress], processCommand);
        });

    program.addCommand(setOwnerCmd);
    program.addCommand(transferOwnershipCmd);

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
