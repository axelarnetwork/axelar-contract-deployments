const { Command } = require('commander');
const { loadConfig, saveConfig, getChainConfig, printInfo, parseTrustedChains, sleep } = require('../common/utils');
const { addBaseOptions, addOptionsToCommands, getWallet } = require('./utils');
const { Cl } = require('@stacks/transactions');
const { sendContractCallTransaction } = require('./utils/sign-utils');

const AXELAR_HUB_IDENTIFIER = 'hub';

async function setTrustedChain(wallet, config, chain, args) {
    const contracts = chain.contracts;
    if (!contracts.InterchainTokenService?.address) {
        throw new Error(`Contract InterchainTokenService not yet deployed`);
    }
    if (!contracts.InterchainTokenServiceImpl?.address) {
        throw new Error(`Contract InterchainTokenServiceImpl not yet deployed`);
    }

    const trustedChains = parseTrustedChains(config.chains, args);

    for (const trustedChain of trustedChains) {
        printInfo(`Setting trusted address for ${trustedChain}`);

        const result = await sendContractCallTransaction(
            contracts.InterchainTokenService.address,
            'set-trusted-address',
            [Cl.address(contracts.InterchainTokenServiceImpl.address), Cl.stringAscii(trustedChain), Cl.stringAscii(AXELAR_HUB_IDENTIFIER)],
            wallet,
        );

        printInfo(`Finished setting ${trustedChain} as trusted`, result.txid);

        // Wait a bit before executing next transaction
        await sleep(5_000);
    }
}

async function removeTrustedChain(wallet, config, chain, args) {
    const contracts = chain.contracts;
    if (!contracts.InterchainTokenService?.address) {
        throw new Error(`Contract InterchainTokenService not yet deployed`);
    }
    if (!contracts.InterchainTokenServiceImpl?.address) {
        throw new Error(`Contract InterchainTokenServiceImpl not yet deployed`);
    }

    const trustedChains = parseTrustedChains(config.chains, args);

    for (const trustedChain of trustedChains) {
        printInfo(`Removing trusted address for ${trustedChain}`);

        const result = await sendContractCallTransaction(
            contracts.InterchainTokenService.address,
            'remove-trusted-address',
            [Cl.address(contracts.InterchainTokenServiceImpl.address), Cl.stringAscii(trustedChain)],
            wallet,
        );

        printInfo(`Finished removing ${trustedChain} from trusted`, result.txid);

        // Wait a bit before executing next transaction
        await sleep(5_000);
    }
}

async function processCommand(command, config, chain, args, options) {
    const wallet = await getWallet(chain, options);

    await command(wallet, config, chain, args, options);
}

async function mainProcessor(command, options, args, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config.chains, options.chainName);
    await processor(command, config, chain, args, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();
    program.name('Gas Service Commands').description('Stacks GasService scripts');

    const setTrustedAddressCmd = new Command()
        .name('set-trusted-chain')
        .command('set-trusted-chain <trusted-chains...>')
        .description(
            `Set a trusted chain. The <trusted-chains> can be a list of chains separated by whitespaces. It can also be a special tag to indicate a specific set of chains e.g. 'all' to target all InterchainTokenService-deployed chains`,
        )
        .action((trustedChains, options) => {
            mainProcessor(setTrustedChain, options, trustedChains, processCommand);
        });

    const removeTrustedAddressCmd = new Command()
        .name('remove-trusted-chain')
        .description('Remove a trusted chain')
        .command('remove-trusted-chain <trusted-chains...>')
        .action((trustedChains, options) => {
            mainProcessor(removeTrustedChain, options, trustedChains, processCommand);
        });

    program.addCommand(setTrustedAddressCmd);
    program.addCommand(removeTrustedAddressCmd);

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
