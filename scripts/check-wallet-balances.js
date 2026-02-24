'use strict';

const path = require('path');
const { ethers } = require('ethers');
const xrpl = require('xrpl');
const { Keypair, Horizon } = require('@stellar/stellar-sdk');
const { SuiClient } = require('@mysten/sui/client');
const { decodeSuiPrivateKey } = require('@mysten/sui/cryptography');
const { Secp256k1Keypair } = require('@mysten/sui/keypairs/secp256k1');

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

const EVM_CHAINS = {
    mainnet: ['monad', 'berachain', 'plume', 'hyperliquid', 'xrpl-evm', 'flow', 'hedera'],
    testnet: ['monad-3', 'berachain', 'plume', 'hyperliquid', 'xrpl-evm', 'flow', 'hedera'],
};

const STELLAR_CHAIN = {
    mainnet: 'stellar',
    testnet: 'stellar-2026-q1-2',
};

const XRPL_CHAIN = 'xrpl';
const SUI_CHAIN = 'sui';

const MAX_RETRIES = 2;
const RETRY_DELAY_MS = 3000;

async function withRetry(fn, label) {
    for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
        try {
            return await fn();
        } catch (err) {
            if (attempt < MAX_RETRIES) {
                console.warn(`  ${label}: attempt ${attempt + 1} failed (${err.message}), retrying in ${RETRY_DELAY_MS / 1000}s...`);
                await new Promise((resolve) => setTimeout(resolve, RETRY_DELAY_MS));
            } else {
                throw err;
            }
        }
    }
}

// Minimum native-token balance thresholds (~5 transactions worth of gas)
const THRESHOLDS = {
    monad: 3,
    'monad-3': 1.5,
    berachain: 0.2,
    plume: 8,
    hyperliquid: 0.01,
    'xrpl-evm': 2,
    flow: 2,
    hedera: 2,
    // Non-EVM chains
    xrpl: 3,
    stellar: 0.5,
    sui: 0.05,
};
const DEFAULT_THRESHOLD = 1;

function loadConfig(env) {
    const configPath = path.resolve(__dirname, '..', 'axelar-chains-config', 'info', `${env}.json`);
    return require(configPath);
}

function parseArgs() {
    const args = process.argv.slice(2);
    const envIdx = args.indexOf('--env');

    if (envIdx === -1 || !args[envIdx + 1]) {
        throw new Error('Usage: node scripts/check-wallet-balances.js --env <mainnet|testnet>');
    }

    const env = args[envIdx + 1];

    if (!['mainnet', 'testnet'].includes(env)) {
        throw new Error('--env must be "mainnet" or "testnet"');
    }

    return env;
}

async function checkEvmBalances(privateKey, chains, config) {
    const wallet = new ethers.Wallet(privateKey);
    const address = wallet.address;
    const results = [];

    for (const chainName of chains) {
        const chain = config.chains[chainName];
        const threshold = THRESHOLDS[chainName] || DEFAULT_THRESHOLD;

        if (!chain) {
            console.warn(`  Warning: chain "${chainName}" not found in config, skipping`);
            continue;
        }

        if (!chain.tokenSymbol) {
            console.error(`  ${chainName}: missing tokenSymbol in config, skipping`);
            continue;
        }

        try {
            const balance = await withRetry(async () => {
                const provider = new ethers.providers.JsonRpcProvider(chain.rpc);
                const balanceWei = await provider.getBalance(address);
                return parseFloat(ethers.utils.formatEther(balanceWei));
            }, chainName);

            console.log(`  ${chainName} (${chain.tokenSymbol}): ${balance}`);
            results.push({ chain: chainName, symbol: chain.tokenSymbol, address, balance, threshold });
        } catch (err) {
            console.error(`  ${chainName}: failed to fetch balance - ${err.message}`);
            results.push({
                chain: chainName,
                symbol: chain.tokenSymbol,
                address,
                balance: 0,
                threshold,
                error: err.message,
            });
        }
    }

    return results;
}

async function checkXrplBalance(privateKey, config) {
    const chain = config.chains[XRPL_CHAIN];

    if (!chain) {
        console.warn(`  Warning: chain "${XRPL_CHAIN}" not found in config, skipping`);
        return [];
    }

    const wallet = xrpl.Wallet.fromSeed(privateKey, { algorithm: xrpl.ECDSA.secp256k1 });
    const address = wallet.address;

    try {
        const balance = await withRetry(async () => {
            const c = new xrpl.Client(chain.wssRpc);

            try {
                await c.connect();

                const response = await c.request({
                    command: 'account_info',
                    account: address,
                    ledger_index: 'validated',
                });

                const balanceDrops = response.result.account_data.Balance;
                return parseFloat(xrpl.dropsToXrp(balanceDrops));
            } finally {
                await c.disconnect();
            }
        }, XRPL_CHAIN);

        console.log(`  ${XRPL_CHAIN} (XRP): ${balance}`);
        return [{ chain: XRPL_CHAIN, symbol: 'XRP', address, balance, threshold: THRESHOLDS.xrpl }];
    } catch (err) {
        if (err.data?.error === 'actNotFound') {
            console.log(`  ${XRPL_CHAIN} (XRP): 0 (account not found)`);
            return [{ chain: XRPL_CHAIN, symbol: 'XRP', address, balance: 0, threshold: THRESHOLDS.xrpl }];
        }

        console.error(`  ${XRPL_CHAIN}: failed to fetch balance - ${err.message}`);
        return [{ chain: XRPL_CHAIN, symbol: 'XRP', address, balance: 0, threshold: THRESHOLDS.xrpl, error: err.message }];
    }
}

