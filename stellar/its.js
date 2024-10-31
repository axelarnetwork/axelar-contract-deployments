const { Command } = require('commander');
const { Contract, nativeToScVal} = require('@stellar/stellar-sdk');

const { saveConfig, loadConfig, addOptionsToCommands, getChainConfig } = require('../common');
const { addBaseOptions, getWallet, broadcast } = require('./utils');
const { prompt } = require('../common/utils');

async function setTrustedAddress(wallet, _, chain, contractConfig, args, options) {
    const contract = new Contract(contractConfig.address);
    const [chainName, trustedAddress] = args;
    const callArgs = [chainName, trustedAddress].map(nativeToScVal);

    const operation = contract.call('set_trusted_address', ...callArgs);

    await broadcast(operation, wallet, chain, 'Trusted Address Set', options);
}

async function removeTrustedAddress(wallet, _, chain, contractConfig, arg, options) {
    const contract = new Contract(contractConfig.address);
    const callArg = nativeToScVal(arg, { type: 'string' });

    const operation = contract.call('remove_trusted_address', callArg);

    await broadcast(operation, wallet, chain, 'Trusted Address Removed', options);
}

async function mainProcessor(processor, args, options) {
    const { action, yes } = options;
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    const wallet = await getWallet(chain, options);

    if (prompt(`Proceed with action ${processor.name}`, yes)) {
        return;
    }

    if (!chain.contracts?.interchain_token_service) {
        throw new Error('Interchain Token Service package not found.');
    }

    await processor(wallet, config, chain, chain.contracts.interchain_token_service, args, options);

    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('its').description('Interchain Token Service contract operations.');
    
    program
        .command('set-trusted-address <chainName> <trustedAddress>')
        .description('set a trusted ITS address for a given chain')
        .action((chainName, trustedAddress, options) => {
            mainProcessor(setTrustedAddress, [chainName, trustedAddress], options);
        })
    
    program
        .command('remove-trusted-address <chainName>')
        .description('remove a trusted ITS address for a given chain')
        .action((chainName, options) => {
            mainProcessor(removeTrustedAddress, chainName, options);
        });

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
