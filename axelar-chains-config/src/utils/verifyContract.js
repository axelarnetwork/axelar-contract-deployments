const { exec } = require('child_process');
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
const verifyContract = async (env, chain, contract, args, options = {}) => {
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

    await new Promise((resolve, reject) => {
        exec(cmd, { stdio: 'inherit' }, (error, stdout, stderr) => {
            console.log(stdout);

            if (error) {
                if (stderr && stderr.includes('Reason: Already Verified')) {
                    resolve();
                    return;
                }

                console.error(`Error occured while trying to verify ${contract} on ${chain.toLowerCase()}.`);
                reject(error);
                return;
            }

            resolve();
        });
    });

    console.log('Verified');
};

module.exports = {
    verifyContract,
};
