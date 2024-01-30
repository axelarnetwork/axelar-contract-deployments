'use strict';

const axios = require('axios');
const assert = require('assert');
const fs = require('fs');
const path = require('path');

async function findValue(obj, contractName, result = []) {
    if (obj && typeof obj === 'object') {
        for (const key in obj) {
            if (key.includes(`/${contractName}.sol`) || key === contractName) {
                result.push(obj[key].metadata ? obj[key].metadata : obj[key].content);
            }

            await findValue(obj[key], contractName, result);
        }
    }
}

async function readAndParseFile(filePath, targetKey) {
    try {
        const jsonData = await fs.promises.readFile(filePath, { encoding: 'utf8' });
        const parsedData = JSON.parse(jsonData);
        const result = [];
        await findValue(parsedData, targetKey, result);
        return result;
    } catch (error) {
        console.error(`Error reading or parsing file ${filePath}: ${error.message}`);
        return [];
    }
}

async function processDirectory(directoryPath, targetKey) {
    try {
        const fileNames = await fs.promises.readdir(directoryPath);
        const filePaths = fileNames.map((fileName) => path.join(directoryPath, fileName));

        const results = await Promise.all(filePaths.map((filePath) => readAndParseFile(filePath, targetKey)));

        const uniqueResults = [...new Set(results.flat().filter(Boolean))];
        return uniqueResults;
    } catch (error) {
        console.error(`Error reading directory ${directoryPath}: ${error.message}`);
        return [];
    }
}

function searchContractName(rootFolder, targetValue) {
    const files = fs.readdirSync(rootFolder);

    for (const fileOrFolder of files) {
        const fullPath = path.join(rootFolder, fileOrFolder);

        if (fs.statSync(fullPath).isDirectory()) {
            const contractName = searchContractName(fullPath, targetValue);

            if (contractName) {
                return contractName;
            }
        } else if (fileOrFolder.endsWith('.json')) {
            const jsonData = JSON.parse(fs.readFileSync(fullPath, 'utf-8'));

            if (jsonData.deployedBytecode === targetValue) {
                return jsonData.contractName;
            }
        }
    }
}

async function verify(dir, provider, address, chainId) {
    try {
        const bytecode = await provider.getCode(address);

        if (!dir) {
            throw new Error('Need to specify directory to verify on sourcify via solidity code');
        }

        const artifactsDir = path.join(__dirname, `../../../${dir}/artifacts/contracts`);
        const contractName = await searchContractName(artifactsDir, bytecode);

        if (!contractName) throw new Error('Byte code match not found');

        const directoryPath = artifactsDir.substring(0, artifactsDir.length - 'contracts'.length) + 'build-info';
        const res = await processDirectory(directoryPath, contractName);

        if (res.length !== 2) throw new Error('Unable to find require metadata and solidity code for sourcify verification');
        const [sol, metadata] = res;

        await uploadToSourcify(metadata, sol, address, chainId);
        console.log('Verified on Sourcify via uplaoding solidity code and metadata');
    } catch (error) {
        console.error('Unable to verify on Sourcify by uploading solidity code and metadata: ', error.message);

        try {
            await verifyFromEtherscan(chainId, address);
            console.log('Verified on Sourcify via etherscan');
        } catch {
            console.error('Unable to verify on Sourcify');
        }
    }
}

async function uploadToSourcify(metadata, sol, address, chainId) {
    const headers = {
        'content-type': 'application/json',
    };

    const payloadObj = {
        address,
        chain: chainId,
        files: {
            'metadata.json': metadata,
            'Source.sol': sol,
        },
    };

    const payload = JSON.stringify(payloadObj);
    const apiUrl = 'https://sourcify.dev/server/verify';
    const res = await axios.post(apiUrl, payload, { headers });
    assert(res.data.result[0].status === 'perfect');
    console.log(`Sourcify: contract verified., ${address}`);
}

async function verifyFromEtherscan(chainId, address) {
    const headers = {
        'content-type': 'application/json',
    };
    const payload = {
        address,
        chainId,
    };

    const apiUrl = 'https://sourcify.dev/server/verify/etherscan';
    const res = await axios.post(apiUrl, payload, { headers });

    assert(res.data.result[0].status === 'perfect');
    console.log(`Sourcify: contract verified., ${address}`);
}

module.exports = { verify };
