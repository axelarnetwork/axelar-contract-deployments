const { Command } = require('commander');
const fsStandard = require('fs');
const fs = require('fs').promises;
const path = require('path');
const axios = require('axios');
const FormData = require('form-data');
const JSZip = require('jszip');
const { printInfo, printError, pascalToSnake, printWarn } = require('../common/utils');

const { addBaseOptions } = require('./utils');

const BASE_URL = 'https://api.welldonestudio.io/compiler/sui';
const MOVE_FOLDER_PATH = './sui/move';
const VERIFICATION_FOLDER_PATH = path.join(MOVE_FOLDER_PATH, 'verification');
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

async function copyAndUpdateDependencies(moveFolderPath = MOVE_FOLDER_PATH, contractName = null) {
    try {
        // Validate moveFolderPath exists
        await fs.access(moveFolderPath).catch(() => {
            throw new Error(`Move folder path does not exist: ${moveFolderPath}`);
        });

        await fs.mkdir(VERIFICATION_FOLDER_PATH, { recursive: true });

        const moveContents = await fs.readdir(moveFolderPath, { withFileTypes: true });

        if (contractName && contractName.toLowerCase() !== 'all') {
            // Copy only the specified contract folder
            const pascalCaseContract = pascalToSnake(contractName);
            const sourcePath = path.join(moveFolderPath, pascalCaseContract);
            const destPath = path.join(VERIFICATION_FOLDER_PATH, pascalCaseContract);
            await fs.access(sourcePath).catch(() => {
                throw new Error(`Contract folder does not exist: ${sourcePath}`);
            });
            await fs.cp(sourcePath, destPath, { recursive: true });
            printInfo('Copied to verification folder', `${sourcePath} to ${destPath}`);
        } else {
            // Copy all folders except verification
            for (const item of moveContents) {
                if (item.name !== 'verification') {
                    const sourcePath = path.join(moveFolderPath, item.name);
                    const destPath = path.join(VERIFICATION_FOLDER_PATH, item.name);
                    await fs.cp(sourcePath, destPath, { recursive: true });
                    printInfo('Copied to verification folder', `${sourcePath} to ${destPath}`);
                }
            }
        }

        // Copy all move folder contents to verification/deps
        const centralDepsFolderPath = path.join(VERIFICATION_FOLDER_PATH, 'deps');
        await fs.mkdir(centralDepsFolderPath, { recursive: true });

        for (const item of moveContents) {
            if (item.name !== 'verification') {
                const sourcePath = path.join(moveFolderPath, item.name);
                const destPath = path.join(centralDepsFolderPath, item.name);
                await fs.cp(sourcePath, destPath, { recursive: true });
                printInfo('Copied to central deps', `${sourcePath} to ${destPath}`);
            }
        }

        const verificationContents = await fs.readdir(VERIFICATION_FOLDER_PATH, { withFileTypes: true });

        for (const item of verificationContents) {
            if (item.isDirectory() && item.name !== 'deps' && item.name !== 'verification') {
                const subDirPath = path.join(VERIFICATION_FOLDER_PATH, item.name);
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

                if (item.isDirectory() && item.name !== 'deps' && item.name !== 'verification') {
                    await updateTomlInFolder(itemPath);
                } else if (item.name === 'Move.toml') {
                    try {
                        let tomlContent = await fs.readFile(itemPath, 'utf-8');

                        tomlContent = tomlContent.replace(/local\s*=\s*"\.\.\/([^"]+)"/g, 'local = "./deps/$1"');

                        await fs.writeFile(itemPath, tomlContent);
                        printInfo('Updated Move.toml', itemPath);
                    } catch (error) {
                        printInfo('Skipping Move.toml update due to error', `${itemPath}: ${error.message}`);
                    }
                }
            }
        };

        await updateTomlInFolder(VERIFICATION_FOLDER_PATH);

        printInfo('Dependency update completed');
    } catch (error) {
        printError('Failed to copy and update dependencies', error.message);
    }
}

async function addFolderToZip(folderPath, zipFolder) {
    const contents = await fs.readdir(folderPath, { withFileTypes: true });

    for (const file of contents) {
        const filePath = path.join(folderPath, file.name);

        if (file.isDirectory()) {
            const newFolder = zipFolder.folder(file.name);
            await addFolderToZip(filePath, newFolder);
        } else {
            const fileData = await fs.readFile(filePath);
            zipFolder.file(file.name, fileData);
        }
    }
}

