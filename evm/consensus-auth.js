'use strict';

const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {
    ContractFactory,
    Contract,
    utils: { keccak256 },
} = ethers;

const {
    printInfo,
    printWarn,
    printError,
    prompt,
    mainProcessor,
    isContract,
    getGasOptions,
    getDeployOptions,
    getDeployedAddress,
    getSaltFromKey,
    wasEventEmitted,
} = require('./utils');
const { addEvmOptions } = require('./cli-utils');
const { getWallet } = require('./sign-utils');
const { getAuthParams } = require('./deploy-consensus-gateway');

const AxelarAuthWeighted = require('@axelar-network/axelar-cgp-solidity/artifacts/contracts/auth/AxelarAuthWeighted.sol/AxelarAuthWeighted.json');
const IDeployer = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IDeployer.json');

// deploying with no seed keeps the init code, and therefore the CREATE2 address, independent of the operator sets
const EMPTY_SEED = [[]];

function authFactory(wallet) {
    return new ContractFactory(AxelarAuthWeighted.abi, AxelarAuthWeighted.bytecode, wallet);
}

async function predictAddress(wallet, chain, salt) {
    const { deployerContract } = getDeployOptions('create2', salt, chain);

    const address = await getDeployedAddress(wallet.address, 'create2', {
        salt,
        deployerContract,
        contractJson: AxelarAuthWeighted,
        constructorArgs: EMPTY_SEED,
        provider: wallet.provider,
    });

    return { address, deployerContract };
}

function gatewayAddress(chain) {
    const address = chain.contracts.AxelarGateway?.address;

    if (!address) {
        throw new Error(`AxelarGateway is not deployed on ${chain.name}`);
    }

    if (chain.contracts.AxelarGateway.connectionType === 'amplifier') {
        throw new Error(`${chain.name} runs an amplifier gateway, which has no AxelarAuthWeighted`);
    }

    return address;
}

// core encodes operator sets canonically, so hashing the encoded params matches the contract's
// keccak256(abi.encode(operators, weights, threshold))
function operatorsHash(params) {
    return keccak256(params);
}

async function reportState(auth, expectedOwner) {
    const owner = await auth.owner();
    const currentEpoch = await auth.currentEpoch();

    printInfo('Auth owner', owner);
    printInfo('Auth currentEpoch', currentEpoch.toString());

    if (currentEpoch.gt(0)) {
        printInfo('Auth newest epoch hash', await auth.hashForEpoch(currentEpoch));
    }

    if (expectedOwner && owner.toLowerCase() !== expectedOwner.toLowerCase()) {
        printError(`Owner is ${owner}, expected ${expectedOwner}`);
        return false;
    }

    return true;
}

async function predict(axelar, chain, chains, options) {
    const wallet = await getWallet(options.privateKey, ethers.getDefaultProvider(chain.rpc), options);
    const { address, deployerContract } = await predictAddress(wallet, chain, options.salt);

    printInfo('Deployer contract', deployerContract);
    printInfo('Salt', options.salt);
    printInfo('Predicted auth module', address);
    printInfo('Already deployed', (await isContract(address, wallet.provider)) ? 'yes' : 'no');
}

async function deploy(axelar, chain, chains, options) {
    const provider = ethers.getDefaultProvider(chain.rpc);
    const wallet = await getWallet(options.privateKey, provider, options);
    const gasOptions = await getGasOptions(chain, options, 'AxelarAuthWeighted');

    const { address, deployerContract } = await predictAddress(wallet, chain, options.salt);

    printInfo('Auth module', address);

    if (await isContract(address, provider)) {
        throw new Error(`Auth module already deployed at ${address}. Use a fresh salt.`);
    }

    if (prompt(`Deploy an unseeded auth at ${address} on ${chain.name}, owned by ${wallet.address}?`, options.yes)) {
        return;
    }

    const deployer = new Contract(deployerContract, IDeployer.abi, wallet);
    const initCode = authFactory(wallet).getDeployTransaction(...EMPTY_SEED).data;
    const init = new ethers.utils.Interface(AxelarAuthWeighted.abi).encodeFunctionData('transferOwnership', [wallet.address]);

    const deployTx = await deployer.deployAndInit(initCode, getSaltFromKey(options.salt), init, gasOptions);
    printInfo('Deploy tx', deployTx.hash);
    await deployTx.wait(chain.confirmations);

    if (!(await isContract(address, provider))) {
        throw new Error(`Auth module was not deployed at the predicted address ${address}`);
    }

    const auth = authFactory(wallet).attach(address);

    if (!(await reportState(auth, wallet.address))) {
        throw new Error('Auth ownership was not handed to the deployer wallet; it cannot be seeded');
    }

    printInfo('Auth deployed unseeded; run `seed` shortly before executing the gateway upgrade', address);
}

