require('dotenv').config();
const { parseEnv } = require('@axelar-network/axelar-cgp-sui/scripts/utils');
const { publishAll } = require('@axelar-network/axelar-cgp-sui/scripts/publish-all');
const { requestSuiFromFaucetV0, getFaucetHost } = require('@mysten/sui.js/faucet');
const { SuiClient, getFullnodeUrl } = require('@mysten/sui.js/client');
const { Ed25519Keypair } = require('@mysten/sui.js/keypairs/ed25519');
const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { execSync } = require('child_process');
const { publishPackageFull } = require('@axelar-network/axelar-cgp-sui/scripts/publish-package');

(async () => {
    const env = parseEnv('localnet');
    const privKey =
        Buffer.from(
            process.env.SUI_PRIVATE_KEY,
            "hex"
        );
        const keypair = Ed25519Keypair.fromSecretKey(privKey);
        const address = keypair.getPublicKey().toSuiAddress();
        // create a new SuiClient object pointing to the network you want to use
        const client = new SuiClient({ url: env.url });

    await publishPackageFull('axelar', client, keypair, env);
})();