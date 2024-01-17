const { getITSContract } = require('../test/utils');
const { ethers } = require('hardhat');

/* const TOKEN_MANAGER_DEPLOYED_HASH = ''; */
const CONFIG_PATH = `${__dirname}/../config/testnet.json`;

async function main() {
    const [wallet] = await ethers.getSigners();
    const tokenService = await getITSContract(CONFIG_PATH, wallet);

    const filter = {
        address: tokenService.address,
        topics: [ethers.utils.id('TokenManagerDeployed(bytes32,address,uint8,bytes)')],
        fromBlock: 0,
    };

    const callPromise = wallet.provider.getLogs(filter);
    console.log('new logs started');
    callPromise.then((events) => {
        events.map((log) => {
            console.log('=======');
            console.log(log);
        });
    });
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
});
