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
const tokenAddressRowIndex = 1;
const destinationChainsRowIndex = 3;
const contactDetailsRowIndex = 4;
const dustTxRowIndex = 5;
const commentsRowIndex = 6;

async function processCommand(config, options) {
    const { file, startingIndex, yes } = options;

    if (startingIndex) {
        validateParameters({ isValidNumber: { startingIndex } });
    }

    const { columnNames, inputData } = await loadCsvFile(file, startingIndex);
    columnNames.forEach((columnName, index) => {
        columnNames[index] = columnName.replace(/,/g, '');
    });
    const data = cleanInputData(file, columnNames, inputData, yes);
    const finalData = copyObject(data);
    let totalRowsRemoved = 0;

    for (let i = 0; i < data.length; ++i) {
        const row = data[i];
        const tokenAddress = row[tokenAddressRowIndex];

        printInfo(`Verifying data at index ${i + 2} for Token address`, tokenAddress);

        const destinationChainsRaw = row[destinationChainsRowIndex].split(',');
        const destinationChains = destinationChainsRaw.map((chain) => chain.trim().toLowerCase()).filter((chain) => chain);
        const dustTx = row[dustTxRowIndex];

        validateParameters({ isValidAddress: { tokenAddress } });

        const invalidDestinationChains = await verifyChains(config, tokenAddress, destinationChains);
        const validDestinationChains = destinationChains.filter((chain) => !invalidDestinationChains.includes(chain));

        if (validDestinationChains.length > 0) {
            finalData[i - totalRowsRemoved][destinationChainsRowIndex] =
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

function cleanInputData(filePath, columnNames, inputData, skipCommonTokenAddress) {
    const uniqueArrays = [];
    const manualCheckIndices = [];
    const subarrayMap = new Map();

    // Identify and remove duplicates based on subarray values
    inputData.forEach((currentArray, index) => {
        const subarray = currentArray.slice(1);
        const subarrayKey = subarray.join(',');

        if (!subarrayMap.has(subarrayKey)) {
            subarrayMap.set(subarrayKey, index);
            uniqueArrays.push(currentArray);
        } else {
            const existingIndex = subarrayMap.get(subarrayKey);

            if (existingIndex > index) {
                // Remove from uniqueArrays if previously added
                uniqueArrays.splice(
                    uniqueArrays.findIndex((arr) => arr === inputData[existingIndex]),
                    1,
                );
                uniqueArrays.push(currentArray);
                subarrayMap.set(subarrayKey, index);
            }
        }
    });

    // Check for matching values in column 1 across different internal arrays
    const seenValues = new Map();
    uniqueArrays.forEach((arr, index) => {
        const value = arr[tokenAddressRowIndex]; // Check only TokenAddress row
        const originalIndex = inputData.indexOf(arr); // Find the original index

        if (seenValues.has(value)) {
            manualCheckIndices.push([seenValues.get(value), originalIndex]);
        } else {
            seenValues.set(value, originalIndex);
        }
    });

    if (!skipCommonTokenAddress && manualCheckIndices.length !== 0) {
        printError('Manually check the following indexes', manualCheckIndices);
        throw new Error('Input data is not properly cleaned');
    }

    const updatedData = copyObject(uniqueArrays);
    updatedData.forEach((arr) => {
        arr[destinationChainsRowIndex] = `"${arr[destinationChainsRowIndex]}"`;
        arr[commentsRowIndex] = `"${arr[commentsRowIndex]}"`;

        if (!arr[commentsRowIndex]) {
            arr[commentsRowIndex] = 'No Comments';
        }
    });
    updateCSVFile(filePath, columnNames, updatedData);
    return uniqueArrays;
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
            invalidDestinationChains.push(chain);
        }
    }

    return invalidDestinationChains;
}

function updateCSVFile(filePath, columnNames, data) {
    if (!data.length) {
        printWarn('Not updating the csv file', filePath);
        return;
    }

    const csvContent = [columnNames, ...data].map((row) => row.join(',')).join('\n');
    writeCSVData(filePath, csvContent);
}

async function loadCsvFile(filePath, startingIndex = 0) {
    const results = [];
    let columnNames = [];

    try {
        const stream = createReadStream(filePath).pipe(csv());

        for await (const row of stream) {
            if (columnNames.length === 0) {
                columnNames = Object.keys(row);
            }

            results.push(Object.values(row));
        }

        return { columnNames, inputData: results.slice(startingIndex) };
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
    const selectedColumns = [tokenAddressRowIndex, destinationChainsRowIndex, contactDetailsRowIndex]; // Indexes of required columns

    const filteredData = data.map((row) => {
        return selectedColumns.map((index) => row[index]);
    });

    const csvContent = [columnNames, ...filteredData].map((row) => row.join(',')).join('\n');
    writeCSVData(filePath, csvContent);
}

function writeCSVData(filePath, csvContent) {
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
        .addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'))
        .action(main);

    program.parse();
}