async function checkStellarBalance(privateKey, config, env) {
    const chainName = STELLAR_CHAIN[env];
    const chain = config.chains[chainName];

    if (!chain) {
        console.warn(`  Warning: chain "${chainName}" not found in config, skipping`);
        return [];
    }

    const keypair = Keypair.fromSecret(privateKey);
    const address = keypair.publicKey();

    try {
        const balance = await withRetry(async () => {
            const server = new Horizon.Server(chain.horizonRpc);
            const account = await server.accounts().accountId(address).call();
            const native = account.balances.find((b) => b.asset_type === 'native');
            return native ? parseFloat(native.balance) : 0;
        }, chainName);

        console.log(`  ${chainName} (XLM): ${balance}`);
        return [{ chain: chainName, symbol: 'XLM', address, balance, threshold: THRESHOLDS.stellar }];
    } catch (err) {
        if (err?.response?.status === 404) {
            console.log(`  ${chainName} (XLM): 0 (account not found)`);
            return [{ chain: chainName, symbol: 'XLM', address, balance: 0, threshold: THRESHOLDS.stellar }];
        }

        console.error(`  ${chainName}: failed to fetch balance - ${err.message}`);
        return [{ chain: chainName, symbol: 'XLM', address, balance: 0, threshold: THRESHOLDS.stellar, error: err.message }];
    }
}

async function checkSuiBalance(privateKey, config) {
    const chain = config.chains[SUI_CHAIN];

    if (!chain) {
        console.warn(`  Warning: chain "${SUI_CHAIN}" not found in config, skipping`);
        return [];
    }

    const { secretKey } = decodeSuiPrivateKey(privateKey);
    const keypair = Secp256k1Keypair.fromSecretKey(secretKey);
    const address = keypair.toSuiAddress();

    try {
        const balance = await withRetry(async () => {
            const client = new SuiClient({ url: chain.rpc });
            const { totalBalance } = await client.getBalance({ owner: address });
            // SUI balance is in MIST (1 SUI = 1e9 MIST)
            return parseFloat(totalBalance) / 1e9;
        }, SUI_CHAIN);

        console.log(`  ${SUI_CHAIN} (SUI): ${balance}`);
        return [{ chain: SUI_CHAIN, symbol: 'SUI', address, balance, threshold: THRESHOLDS.sui }];
    } catch (err) {
        console.error(`  ${SUI_CHAIN}: failed to fetch balance - ${err.message}`);
        return [{ chain: SUI_CHAIN, symbol: 'SUI', address, balance: 0, threshold: THRESHOLDS.sui, error: err.message }];
    }
}

async function main() {
    const env = parseArgs();
    const config = loadConfig(env);

    const evmPrivateKey = process.env.EVM_PRIVATE_KEY;
    const xrplPrivateKey = process.env.XRPL_PRIVATE_KEY;
    const stellarPrivateKey = process.env.STELLAR_PRIVATE_KEY;
    const suiPrivateKey = process.env.SUI_PRIVATE_KEY;

    const allResults = [];

    console.log('Checking EVM wallet balances...');
    allResults.push(...(await checkEvmBalances(evmPrivateKey, EVM_CHAINS[env], config)));

    console.log('Checking XRPL wallet balance...');
    allResults.push(...(await checkXrplBalance(xrplPrivateKey, config)));

    console.log('Checking Stellar wallet balance...');
    allResults.push(...(await checkStellarBalance(stellarPrivateKey, config, env)));

    console.log('Checking SUI wallet balance...');
    allResults.push(...(await checkSuiBalance(suiPrivateKey, config)));

    const lowBalances = allResults.filter((r) => r.balance < r.threshold);

    if (lowBalances.length > 0) {
        const details = lowBalances.map((e) => `${e.chain} (${e.symbol}): ${e.balance} < ${e.threshold}`).join(', ');
        throw new Error(`Wallet(s) below minimum balance threshold: ${details}`);
    }

    console.log('\nAll wallet balances are above minimum thresholds.');
}

main()
    .then(() => process.exit(0))
    .catch(async (err) => {
        console.error(err);

        try {
            const Sentry = require('@sentry/node');
            Sentry.captureException(err);
            await Sentry.close(2000);
        } catch (_) {}

        process.exit(1);
    });
