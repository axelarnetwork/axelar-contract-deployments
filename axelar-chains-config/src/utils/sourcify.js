'use strict';

const axios = require('axios');
const fs = require('fs');
const path = require('path');
const { readJSON } = require('./readJSON');

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
    const parsedData = readJSON(filePath);
    const result = [];
    await findValue(parsedData, targetKey, result);
    return result;
}

async function processDirectory(directoryPath, targetKey) {
    const fileNames = await fs.promises.readdir(directoryPath);
    const filePaths = fileNames.map((fileName) => path.join(directoryPath, fileName));
    const results = await Promise.all(filePaths.map((filePath) => readAndParseFile(filePath, targetKey)));
    const uniqueResults = [...new Set(results.flat().filter(Boolean))];
    return uniqueResults;
}

function searchContractName(rootFolder, bytecode) {
    const files = fs.readdirSync(rootFolder);

    for (const fileOrFolder of files) {
        const fullPath = path.join(rootFolder, fileOrFolder);

        if (fs.statSync(fullPath).isDirectory()) {
            const contractName = searchContractName(fullPath, bytecode);

            if (contractName) {
                return contractName;
            }
        } else if (fileOrFolder.endsWith('.json')) {
            const jsonData = readJSON(fullPath);

            if (jsonData.deployedBytecode && jsonData.deployedBytecode.startsWith('0x') && jsonData.deployedBytecode === bytecode) {
                return jsonData.contractName;
            }
        }
    }
}

function findProjectRoot(startDir) {
    let currentDir = startDir;

    while (currentDir !== path.parse(currentDir).root) {
        const potentialPackageJson = path.join(currentDir, 'package.sh');

        if (fs.existsSync(potentialPackageJson)) {
            return currentDir;
        }

        currentDir = path.resolve(currentDir, '..');
    }

    throw new Error('Unable to find project root');
}

async function verifyOnSourcify(dir, provider, address, chainId) {
    try {
        const bytecode = await provider.getCode(address);

        if (!bytecode && !bytecode.startsWith('0x')) {
            throw new Error('Unable to fetch valid byte code from chain');
        }

        if (!dir) {
            throw new Error('Need to specify directory to verify on sourcify via solidity code');
        }

        const projectRoot = findProjectRoot(__dirname);
        const artifactsDir = `${projectRoot}/${dir}/artifacts/contracts`;
        const contractName = searchContractName(artifactsDir, bytecode);

        if (!contractName) throw new Error('Byte code match not found');

        const directoryPath = artifactsDir.substring(0, artifactsDir.length - 'contracts'.length) + 'build-info';
        const res = await processDirectory(directoryPath, contractName);

        if (res.length !== 2) throw new Error('Unable to find require metadata and solidity code for sourcify verification');
        const [sol, metadata] = res;

        await uploadToSourcify(metadata, sol, address, chainId);
        console.log('Verified on Sourcify via uploading solidity code and metadata');
    } catch (error) {
        console.error('Unable to verify on Sourcify by uploading solidity code and metadata: ', error.message);

        await verifyFromEtherscan(chainId, address);
        console.log('Verified on Sourcify via etherscan');
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

    switch (res.data.result[0].status) {
        case 'perfect':
            console.log(`Sourcify: contract verified perfectly., ${address}`);
            break;
        case 'partial':
            console.log(`Sourcify: contract verified partially., ${address}`);
            throw new Error('Partially verified using metadata!');
        default:
            throw new Error('Sourcify verification failed');
    }
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

    switch (res.data.result[0].status) {
        case 'perfect':
            console.log(`Sourcify: contract verified perfectly., ${address}`);
            break;
        case 'partial':
            console.log(`Sourcify: contract verified partially., ${address}`);
            break;
        default:
            throw new Error('Sourcify verification failed');
    }
}

module.exports = { verifyOnSourcify };
