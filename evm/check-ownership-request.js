require('dotenv').config();

const axios = require('axios');
const { Command, Option } = require('commander');
const csv = require('csv-parser');
const { writeFile, createReadStream } = require('fs');
const { ethers } = require('hardhat');
const { Contract, getDefaultProvider } = ethers;

const { readJSON } = require(`${__dirname}/../axelar-chains-config`);
const keys = readJSON(`${__dirname}/../keys.json`);
const {
    validateParameters,
    getContractJSON,
    loadConfig,
    copyObject,
    printWarn,
    printError,
    getDeploymentTx,
    printInfo,
} = require('./utils');

const interchainTokenABI = getContractJSON('InterchainToken').abi;

async function processCommand(config, options) {
    const { file, startingIndex } = options;

    if (startingIndex) {
        validateParameters({ isValidNumber: { startingIndex } });
    }

    const data = await loadCsvFile(file, startingIndex);
    const finalData = copyObject(data);
    let totalRowsRemoved = 0;

    for (let i = 0; i < data.length; ++i) {
        const row = data[i];
        const tokenAddress = row[0];
        const destinationChainsRaw = row[2].split(',');
        const destinationChains = destinationChainsRaw.map((chain) => chain.trim().toLowerCase()).filter((chain) => chain);
        const dustTx = row[4];

        validateParameters({ isValidAddress: { tokenAddress } });

        const invalidDestinationChains = await verifyChains(config, tokenAddress, destinationChains);
        const validDestinationChains = destinationChains.filter((chain) => !invalidDestinationChains.includes(chain));

        if (validDestinationChains.length > 0) {
            finalData[i - totalRowsRemoved][2] =
                validDestinationChains.length === 1 ? `${validDestinationChains[0]}` : `"${validDestinationChains.join(', ')}"`;
        } else {
            finalData.splice(i - totalRowsRemoved, 1);
            ++totalRowsRemoved;
            continue;
        }

        const chain = validDestinationChains[0];
        const apiUrl = config.chains[chain].explorer.api;
        const apiKey = keys.chains[chain].api;
        let deploymentTx, isValidDustx;

        try {
            deploymentTx = await getDeploymentTx(apiUrl, apiKey, tokenAddress);
            isValidDustx = await verifyDustTx(deploymentTx, dustTx, config.chains);
        } catch {}

        if (!isValidDustx) {
            finalData.splice(i - totalRowsRemoved, 1);
            ++totalRowsRemoved;
        }
    }

    await createCsvFile('pending_ownership_requests.csv', finalData);
}

async function verifyDustTx(deploymentTx, dustTx, chains) {
    const senderDeploymentTx = await getSenderDeploymentTx(deploymentTx);
    const senderDustTx = await getSenderDustTx(dustTx, chains);

    return senderDeploymentTx === senderDustTx;
}

async function getSenderDeploymentTx(deploymentTx) {
    try {
        const response = await axios.get('https://api.axelarscan.io/gmp/searchGMP', {
            params: { txHash: deploymentTx },
            headers: { 'Content-Type': 'application/json' },
        });

        const data = response.data.data[0];
        return data.call.receipt.from.toLowerCase();
    } catch (error) {
        throw new Error('Error fetching sender from deploymentTx: ', error);
    }
}

async function getSenderDustTx(dustTx, chains) {
    if (!dustTx.startsWith('https') && !dustTx.startsWith('0x')) {
        throw new Error('Invalid dustTx format. It must start with "https" or "0x".');
    }

    const txHash = dustTx.startsWith('https') ? dustTx.split('/').pop() : dustTx;

    for (const chainName in chains) {
        const chain = chains[chainName];

        if (chain.id.toLowerCase().includes('axelar')) continue;

        try {
            const provider = getDefaultProvider(chain.rpc);
            const tx = await provider.getTransaction(txHash);
            if (tx) return tx.from.toLowerCase();
        } catch {}
    }

    throw new Error(`Transaction ${dustTx} not found on any chain`);
}

async function verifyChains(config, tokenAddress, destinationChains) {
    const invalidDestinationChains = [];

    for (const chain of destinationChains) {
        try {
            const chainConfig = config.chains[chain];
            const provider = getDefaultProvider(chainConfig.rpc);
            const token = new Contract(tokenAddress, interchainTokenABI, provider);
            const tokenId = await token.interchainTokenId();

            validateParameters({ isValidTokenId: { tokenId } });
        } catch {
            // printWarn(`No Interchain token found for address ${tokenAddress} on chain ${chain}`);
            invalidDestinationChains.push(chain);
        }
    }

    return invalidDestinationChains;
}

async function loadCsvFile(filePath, startingIndex = 0) {
    const results = [];

    try {
        const stream = createReadStream(filePath).pipe(csv());

        for await (const row of stream) {
            results.push(Object.values(row));
        }

        return results.slice(startingIndex);
    } catch (error) {
        throw new Error(`Error loading CSV file: ${error}`);
    }
}

async function createCsvFile(filePath, data) {
    if (!data.length) {
        printWarn('Input data is empty. No CSV file created.');
        return;
    }

    const columnNames = ['Token Address', 'Chains to claim token ownership on', 'Telegram Contact details'];
    const selectedColumns = [0, 2, 3]; // Indexes of required columns

    const filteredData = data.map((row) => {
        return selectedColumns.map((index) => row[index]);
    });

    const csvContent = [columnNames, ...filteredData].map((row) => row.join(',')).join('\n');

    writeFile(filePath, csvContent, { encoding: 'utf8' }, (error) => {
        if (error) {
            printError('Error writing CSV file:', error);
        } else {
            printInfo('Created CSV file at', filePath);
        }
    });
}

async function main(options) {
    const env = 'mainnet';
    const config = loadConfig(env);

    await processCommand(config, options);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('check-ownership-requests')
        .description('Script to check token ownership claim requests')
        .addOption(
            new Option('-f, --file <file>', 'The csv file path containing details about pending token ownership requests')
                .makeOptionMandatory(true)
                .env('FILE'),
        )
        .addOption(
            new Option(
                '-s, --startingIndex <startingIndex>',
                'The starting index from which data will be read. if not provided then whole file will be read',
            ),
        )
        .action(main);

    program.parse();
}
