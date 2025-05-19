const { Command } = require('commander');
const { loadConfig, saveConfig, getChainConfig, printInfo, parseTrustedChains } = require('../common/utils');
const {
    addBaseOptions,
    addOptionsToCommands,
    getWallet,
} = require('./utils');
const {
    makeContractCall,
    PostConditionMode,
    AnchorMode,
    broadcastTransaction,
    Cl,
} = require('@stacks/transactions');

const AXELAR_HUB_IDENTIFIER = 'hub';

async function setTrustedAddress(privateKey, networkType, config, chain, args) {
    const contracts = chain.contracts;
    if (!contracts.InterchainTokenService?.address) {
        throw new Error(`Contract InterchainTokenService not yet deployed`);
    }
    if (!contracts.InterchainTokenServiceImpl?.address) {
        throw new Error(`Contract InterchainTokenServiceImpl not yet deployed`);
    }

    const trustedChains = parseTrustedChains(config, args);

    for (const trustedChain of trustedChains) {
        printInfo(`Setting trusted address for ${trustedChain}`);

        const itsAddress = contracts.InterchainTokenService.address.split('.');
        const registerTransaction = await makeContractCall({
            contractAddress: itsAddress[0],
            contractName: itsAddress[1],
            functionName: 'set-trusted-address',
            functionArgs: [
                Cl.address(contracts.InterchainTokenServiceImpl.address),
                Cl.stringAscii(trustedChain),
                Cl.stringAscii(AXELAR_HUB_IDENTIFIER),
            ],
            senderKey: privateKey,
            network: networkType,
            postConditionMode: PostConditionMode.Allow,
            anchorMode: AnchorMode.Any,
            fee: 10_000,
        });
        const result = await broadcastTransaction({
            transaction: registerTransaction,
            network: networkType,
        });

        printInfo(`Finished setting ${trustedChain} as trusted`, result.txid);

        // Wait a bit before executing next transaction
        await new Promise((resolve) => setTimeout(resolve, 5_000));
    }
}

async function removeTrustedAddress(privateKey, networkType, config, chain, args) {
    const contracts = chain.contracts;
    if (!contracts.InterchainTokenService?.address) {
        throw new Error(`Contract InterchainTokenService not yet deployed`);
    }
    if (!contracts.InterchainTokenServiceImpl?.address) {
        throw new Error(`Contract InterchainTokenServiceImpl not yet deployed`);
    }

    const trustedChains = parseTrustedChains(config, args);

    for (const trustedChain of trustedChains) {
        printInfo(`Removing trusted address for ${trustedChain}`);

        const itsAddress = contracts.InterchainTokenService.address.split('.');
        const registerTransaction = await makeContractCall({
            contractAddress: itsAddress[0],
            contractName: itsAddress[1],
            functionName: 'remove-trusted-address',
            functionArgs: [
                Cl.address(contracts.InterchainTokenServiceImpl.address),
                Cl.stringAscii(trustedChain),
            ],
            senderKey: privateKey,
            network: networkType,
            postConditionMode: PostConditionMode.Allow,
            anchorMode: AnchorMode.Any,
            fee: 10_000,
        });
        const result = await broadcastTransaction({
            transaction: registerTransaction,
            network: networkType,
        });

        printInfo(`Finished removing ${trustedChain} from trusted`, result.txid);

        // Wait a bit before executing next transaction
        await new Promise((resolve) => setTimeout(resolve, 5_000));
    }
}

async function processCommand(command, config, chain, args, options) {
    const { privateKey, networkType } = await getWallet(chain, options);

    await command(privateKey, networkType, config, chain, args, options);
}

async function mainProcessor(command, options, args, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    await processor(command, config, chain, args, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();
    program.name('Gas Service Commands').description('Stacks GasService scripts');

    const setTrustedAddressCmd = new Command()
        .name('set-trusted-address')
        .command('set-trusted-address <trusted-chains...>')
        .description(
            `Set trusted address for chains. The <trusted-chains> can be a list of chains separated by whitespaces. It can also be a special tag to indicate a specific set of chains e.g. 'all' to target all InterchainTokenService-deployed chains`,
        )
        .action((trustedChains, options) => {
            mainProcessor(setTrustedAddress, options, trustedChains, processCommand);
        });

    const removeTrustedAddressCmd = new Command()
        .name('remove-trusted-address')
        .description('Remove trusted address')
        .command('remove-trusted-address <trusted-chains...>')
        .action((trustedChains, options) => {
            mainProcessor(removeTrustedAddress, options, trustedChains, processCommand);
        });

    program.addCommand(setTrustedAddressCmd);
    program.addCommand(removeTrustedAddressCmd);

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
