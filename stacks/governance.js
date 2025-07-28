const { Command } = require('commander');
const { loadConfig, saveConfig, getChainConfig, printInfo } = require('../common/utils');
const { addBaseOptions, addOptionsToCommands, getWallet } = require('./utils');
const { makeContractCall, PostConditionMode, AnchorMode, broadcastTransaction, Cl } = require('@stacks/transactions');

async function setOwner(stacksAddress, privateKey, networkType, chain, args) {
    const [contract, governanceAddress] = args;

    const contracts = chain.contracts;
    if (!contracts?.[contract]?.address) {
        throw new Error(`Contract ${contract} not yet deployed`);
    }

    const contractAddress = contracts[contract].address;

    printInfo(`Setting owner for contract ${contract}, address ${contractAddress}`);

    const contractAddressSplit = contractAddress.split('.');
    const setOwnerTransaction = await makeContractCall({
        contractAddress: contractAddressSplit[0],
        contractName: contractAddressSplit[1],
        functionName: 'set-owner',
        functionArgs: [Cl.address(governanceAddress)],
        senderKey: privateKey,
        network: networkType,
        postConditionMode: PostConditionMode.Allow,
        anchorMode: AnchorMode.Any,
        fee: 10_000,
    });
    const result = await broadcastTransaction({
        transaction: setOwnerTransaction,
        network: networkType,
    });

    printInfo(`Finished setting owner`, result.txid);
}

async function processCommand(command, chain, args, options) {
    const { privateKey, stacksAddress, networkType } = await getWallet(chain, options);

    await command(stacksAddress, privateKey, networkType, chain, args, options);
}

async function mainProcessor(command, options, args, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    await processor(command, chain, args, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();
    program.name('Governance Commands').description('Stacks Governance scripts');

    const setOwnerCmd = new Command()
        .name('set-owner')
        .description('Set the owner of a contract')
        .command('set-owner <contract> <governance-address>')
        .action((contract, governanceAddress, options) => {
            mainProcessor(setOwner, options, [contract, governanceAddress], processCommand);
        });

    program.addCommand(setOwnerCmd);

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
