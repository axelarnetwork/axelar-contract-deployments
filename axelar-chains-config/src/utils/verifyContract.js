const { exec } = require('child_process');
const { writeFile, writeFileSync, existsSync } = require('fs');
const { promisify } = require('util');

const execAsync = promisify(exec);
const writeFileAsync = promisify(writeFile);

/**
 * Verifies a contract on etherscan-like explorer of the provided chain using hardhat.
 * This assumes that the chain has been loaded as a custom network in hardhat.
 *
 * @async
 * @param {string} env
 * @param {string} chain
 * @param {string} contract
 * @param {any[]} args
 * @returns {Promise<void>}
 */
const verifyContract = async (env, chain, contract, args, options = {}) => {
    const stringArgs = args.map((arg) => JSON.stringify(arg));
    const content = `module.exports = [\n    ${stringArgs.join(',\n    ')}\n];`;
    const file = options.dir ? `${options.dir}/temp-arguments.js` : 'temp-arguments.js';

    if (!existsSync(file)) {
        writeFileSync(file, '', 'utf-8');
    }

    const contractArg = options.contractPath ? `--contract ${options.contractPath}` : '';
    const dirPrefix = options.dir ? `cd ${options.dir};` : '';
    const cmd = `${dirPrefix} ENV=${env} npx hardhat verify --network ${chain.toLowerCase()} ${contractArg} --no-compile --constructor-args ${file} ${contract} --show-stack-traces`;

    return writeFileAsync(file, content, 'utf-8')
        .then(() => {
            console.log(`Verifying contract ${contract} with args '${stringArgs.join(',')}'`);
            console.log(cmd);

            return execAsync(cmd, { stdio: 'inherit' });
        })
        .then(() => {
            console.log('Verified!');
        });
};

module.exports = {
    verifyContract,
};