async function zipSubdirectories(moveFolderPath = MOVE_FOLDER_PATH, contractName = null) {
    try {
        // Validate verificationFolderPath exists
        await fs.access(VERIFICATION_FOLDER_PATH).catch(() => {
            throw new Error(`Verification folder path does not exist: ${VERIFICATION_FOLDER_PATH}`);
        });

        const verificationContents = await fs.readdir(VERIFICATION_FOLDER_PATH, { withFileTypes: true });

        if (verificationContents.length === 0) {
            printInfo('No subdirectories found to zip');
            return;
        }

        if (contractName && contractName.toLowerCase() !== 'all') {
            // Zip only the specified contract folder
            const pascalCaseContract = pascalToSnake(contractName);
            const subDirPath = path.join(VERIFICATION_FOLDER_PATH, pascalCaseContract);
            const zipFilePath = path.join(VERIFICATION_FOLDER_PATH, `${pascalCaseContract}.zip`);

            await fs.access(subDirPath).catch(() => {
                throw new Error(`Subdirectory does not exist: ${subDirPath}`);
            });

            const zip = new JSZip();
            await addFolderToZip(subDirPath, zip.folder(pascalCaseContract));

            const zipContent = await zip.generateAsync({
                type: 'nodebuffer',
                compression: 'DEFLATE',
                compressionOptions: { level: 6 },
            });

            await fs.writeFile(zipFilePath, zipContent);
            printInfo('Created ZIP file', zipFilePath);
        } else {
            // Zip all subdirectories
            for (const item of verificationContents) {
                if (item.isDirectory() && item.name !== 'deps' && item.name !== 'verification') {
                    const subDirPath = path.join(VERIFICATION_FOLDER_PATH, item.name);
                    const zipFilePath = path.join(VERIFICATION_FOLDER_PATH, `${item.name}.zip`);

                    await fs.access(subDirPath).catch(() => {
                        throw new Error(`Subdirectory does not exist: ${subDirPath}`);
                    });

                    const zip = new JSZip();
                    await addFolderToZip(subDirPath, zip.folder(item.name));

                    const zipContent = await zip.generateAsync({
                        type: 'nodebuffer',
                        compression: 'DEFLATE',
                        compressionOptions: { level: 6 },
                    });

                    await fs.writeFile(zipFilePath, zipContent);
                    printInfo('Created ZIP file', zipFilePath);
                }
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
        throw new Error(`Failed to check verification status: ${error.message}`);
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
        throw new Error(`Failed to upload source code: ${error.message}`);
    }
}

async function verifyPackage(network, packageId, srcFileId, isSrcUploaded) {
    try {
        const payload = { network, packageId };
        if (!isSrcUploaded && srcFileId) payload.srcFileId = srcFileId;
        const response = await axios.post(`${BASE_URL}/verifications`, payload, { headers: { 'Content-Type': 'application/json' } });
        return response.data;
    } catch (error) {
        throw new Error(`Failed to verify package: ${error.message}`);
    }
}

async function getVerifiedSourceCode(network, packageId) {
    try {
        const response = await axios.get(`${BASE_URL}/verifications/module-sources/${network}/${packageId}`, {
            headers: { accept: 'application/json' },
        });
        return response.data;
    } catch (error) {
        throw new Error(`Failed to fetch verified source code: ${error.message}`);
    }
}

async function processVerification(network, packageId, srcZipPath) {
    printInfo('Checking verification status for package', packageId);
    const status = await checkVerificationStatus(network, packageId);
    if (!status) return;
    printInfo('Verification status', JSON.stringify(status, null, 2));

    let srcFileId = null;

    if (!status.isSrcUploaded && srcZipPath) {
        printInfo('Uploading source code', srcZipPath);
        srcFileId = await uploadSourceCode(network, packageId, srcZipPath);
        printInfo('Source file uploaded with ID', srcFileId);
    } else if (!status.isSrcUploaded) {
        throw new Error('Source code not uploaded via Remix and no source zip provided');
    }

    if (!status.isVerified) {
        printInfo('Verifying package', packageId);
        const verificationResult = await verifyPackage(network, packageId, srcFileId, status.isSrcUploaded);
        printInfo('Verification result', JSON.stringify(verificationResult, null, 2));
        if (!verificationResult.isVerified) {
            throw new Error('Package verification failed');
        }
    } else {
        printInfo('Package already verified');
    }

    printInfo('Fetching verified source code', packageId);
    const sourceCode = await getVerifiedSourceCode(network, packageId);
    printInfo('Verified source code', JSON.stringify(sourceCode, null, 2));
}

async function verifyContracts(contractName, options) {
    printInfo('Starting contract verification for environment', options.env);
    await copyAndUpdateDependencies(MOVE_FOLDER_PATH, contractName);
    await zipSubdirectories(MOVE_FOLDER_PATH, contractName);

    const contractsToVerify = contractName.toLowerCase() === 'all' ? CONTRACTS : [contractName];
    let verificationFailed = false;

    if (!CONTRACTS.includes(contractName) && contractName.toLowerCase() !== 'all') {
        throw new Error(`Invalid contract name: ${contractName}. Must be one of: ${CONTRACTS.join(', ')} or 'all'`);
    }

    for (const contract of contractsToVerify) {
        const pascalCaseContract = pascalToSnake(contract);

        try {
            const address = await getContractAddress(options.env, contract);
            const srcZipPath = path.join(VERIFICATION_FOLDER_PATH, `${pascalCaseContract}.zip`);
            await processVerification(options.env, address, srcZipPath);
            printInfo('Successfully verified', contract);
        } catch (error) {
            printError('Failed to verify', `${contract}: ${error.message}`);
            verificationFailed = true;
        }
    }

    if (verificationFailed) {
        printWarn('Contract verification process completed with some failures');
    } else {
        printInfo('All contracts successfully verified');
    }
    printWarn('!! Please complete `Post-Command Cleanup Steps` outlined in the README before retrying.');
}

if (require.main === module) {
    const program = new Command();
    program
        .name('verify-contract')
        .description('Verify Sui contracts using WELLDONE Studio API.')
        .argument('<contractName>', 'Contract name to verify or "all" to verify all contracts')
        .action((contractName, options) => {
            verifyContracts(contractName, options);
        });

    addBaseOptions(program);

    program.parse();
}