async function seed(axelar, chain, chains, options) {
    const provider = ethers.getDefaultProvider(chain.rpc);
    const wallet = await getWallet(options.privateKey, provider, options);
    const gasOptions = await getGasOptions(chain, options, 'AxelarAuthWeighted');

    const proxy = gatewayAddress(chain);
    const address = options.authModule || (await predictAddress(wallet, chain, options.salt)).address;

    printInfo('Gateway proxy', proxy);
    printInfo('Auth module', address);

    if (!(await isContract(address, provider))) {
        throw new Error(`No auth module at ${address}; run \`deploy\` first`);
    }

    const auth = authFactory(wallet).attach(address);

    if (!(await reportState(auth, wallet.address))) {
        throw new Error(`Auth at ${address} is not owned by ${wallet.address}; it cannot be seeded`);
    }

    const { params } = await getAuthParams(axelar, chain.axelarId, options);
    const seeds = [];

    for (const set of params) {
        // skip anything already registered: transferOperatorship reverts DuplicateOperators
        if (!seeds.includes(set) && (await auth.epochForHash(operatorsHash(set))).eq(0)) {
            seeds.push(set);
        }
    }

    if (!seeds.length) {
        printInfo('Auth already holds every current operator set; nothing to seed', address);
        return;
    }

    printInfo('Operator sets to seed', seeds.length);

    if (prompt(`Seed ${seeds.length} set(s) into ${address}? Ownership stays with ${wallet.address}.`, options.yes)) {
        return;
    }

    for (const [index, set] of seeds.entries()) {
        const tx = await auth.transferOperatorship(set, gasOptions);
        printInfo(`Seed ${index + 1}/${seeds.length} tx`, tx.hash);

        const receipt = await tx.wait(chain.confirmations);

        if (!wasEventEmitted(receipt, auth, 'OperatorshipTransferred')) {
            throw new Error(`Seeding operator set ${index + 1} did not emit OperatorshipTransferred`);
        }
    }

    const seededEpoch = await auth.currentEpoch();
    const newestHash = await auth.hashForEpoch(seededEpoch);
    const expectedHash = operatorsHash(seeds[seeds.length - 1]);

    if (newestHash !== expectedHash) {
        throw new Error(`Newest epoch hash is ${newestHash}, expected ${expectedHash}`);
    }

    await reportState(auth, wallet.address);

    printInfo('Seeded; ownership NOT transferred yet. Run `handoff` immediately before executing the upgrade', address);
}

