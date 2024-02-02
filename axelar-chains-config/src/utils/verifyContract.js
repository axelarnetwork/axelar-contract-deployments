const { execSync } = require('child_process');
const { writeFileSync } = require('fs');
const fs = require('fs');
const path = require('path');
const axios = require('axios');

async function getSmallestFile(directoryPath) {
    const fileNames = await fs.promises.readdir(directoryPath);
    let smallestFile = null;
    let smallestSize = Number.MAX_SAFE_INTEGER;

    for (const fileName of fileNames) {
        const filePath = path.join(directoryPath, fileName);
        const stats = await fs.promises.stat(filePath);

        if (stats.size < smallestSize) {
            smallestFile = filePath;
            smallestSize = stats.size;
        }
    }

    return smallestFile;
}

function getContractFileName(config, address) {
    for (const [key, currentValue] of Object.entries(config)) {
        if (currentValue === address) {
            return key.charAt(0).toUpperCase() + key.slice(1);
        }
    }

    return null;
}

async function findFilePath(startPath, targetFileName) {
    if (!fs.existsSync(startPath)) {
        throw new Error(`Start path does not exist: ${startPath}`);
    }

    const directoriesToSearch = [startPath];

    while (directoriesToSearch.length > 0) {
        const currentDirectory = directoriesToSearch.pop();
        const filesAndDirectories = fs.readdirSync(currentDirectory, { withFileTypes: true });

        for (const fileOrDirectory of filesAndDirectories) {
            const fullPath = path.join(currentDirectory, fileOrDirectory.name);

            if (fileOrDirectory.isDirectory()) {
                directoriesToSearch.push(fullPath);
            } else if (fileOrDirectory.isFile() && fileOrDirectory.name === targetFileName) {
                return fullPath;
            }
        }
    }

    throw new Error(`File not found: ${targetFileName}`);
}

async function verifyFilfox(env, contract, options) {
    const { dir, contractName } = options;

    if (!dir || !contractName) {
        throw new Error('Invalid verification options');
    }

    const config = require(`../../info/${env}.json`);
    const contractConfig = config.chains.filecoin.contracts[contractName];
    const contractFileName = getContractFileName(contractConfig, contract);

    const contractPath = await findFilePath(dir, `${contractFileName}.sol`);

    const sourceFiles = {
        [`${contractFileName}.sol`]: {
            content: fs.readFileSync(contractPath, 'utf8'),
        },
    };

    let buildInfo;

    try {
        const buildInfoPath = path.join(dir, 'build-info');
        const smallestFile = await getSmallestFile(buildInfoPath);

        if (!smallestFile) {
            throw new Error('No build info files found');
        }

        const buildInfoContent = fs.readFileSync(smallestFile, 'utf8');
        buildInfo = JSON.parse(buildInfoContent);
    } catch (error) {
        console.error('Error reading contract build info', error);
    }

    const language = buildInfo.input?.language;
    const compiler = buildInfo.solcLongVersion;
    const settings = buildInfo.input?.settings;

    const optimize = settings.optimizer?.enabled || true;
    const optimizeRuns = settings.optimizer?.runs;
    const optimizerDetails = settings.optimizer?.details || '';
    const evmVersion = settings.evmVersion;

    const data = {
        address: contract,
        language,
        compiler,
        optimize,
        optimizeRuns,
        optimizerDetails,
        sourceFiles,
        license: 'MIT License (MIT)',
        evmVersion,
        viaIR: false,
        libraries: '',
        metadata: '',
    };

    const api = config.chains.filecoin.explorer?.api;

    if (!api) {
        throw new Error(`Explorer API not present for filecoin ${env}`);
    }

    try {
        const response = await axios.post(api, data, {
            headers: {
                'Content-Type': 'application/json',
            },
        });

        console.log('Verification response:', JSON.stringify(response.data));
    } catch (error) {
        console.error('Error during verification:', JSON.stringify(error.response.data));
    }
}

/**
 * Verifies a contract on etherscan-like explorer of the provided chain using hardhat.
 * This assumes that the chain has been loaded as a custom network in hardhat.
 *
 * @async
 * @param {string} env
 * @param {string} chain
 * @param {string} contract
 * @param {any[]} args
 * @returns {void}
 */
const verifyContract = async (env, chain, contract, args, options = {}) => {
    if (chain.toLowerCase() === 'filecoin') {
        await verifyFilfox(env, contract, options);
        return;
    }

    const stringArgs = args.map((arg) => JSON.stringify(arg));
    const content = `module.exports = [\n    ${stringArgs.join(',\n    ')}\n];`;
    const file = 'temp-arguments.js';
    const filePath = options.dir ? `${options.dir}/temp-arguments.js` : 'temp-arguments.js';

    const contractArg = options.contractPath ? `--contract ${options.contractPath}` : '';
    const dirPrefix = options.dir ? `cd ${options.dir};` : '';
    const cmd = `${dirPrefix} ENV=${env} npx hardhat verify --network ${chain.toLowerCase()} ${contractArg} --no-compile --constructor-args ${file} ${contract} --show-stack-traces`;

    writeFileSync(filePath, content, 'utf-8');

    console.log(`Verifying contract ${contract} with args '${stringArgs.join(',')}'`);
    console.log(cmd);

    execSync(cmd, { stdio: 'inherit' });

    console.log('Verified!');
};

module.exports = {
    verifyContract,
};
