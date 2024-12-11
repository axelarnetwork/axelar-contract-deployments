const { Command } = require('commander');
const { Contract, nativeToScVal } = require('@stellar/stellar-sdk');

const { saveConfig, loadConfig, addOptionsToCommands, getChainConfig } = require('../common');
const { addBaseOptions, getWallet, broadcast } = require('./utils');
const { prompt } = require('../common/utils');

async function setTrustedChain(wallet, _, chain, contractConfig, arg, options) {
    const contract = new Contract(contractConfig.address);
    const callArg = nativeToScVal(arg, { type: 'string' });

    const operation = contract.call('set_trusted_chain', callArg);

    await broadcast(operation, wallet, chain, 'Trusted Chain Set', options);
}

async function removeTrustedChain(wallet, _, chain, contractConfig, arg, options) {
    const contract = new Contract(contractConfig.address);
    const callArg = nativeToScVal(arg, { type: 'string' });

    const operation = contract.call('remove_trusted_chain', callArg);

    await broadcast(operation, wallet, chain, 'Trusted Chain Removed', options);
}

async function mainProcessor(processor, args, options) {
    const { yes } = options;
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
        .command('set-trusted-chain <chainName>')
        .description('set a trusted ITS chain')
        .action((chainName, options) => {
            mainProcessor(setTrustedChain, chainName, options);
        });

    program
        .command('remove-trusted-chain <chainName>')
        .description('remove a trusted ITS chain')
        .action((chainName, options) => {
            mainProcessor(removeTrustedChain, chainName, options);
        });

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