async function handoff(axelar, chain, chains, options) {
    const provider = ethers.getDefaultProvider(chain.rpc);
    const wallet = await getWallet(options.privateKey, provider, options);
    const gasOptions = await getGasOptions(chain, options, 'AxelarAuthWeighted');

    const proxy = gatewayAddress(chain);
    const address = options.authModule || (await predictAddress(wallet, chain, options.salt)).address;

    printInfo('Gateway proxy', proxy);
    printInfo('Auth module', address);

    if (!(await isContract(address, provider))) {
        throw new Error(`No auth module at ${address}; run \`deploy\` first`);
    }

    const auth = authFactory(wallet).attach(address);

    if (!(await reportState(auth, wallet.address))) {
        throw new Error(`Auth at ${address} is not owned by ${wallet.address}`);
    }

    // handing off a stale seed is unrecoverable: only the owner can add sets, and that becomes the proxy
    const { params } = await getAuthParams(axelar, chain.axelarId, options);
    const liveHash = operatorsHash(params[params.length - 1]);
    const currentEpoch = await auth.currentEpoch();

    if (currentEpoch.eq(0)) {
        throw new Error('Auth is unseeded; run `seed` first');
    }

    if ((await auth.hashForEpoch(currentEpoch)) !== liveHash) {
        throw new Error(`Newest epoch does not match the live operator set ${liveHash}; run \`seed\` again before handing off`);
    }

    if (prompt(`Hand ownership of ${address} to ${proxy}? This is one way.`, options.yes)) {
        return;
    }

    const tx = await auth.transferOwnership(proxy, gasOptions);
    printInfo('Transfer ownership tx', tx.hash);
    await tx.wait(chain.confirmations);

    await reportState(auth, proxy);

    printInfo('Auth module ready', address);
}

async function verify(axelar, chain, chains, options) {
    const provider = ethers.getDefaultProvider(chain.rpc);
    const wallet = await getWallet(options.privateKey, provider, options);

    const proxy = gatewayAddress(chain);
    const address = options.authModule || (await predictAddress(wallet, chain, options.salt)).address;

    printInfo('Auth module', address);

    if (!(await isContract(address, provider))) {
        throw new Error(`No contract at ${address}`);
    }

    const auth = authFactory(wallet).attach(address);
    let ok = await reportState(auth, proxy);

    const { params } = await getAuthParams(axelar, chain.axelarId, options);
    const liveHash = operatorsHash(params[params.length - 1]);
    const currentEpoch = await auth.currentEpoch();
    const newestHash = await auth.hashForEpoch(currentEpoch);

    printInfo('Live operator set hash', liveHash);

    if (newestHash !== liveHash) {
        printError(`Newest epoch hash ${newestHash} does not match the live operator set ${liveHash}`);
        ok = false;
    }

    if (!(await auth.epochForHash(liveHash)).gt(0)) {
        printError('Live operator set is not registered in this auth module');
        ok = false;
    }

    if (!ok) {
        throw new Error('Auth module is NOT ready; do not execute the gateway upgrade');
    }

    printInfo('Auth module verified', address);
}

if (require.main === module) {
    const program = new Command();

    program.name('consensus-auth').description('Deploy and verify a replacement AxelarAuthWeighted for a consensus gateway');

    const addOptions = (cmd, { salt = true } = {}) => {
        addEvmOptions(cmd);

        if (salt) {
            cmd.addOption(new Option('-s, --salt <salt>', 'CREATE2 salt for the auth module').makeOptionMandatory(true));
        }

        cmd.addOption(new Option('--prevKeyIDs <prevKeyIDs>', 'comma separated older key IDs to seed alongside the current one'));

        return cmd;
    };

    addOptions(program.command('predict').description('Print the predicted auth module address')).action((options) =>
        mainProcessor(options, predict),
    );

    addOptions(program.command('deploy').description('Deploy an unseeded auth module owned by the deployer wallet')).action((options) =>
        mainProcessor(options, deploy),
    );

    addOptions(program.command('seed').description('Seed the auth module with the current operator sets; repeatable, keeps ownership'))
        .addOption(new Option('--authModule <authModule>', 'seed this address instead of the predicted one'))
        .action((options) => mainProcessor(options, seed));

    addOptions(program.command('handoff').description('Hand auth ownership to the gateway proxy; refuses a stale seed'))
        .addOption(new Option('--authModule <authModule>', 'hand off this address instead of the predicted one'))
        .action((options) => mainProcessor(options, handoff));

    addOptions(program.command('verify').description('Check the auth module is seeded with the live operator set and owned by the gateway'))
        .addOption(new Option('--authModule <authModule>', 'verify this address instead of the predicted one'))
        .action((options) => mainProcessor(options, verify));

    program.parse();
}

module.exports = {
    predictAuthAddress: predictAddress,
};
