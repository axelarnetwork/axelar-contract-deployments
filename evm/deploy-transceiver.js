const { ethers } = require('hardhat');
const {
    Wallet,
    Contract,
    getDefaultProvider,
    utils: { defaultAbiCoder, isAddress },
    ContractFactory,
} = ethers;

const {
    deployContract,
    printWalletInfo,
    saveConfig,
    printInfo,
    printWarn,
    printError,
    getContractJSON,
    mainProcessor,
    prompt,
    sleep,
    getBytecodeHash,
    getGasOptions,
    isContract,
    isValidAddress,
    getDeployOptions,
    getDeployedAddress,
    wasEventEmitted,
    verifyContract,
} = require('./utils');
const { addExtendedOptions } = require('./cli-utils');
const { Command, Option } = require('commander');

const sepolia = '0xaa8267908e8d2BEfeB601f88A7Cf3ec148039423';

const implementation = require('../../example-wormhole-axelar-wsteth/artifacts/src/axelar/AxelarTransceiver.sol/AxelarTransceiver.json');
const proxy = require('../../example-wormhole-axelar-wsteth/artifacts/openzeppelin-contracts/contracts/proxy/ERC1967/ERC1967Proxy.sol/ERC1967Proxy.json');
const library = require('../../example-wormhole-axelar-wsteth/artifacts/@wormhole-foundation/native_token_transfer/libraries/TransceiverStructs.sol/TransceiverStructs.json');
async function processCommand(config, chain, options) {
    const gateway = chain.contracts.AxelarGateway.address;
    const gasService = chain.contracts.AxelarGasService.address
    const nttManager = '0x66Cb5a992570EF01b522Bc59A056a64A84Bd0aAa';
    
    const { privateKey, reuseProxy, reuseHelpers, reuseAuth, verify, yes, predictOnly } = options;
    const rpc = options.rpc || chain.rpc;
    console.log(rpc);
    const provider = getDefaultProvider(rpc);
    const wallet = new Wallet(privateKey).connect(provider);

    console.log(wallet.address, await provider.getBlockNumber());
    let deployerContract =
        options.deployMethod === 'create3' ? chain.contracts.Create3Deployer?.address : chain.contracts.ConstAddressDeployer?.address;
   
    options.deployerContract = deployerContract;

    //const libraryContract = await deployContract('create', wallet, library, [], options);
    //console.log(libraryContract.address);
    const libraryAddress = 'eEcf56798CFC9e927A83F98b0112484623Cf175a';

    const index = implementation.bytecode.indexOf('__');

    const toReplace = implementation.bytecode.slice(index, index+40);
    implementation.bytecode = implementation.bytecode.replace(toReplace, libraryAddress);
    
    //const implementationContract = await deployContract('create', wallet, implementation, [gateway, gasService, nttManager], options);
    //console.log(implementationContract.address);
    const implementationAddress = '0x4FE90D921E279f149Ee7C7E1a79eE75803E846B1';

    deployerContract =
        options.proxyDeployMethod === 'create3' ? chain.contracts.Create3Deployer?.address : chain.contracts.ConstAddressDeployer?.address;
   
    options.deployerContract = deployerContract;
    //let proxyContract = await deployContract(options.proxyDeployMethod, wallet, proxy, [implementationAddress, '0x'], options);
    //console.log(proxyContract.address);
    const proxyAddress = '0xaa8267908e8d2BEfeB601f88A7Cf3ec148039423';
    //await verifyContract('testnet', chain.name, proxyAddress, [implementationAddress, '0x'], {});
    
    let proxyContract = new Contract(proxyAddress, implementation.abi, wallet);
    //await (await proxyContract.setAxelarAddress()).wait();
    console.log(await proxyContract.populateTransaction.setAxelarChainId(10002, 'ethereum-sepolia', '0xaa8267908e8d2BEfeB601f88A7Cf3ec148039423'));


}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('deploy-transceiver').description('Deploy interchain token service and interchain token factory');

    program.addOption(
        new Option('-m, --deployMethod <deployMethod>', 'deployment method').choices(['create', 'create2', 'create3']).default('create2'),
    );
    program.addOption(
        new Option(
            '--proxyDeployMethod <proxyDeployMethod>',
            'proxy deployment method, overrides normal deployment method (defaults to create3)',
        )
            .choices(['create', 'create3'])
            .default('create3'),
    );

    addExtendedOptions(program, { artifactPath: true, skipExisting: true, upgrade: true, predictOnly: true });

    program.addOption(new Option('--reuseProxy', 'reuse existing proxy (useful for upgrade deployments'));
    program.addOption(new Option('-s, --salt <salt>', 'deployment salt to use for ITS deployment').env('SALT'));
    program.addOption(
        new Option(
            '--proxySalt <proxySalt>',
            'deployment salt to use for ITS proxies, this allows deploying latest releases to new chains while deriving the same proxy address',
        )
            .default('v1.0.0')
            .env('PROXY_SALT'),
    );
    program.addOption(
        new Option('-o, --operatorAddress <operatorAddress>', 'address of the ITS operator/rate limiter').env('OPERATOR_ADDRESS'),
    );

    program.action(async (options) => {
        await main(options);
    });

    program.parse();
} else {
    module.exports = { deployITS: deploy };
}
