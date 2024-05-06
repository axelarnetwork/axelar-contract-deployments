const { execSync } = require('child_process');
const { writeFileSync } = require('fs');

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
const verifyContract = (env, chain, contract, args, options = {}) => {
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

    try {
        execSync(cmd, { stdio: ['inherit', 'pipe', 'pipe'] });
        console.log('Verified!');
    } catch (error) {
        if (error.message.includes('Reason: Already Verified')) {
            console.log(`Contract ${contract} is already verified on ${chain.toLowerCase()}.`);
        } else {
            throw new Error(`An error occurred while trying to verify ${contract} on ${chain.toLowerCase()}: ${error}`);
        }
    }
};

module.exports = {
    verifyContract,
};
