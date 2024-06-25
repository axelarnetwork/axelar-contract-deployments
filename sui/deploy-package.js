const { getWallet } = require('./sign-utils');
const { updateMoveToml, TxBuilder } = require('@axelar-network/axelar-cgp-sui');

async function deployPackage(chain, options, packageName) {
    const [keypair, client] = getWallet(chain, options);

    const builder = new TxBuilder(client);
    await builder.publishPackageAndTransferCap(packageName, keypair.toSuiAddress());
    const publishTxn = await builder.signAndExecute(keypair);

    const packageId = (publishTxn.objectChanges?.find((a) => a.type === 'published') ?? []).packageId;

    updateMoveToml(packageName, packageId);

    return { packageId, publishTxn };
}

module.exports = {
    deployPackage,
}