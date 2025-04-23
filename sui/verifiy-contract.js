const fsStandard = require('fs');
const fs = require('fs').promises;
const path = require('path');
const axios = require('axios');
const FormData = require('form-data');
const archiver = require('archiver');
const { Command, Option } = require('commander');
const { addOptionsToCommands, addBaseOptions } = require('./utils');

const BASE_URL = 'https://api.welldonestudio.io/compiler/sui';
const MOVE_FOLDER_PATH = './sui/move';
const CONTRACTS = [
    'Utils',
    'VersionControl',
    'AxelarGateway',
    'GasService',
    'Abi',
    'Operators',
    'Governance',
    'RelayerDiscovery',
    'InterchainTokenService',
    'Squid',
    'Example',
];

function printInfo(message, value = '') {
    console.log(`[INFO] ${message}: ${value}`);
}

function printError(message, value = '') {
    console.error(`[ERROR] ${message}: ${value}`);
    process.exit(1);
}

function toCamelCase(str) {
    return str
        .replace(/(?:^\w|[A-Z]|\b\w)/g, (word, index) => (index === 0 ? word.toLowerCase() : word.toUpperCase()))
        .replace(/\s+/g, '')
        .replace(/([a-z])([A-Z])/g, '$1$2');
}

async function getContractAddress(env, contract) {
    const configPath = path.join(__dirname, '../axelar-chains-config', 'info', `${env}.json`);

    try {
        const configData = await fs.readFile(configPath, 'utf-8');
        const config = JSON.parse(configData);
        const address = config.chains?.sui?.contracts?.[contract]?.address;
        if (!address) throw new Error(`Address for contract ${contract} not found`);
        return address;
    } catch (error) {
        throw new Error(`Failed to read config for ${contract}: ${error.message}`);
    }
}

async function copyAndUpdateDependencies(moveFolderPath = MOVE_FOLDER_PATH) {
    try {
        const centralDepsFolderPath = path.join(moveFolderPath, 'deps');
        await fs.mkdir(centralDepsFolderPath, { recursive: true });
        const moveContents = await fs.readdir(moveFolderPath, { withFileTypes: true });

        for (const item of moveContents) {
            const sourcePath = path.join(moveFolderPath, item.name);
            const destPath = path.join(centralDepsFolderPath, item.name);
            if (item.name === 'deps') continue;
            await fs.cp(sourcePath, destPath, { recursive: true });
            printInfo('Copied to central deps', `${sourcePath} to ${destPath}`);
        }

        for (const item of moveContents) {
            if (item.isDirectory() && item.name !== 'deps') {
                const subDirPath = path.join(moveFolderPath, item.name);
                const subDirDepsFolderPath = path.join(subDirPath, 'deps');
                await fs.mkdir(subDirDepsFolderPath, { recursive: true });
                const centralDepsContents = await fs.readdir(centralDepsFolderPath, { withFileTypes: true });

                for (const depItem of centralDepsContents) {
                    if (depItem.name !== item.name) {
                        const sourcePath = path.join(centralDepsFolderPath, depItem.name);
                        const destPath = path.join(subDirDepsFolderPath, depItem.name);
                        await fs.cp(sourcePath, destPath, { recursive: true });
                        printInfo('Copied to subdirectory deps', `${sourcePath} to ${destPath}`);
                    }
                }
            }
        }

        const updateTomlInFolder = async (folderPath) => {
            const contents = await fs.readdir(folderPath, { withFileTypes: true });

            for (const item of contents) {
                const itemPath = path.join(folderPath, item.name);

                if (item.isDirectory() && item.name !== 'deps') {
                    await updateTomlInFolder(itemPath);
                } else if (item.name === 'Move.toml') {
                    try {
                        let tomlContent = await fs.readFile(itemPath, 'utf-8');
                        tomlContent = tomlContent.replace(/local\s*=\s*"\.\.\/([^"]+)"/g, 'local = "./deps/$1"');
                        await fs.writeFile(itemPath, tomlContent);
                        printInfo('Updated Move.toml', itemPath);
                    } catch (error) {
                        printInfo('Skipping Move.toml update', `${itemPath}: ${error.message}`);
                    }
                }
            }
        };

        await updateTomlInFolder(moveFolderPath);
        printInfo('Dependency update completed');
    } catch (error) {
        printError('Failed to copy and update dependencies', error.message);
    }
}

