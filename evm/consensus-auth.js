'use strict';

const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {
    ContractFactory,
    Contract,
    utils: { defaultAbiCoder, keccak256 },
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

const AxelarGateway = require('@axelar-network/axelar-cgp-solidity/artifacts/contracts/AxelarGateway.sol/AxelarGateway.json');
const AxelarAuthWeighted = require('@axelar-network/axelar-cgp-solidity/artifacts/contracts/auth/AxelarAuthWeighted.sol/AxelarAuthWeighted.json');
const IDeployer = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IDeployer.json');

// deploying with no seed keeps the init code, and therefore the CREATE2 address, independent of the operator sets
const EMPTY_SEED = [[]];

// AxelarAuthWeighted rejects a proof whose operator set is this many epochs behind the current one
const OLD_KEY_RETENTION = 16;

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

async function resolveAuthAddress(wallet, chain, options) {
    if (Boolean(options.salt) === Boolean(options.authModule)) {
        throw new Error('Provide exactly one of --salt or --authModule');
    }

    if (options.authModule) {
        return options.authModule;
    }

    return (await predictAddress(wallet, chain, options.salt)).address;
}

// candidates are oldest first, so epochs land in the same order the live auth saw them
async function selectSetsToSeed(auth, candidates) {
    const seeds = [];

    for (const set of candidates) {
        // skip anything already registered: transferOperatorship reverts DuplicateOperators
        if (!seeds.includes(set) && (await auth.epochForHash(operatorsHash(set))).eq(0)) {
            seeds.push(set);
        }
    }

    return seeds;
}

function encodeOperatorSet(operators, weights, threshold) {
    return defaultAbiCoder.encode(['address[]', 'uint256[]', 'uint256'], [operators, weights, threshold]);
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
    const allowedOwners = [].concat(expectedOwner || []);
    const owner = await auth.owner();
    const currentEpoch = await auth.currentEpoch();

    printInfo('Auth owner', owner);
    printInfo('Auth currentEpoch', currentEpoch.toString());

    if (currentEpoch.gt(0)) {
        printInfo('Auth newest epoch hash', await auth.hashForEpoch(currentEpoch));
    }

    if (allowedOwners.length > 0 && !allowedOwners.some((allowed) => allowed.toLowerCase() === owner.toLowerCase())) {
        printError(`Owner is ${owner}, expected ${allowedOwners.join(' or ')}`);
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

// block times differ by two orders of magnitude across the fleet, so a fixed block count is not a usable window.
// Sample the recent average and convert the requested number of days into blocks.
async function blocksPerDay(provider, latest) {
    const span = Math.min(latest, 10000);

    if (span === 0) {
        throw new Error('Chain has no history to sample a block time from');
    }

    const [head, earlier] = await Promise.all([provider.getBlock(latest), provider.getBlock(latest - span)]);
    const seconds = (head.timestamp - earlier.timestamp) / span;

    if (!(seconds > 0)) {
        throw new Error(`Sampled a non positive block time of ${seconds}s; pass --lookbackBlocks instead`);
    }

    return Math.ceil(86400 / seconds);
}

// AxelarAuthWeighted keeps a proof valid for OLD_KEY_RETENTION epochs, so a batch signed before a rotation still
// validates against the live auth. Copy the tail of its history so the replacement accepts those batches too.
async function recentOperatorSets(provider, chain, options) {
    const count = Number(options.copyEpochs);

    if (!Number.isInteger(count) || count < 0 || count >= OLD_KEY_RETENTION) {
        throw new Error(`--copyEpochs must be an integer between 0 and ${OLD_KEY_RETENTION - 1}`);
    }

    if (count === 0) {
        return [];
    }

    const liveAuth = await new Contract(gatewayAddress(chain), AxelarGateway.abi, provider).authModule();
    const auth = new Contract(liveAuth, AxelarAuthWeighted.abi, provider);
    const filter = auth.filters.OperatorshipTransferred();

    const chunk = Number(options.logChunkSize);
    const latest = await provider.getBlockNumber();
    const lookback = options.lookbackBlocks
        ? Number(options.lookbackBlocks)
        : Number(options.lookbackDays) * (await blocksPerDay(provider, latest));
    const floor = Math.max(0, latest - lookback + 1);

    printInfo('Live auth module', liveAuth);
    printInfo('Scanning back', `${lookback} blocks in ${chunk} block requests`);

    const events = [];
    let to = latest;

    // walk backwards and stop as soon as enough rotations are in hand, so the window only costs what it needs to
    while (events.length < count && to >= floor) {
        const from = Math.max(floor, to - chunk + 1);
        events.unshift(...(await auth.queryFilter(filter, from, to)));
        to = from - 1;
    }

    // a live auth has hundreds of epochs, so finding nothing means the RPC did not serve the logs, not that
    // there is no history. Several providers gate getLogs beyond a few recent blocks behind an archive tier.
    if (events.length === 0) {
        throw new Error(
            `No OperatorshipTransferred logs from ${liveAuth} in the last ${lookback} blocks. ` +
                'Use an RPC that serves historical getLogs, or pass --copyEpochs 0 to skip copying history.',
        );
    }

    if (events.length < count) {
        printWarn(
            `Found ${events.length} of ${count} requested operator set(s) in the last ${lookback} blocks of ${liveAuth}`,
            'raise --lookbackDays to copy more history',
        );
    }

    const sets = events.slice(-count).map(({ args }) => encodeOperatorSet(args.newOperators, args.newWeights, args.newThreshold));
    printInfo('Historical operator sets to copy', sets.length);

    return sets;
}

async function seed(axelar, chain, chains, options) {
    // both flags add older sets, from different sources; mixing them makes the resulting epoch order unpredictable
    if (options.prevKeyIDs && Number(options.copyEpochs) !== 0) {
        throw new Error('Pass either --prevKeyIDs or --copyEpochs; combine --prevKeyIDs with --copyEpochs 0');
    }

    const provider = ethers.getDefaultProvider(chain.rpc);
    const wallet = await getWallet(options.privateKey, provider, options);
    const gasOptions = await getGasOptions(chain, options, 'AxelarAuthWeighted');

    const proxy = gatewayAddress(chain);
    const address = await resolveAuthAddress(wallet, chain, options);

    printInfo('Gateway proxy', proxy);
    printInfo('Auth module', address);

    if (!(await isContract(address, provider))) {
        throw new Error(`No auth module at ${address}; run \`deploy\` first`);
    }

    const auth = authFactory(wallet).attach(address);

    if (!(await reportState(auth, wallet.address))) {
        throw new Error(`Auth at ${address} is not owned by ${wallet.address}; it cannot be seeded`);
    }

    const history = await recentOperatorSets(provider, chain, options);
    const { params } = await getAuthParams(axelar, chain.axelarId, options);
    const liveSet = params[params.length - 1];
    const seeds = await selectSetsToSeed(auth, [...history, ...params]);

    if (!seeds.length) {
        printInfo('Auth already holds every current operator set; nothing to seed', address);
        return;
    }

    // seeds land in order, so the live set has to be last or the newest epoch ends up holding a retired set.
    // that happens when a rerun widens --copyEpochs: the recent sets are already registered and only the older
    // ones are left to add. epochs cannot be rewritten, so the auth is spent; deploy a fresh one with a new salt.
    if (seeds[seeds.length - 1] !== liveSet) {
        throw new Error(
            `Seeding would leave a retired operator set in the newest epoch of ${address}. ` +
                'This auth module cannot be repaired; deploy a replacement with a different --salt and seed it once.',
        );
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
    const address = await resolveAuthAddress(wallet, chain, options);

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
    const address = await resolveAuthAddress(wallet, chain, options);

    printInfo('Auth module', address);

    if (!(await isContract(address, provider))) {
        throw new Error(`No contract at ${address}`);
    }

    const auth = authFactory(wallet).attach(address);
    let ok = await reportState(auth, [proxy, wallet.address]);

    if (ok && (await auth.owner()).toLowerCase() === wallet.address.toLowerCase()) {
        printWarn('Auth is still owned by the seeding wallet', 'run `handoff` before rotations resume flowing through the gateway');
    }

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

    // predict and deploy derive the address from the salt, so they cannot accept an address instead
    const addDerivingOptions = (cmd) => {
        addEvmOptions(cmd);
        cmd.addOption(new Option('-s, --salt <salt>', 'CREATE2 salt for the auth module').makeOptionMandatory(true));

        return cmd;
    };

    // the rest act on an auth that already exists, addressed either by its salt or by its address
    const addExistingOptions = (cmd, verb) => {
        addEvmOptions(cmd);
        cmd.addOption(new Option('-s, --salt <salt>', 'CREATE2 salt the auth module was deployed with'));
        cmd.addOption(new Option('--authModule <authModule>', `${verb} this address instead of deriving it from --salt`));

        return cmd;
    };

    addDerivingOptions(program.command('predict').description('Print the predicted auth module address')).action((options) =>
        mainProcessor(options, predict),
    );

    addDerivingOptions(program.command('deploy').description('Deploy an unseeded auth module owned by the deployer wallet')).action(
        (options) => mainProcessor(options, deploy),
    );

    addExistingOptions(
        program.command('seed').description('Seed the auth module with the current operator sets; repeatable, keeps ownership'),
        'seed',
    )
        .addOption(
            new Option(
                '--prevKeyIDs <prevKeyIDs>',
                'comma separated older key IDs to seed alongside the current one; requires --copyEpochs 0',
            ),
        )
        .addOption(
            new Option('--copyEpochs <copyEpochs>', 'how many recent operator sets to copy from the live auth module').default(
                String(OLD_KEY_RETENTION - 1),
            ),
        )
        .addOption(new Option('--lookbackDays <lookbackDays>', 'how far back to scan for those sets').default('30'))
        .addOption(new Option('--lookbackBlocks <lookbackBlocks>', 'scan this many blocks instead of deriving it from --lookbackDays'))
        .addOption(new Option('--logChunkSize <logChunkSize>', 'block range per getLogs request').default('10000'))
        .action((options) => mainProcessor(options, seed));

    addExistingOptions(
        program.command('handoff').description('Hand auth ownership to the gateway proxy; refuses a stale seed'),
        'hand off',
    ).action((options) => mainProcessor(options, handoff));

    addExistingOptions(
        program.command('verify').description('Check the auth module is seeded with the live operator set and owned by the gateway'),
        'verify',
    ).action((options) => mainProcessor(options, verify));

    program.parse();
}

module.exports = {
    predictAuthAddress: predictAddress,
    resolveAuthAddress,
    blocksPerDay,
    selectSetsToSeed,
    recentOperatorSets,
    encodeOperatorSet,
    operatorsHash,
    OLD_KEY_RETENTION,
};
