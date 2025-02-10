const { getBagContentFields, getWallet, printWalletInfo } = require('./utils');
const { loadConfig, getChainConfig } = require('../common/utils');

require('dotenv').config();

async function main() {
    const options = {
        signatureScheme: 'secp256k1',
        privateKeyType: 'bech32',
        privateKey: process.env.PRIVATE_KEY,
        chainName: 'sui',
    };

    const config = loadConfig('devnet-amplifier');
    const chain = getChainConfig(config, options.chainName);

    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);
    const objectType = `0x0fec69a5b777ede171b1cb86281935cdd589a04ff88f12632e0c0343287cb044::aaa::AAA`;

    const bagContent = await getBagContentFields(
        client,
        objectType,
        '0x6a46f0a9caf4fac584c45c3286b67ee928ba16d4081b70babc2a071a57df01ba',
        'registered_coins',
    );

    console.log(bagContent);
}

main();