async function createZipsForSubdirectories(moveFolderPath = MOVE_FOLDER_PATH) {
    try {
        const zipFolderPath = path.join(moveFolderPath, 'zip');
        await fs.mkdir(zipFolderPath, { recursive: true });
        const moveContents = await fs.readdir(moveFolderPath, { withFileTypes: true });

        for (const item of moveContents) {
            if (item.isDirectory() && item.name !== 'deps' && item.name !== 'zip') {
                const subDirPath = path.join(moveFolderPath, item.name);
                const zipFilePath = path.join(zipFolderPath, `${item.name}.zip`);
                const output = fsStandard.createWriteStream(zipFilePath);
                const archive = archiver('zip', { zlib: { level: 9 } });
                archive.pipe(output);
                archive.directory(subDirPath, item.name);
                await archive.finalize();
                printInfo('Created ZIP file', zipFilePath);
            }
        }

        printInfo('ZIP creation completed');
    } catch (error) {
        printError('Failed to create ZIP files', error.message);
    }
}

async function checkVerificationStatus(network, packageId) {
    try {
        const response = await axios.get(`${BASE_URL}/verifications`, { params: { network, packageId } });
        return response.data;
    } catch (error) {
        printError('Failed to check verification status', error.message);
    }
}

async function uploadSourceCode(network, packageId, srcZipPath) {
    try {
        const form = new FormData();
        form.append('network', network);
        form.append('packageId', packageId);
        form.append('srcZipFile', fsStandard.createReadStream(srcZipPath));
        const response = await axios.post(`${BASE_URL}/verifications/sources`, form, { headers: form.getHeaders() });
        return response.data.srcFileId;
    } catch (error) {
        printError('Failed to upload source code', error.message);
    }
}

async function verifyPackage(network, packageId, srcFileId, isRemixSrcUploaded) {
    try {
        const payload = { network, packageId };
        if (!isRemixSrcUploaded && srcFileId) payload.srcFileId = srcFileId;
        const response = await axios.post(`${BASE_URL}/verifications`, payload, { headers: { 'Content-Type': 'application/json' } });
        return response.data;
    } catch (error) {
        printError('Failed to verify package', error.message);
    }
}

async function getVerifiedSourceCode(network, packageId) {
    try {
        const response = await axios.get(`${BASE_URL}/verifications/module-sources/${network}/${packageId}`, {
            headers: { accept: 'application/json' },
        });
        return response.data;
    } catch (error) {
        printError('Failed to fetch verified source code', error.message);
    }
}

async function processVerification(network, packageId, srcZipPath) {
    printInfo('Checking verification status for package', packageId);
    const status = await checkVerificationStatus(network, packageId);
    if (!status) return;
    printInfo('Verification status', JSON.stringify(status, null, 2));

    let srcFileId = null;

    if (!status.isRemixSrcUploaded && srcZipPath) {
        printInfo('Uploading source code', srcZipPath);
        srcFileId = await uploadSourceCode(network, packageId, srcZipPath);
        printInfo('Source file uploaded with ID', srcFileId);
    } else if (!status.isRemixSrcUploaded) {
        printError('Source code not uploaded via Remix and no source zip provided');
    }

    if (!status.isVerified) {
        printInfo('Verifying package', packageId);
        const verificationResult = await verifyPackage(network, packageId, srcFileId, status.isRemixSrcUploaded);
        printInfo('Verification result', JSON.stringify(verificationResult, null, 2));
    } else {
        printInfo('Package already verified');
    }

    printInfo('Fetching verified source code', packageId);
    const sourceCode = await getVerifiedSourceCode(network, packageId);
    printInfo('Verified source code', JSON.stringify(sourceCode, null, 2));
}

async function verifyContracts(env, contractName) {
    console.log(`[INFO] Starting contract verification for environment: ${env}`);
    await copyAndUpdateDependencies();
    await createZipsForSubdirectories();

    const contractsToVerify = contractName.toLowerCase() === 'all' ? CONTRACTS : [contractName];

    if (!CONTRACTS.includes(contractName) && contractName.toLowerCase() !== 'all') {
        printError(`Invalid contract name: ${contractName}. Must be one of: ${CONTRACTS.join(', ')} or 'all'`);
    }

    for (const contract of contractsToVerify) {
        const camelCaseContract = toCamelCase(contract);

        try {
            const address = await getContractAddress(env, contract);
            const srcZipPath = path.join(MOVE_FOLDER_PATH, 'zip', `${camelCaseContract}.zip`);
            await processVerification(env, address, srcZipPath);
            console.log(`[INFO] Successfully verified ${contract}`);
        } catch (error) {
            console.error(`[ERROR] Failed to verify ${contract}: ${error.message}`);
        }
    }

    console.log(`[INFO] Contract verification process completed`);
}

if (require.main === module) {
    const program = new Command();
    addOptionsToCommands(program, addBaseOptions);
    program
        .name('verify-sui-contract')
        .description('Verify Sui Move contracts using WELLDONE Studio API.')
        .addOption(new Option('-c, --contract <contractName>', 'Contract name to verify or "all" for all contracts').default('all'))
        .action(async (options) => {
            try {
                await verifyContracts(options.env, options.contract);
            } catch (error) {
                printError('Verification process failed', error.message);
            }
        });
    program.parse();
}
