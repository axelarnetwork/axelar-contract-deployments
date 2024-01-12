const axios = require('axios');
const fs = require('fs');
const { Command, Option } = require('commander');
const { addBaseOptions } = require('./cli-utils');
const { validateParameters, loadConfig, printInfo, printError } = require('./utils');

async function verifyFilfox(options) {
    const { env, address, contractName, contractPath, compilerVersion, optimizeRuns } = options;

    validateParameters({
        isValidAddress: { address },
        isNonEmptyString: { contractName, contractPath, compilerVersion },
        isValidNumber: { optimizeRuns },
    });

    const sourceFiles = {
        [`${contractName}.sol`]: {
            content: fs.readFileSync(contractPath, 'utf8'),
        },
    };

    const optimizerDetails = '';
    const license = 'MIT License (MIT)';
    const evmVersion = 'london';
    const libraries = '';
    const metadata = '';

    const data = {
        address,
        language: 'Solidity',
        compiler: compilerVersion,
        optimize: true,
        optimizeRuns,
        optimizerDetails,
        sourceFiles,
        license,
        evmVersion,
        viaIR: false,
        libraries,
        metadata,
    };

    const config = loadConfig(env);
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

        printInfo('Verification successful', JSON.stringify(response.data));
    } catch (error) {
        printError('Error during verification', JSON.stringify(error.response.data));
    }
}

async function main(options) {
    await verifyFilfox(options);
}

if (require.main === module) {
    const program = new Command();

    program.name('verify-filfox').description('Verify contracts on filfox explorer for filecoin network');

    addBaseOptions(program, { ignorePrivateKey: true, ignoreChainNames: true, address: true });

    program.addOption(new Option('-c, --contractName <contractName>', 'contract name'));
    program.addOption(new Option('-p, --contractPath <contractPath>', 'flattened contract file path with respect to project root'));
    program.addOption(new Option('--configPath <configPath>', 'hardhat config path with respect to project root'));
    program.addOption(new Option('--compilerVersion <compilerVersion>', 'compiler version used to compile contract'));
    program.addOption(new Option('--optimizeRuns <optimizeRuns>', 'optimize runs used during contract compilation'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
