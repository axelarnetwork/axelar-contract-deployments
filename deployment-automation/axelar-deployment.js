"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
Object.defineProperty(exports, "__esModule", { value: true });
const fs = __importStar(require("fs"));
const path = __importStar(require("path"));
const readline = __importStar(require("readline"));
const child_process_1 = require("child_process");
const util = __importStar(require("util"));
// Constants
const BASE_URL = 'https://static.axelar.network/releases/cosmwasm';
const WASM_DIR = '../wasm';
// Promisified version of exec
const execAsync = util.promisify(child_process_1.exec);
// Create an interface for readline
const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
});
// Promisified version of readline question
const question = (query) => {
    return new Promise((resolve) => {
        rl.question(query, resolve);
    });
};
// Initialize config object
const config = {
    NAMESPACE: '',
};
/**
 * Function to get the network name from user input
 */
function getNetworkName() {
    return __awaiter(this, void 0, void 0, function* () {
        console.log("Select a network:");
        console.log("1) mainnet");
        console.log("2) testnet");
        console.log("3) stagenet");
        console.log("4) devnet-amplifier");
        console.log("5) Custom devnet (e.g., devnet-user)");
        while (true) {
            const choice = yield question("Enter your choice (1-5): ");
            switch (choice) {
                case '1':
                    config.NAMESPACE = "mainnet";
                    return;
                case '2':
                    config.NAMESPACE = "testnet";
                    return;
                case '3':
                    config.NAMESPACE = "stagenet";
                    return;
                case '4':
                    config.NAMESPACE = "devnet-amplifier";
                    return;
                case '5':
                    const customName = yield question("Enter your custom devnet name (e.g., devnet-user): ");
                    config.NAMESPACE = customName;
                    return;
                default:
                    console.log("❌ Invalid choice. Please select 1, 2, 3, 4 or 5.");
                    break;
            }
        }
    });
}
/**
 * Function to check if the network is a custom devnet
 */
function isCustomDevnet() {
    if (config.NAMESPACE === "mainnet" ||
        config.NAMESPACE === "testnet" ||
        config.NAMESPACE === "stagenet" ||
        config.NAMESPACE === "devnet-amplifier") {
        return false; // Not a custom devnet
    }
    else {
        return true; // Is a custom devnet
    }
}
/**
 * Function to set predefined values for known networks
 */
function setPredefinedValues() {
    switch (config.NAMESPACE) {
        case "mainnet":
            config.GOVERNANCE_ADDRESS = "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj";
            config.ADMIN_ADDRESS = "axelar1pczf792wf3p3xssk4dmwfxrh6hcqnrjp70danj";
            config.SERVICE_NAME = "amplifier";
            config.VOTING_THRESHOLD = '["2", "3"]';
            config.SIGNING_THRESHOLD = '["2", "3"]';
            config.CONFIRMATION_HEIGHT = "1";
            config.MINIMUM_ROTATION_DELAY = "86400";
            config.DEPLOYMENT_TYPE = "create";
            config.DEPLOYER = "0xB8Cd93C83A974649D76B1c19f311f639e62272BC";
            config.CONTRACT_ADMIN = "axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am";
            config.PROVER_ADMIN = "axelar1pczf792wf3p3xssk4dmwfxrh6hcqnrjp70danj";
            config.DEPOSIT_VALUE = "2000000000";
            config.REWARD_AMOUNT = "1000000uaxl";
            break;
        case "testnet":
            config.GOVERNANCE_ADDRESS = "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj";
            config.ADMIN_ADDRESS = "axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35";
            config.SERVICE_NAME = "amplifier";
            config.VOTING_THRESHOLD = '["51", "100"]';
            config.SIGNING_THRESHOLD = '["51", "100"]';
            config.CONFIRMATION_HEIGHT = "1";
            config.MINIMUM_ROTATION_DELAY = "3600";
            config.DEPLOYMENT_TYPE = "create";
            config.DEPLOYER = "0xba76c6980428A0b10CFC5d8ccb61949677A61233";
            config.CONTRACT_ADMIN = "axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7";
            config.PROVER_ADMIN = "axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35";
            config.DEPOSIT_VALUE = "2000000000";
            config.REWARD_AMOUNT = "1000000uaxl";
            break;
        case "stagenet":
            config.GOVERNANCE_ADDRESS = "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj";
            config.ADMIN_ADDRESS = "axelar1l7vz4m5g92kvga050vk9ycjynywdlk4zhs07dv";
            config.SERVICE_NAME = "amplifier";
            config.VOTING_THRESHOLD = '["51", "100"]';
            config.SIGNING_THRESHOLD = '["51", "100"]';
            config.CONFIRMATION_HEIGHT = "1";
            config.MINIMUM_ROTATION_DELAY = "300";
            config.DEPLOYMENT_TYPE = "create3";
            config.DEPLOYER = "0xba76c6980428A0b10CFC5d8ccb61949677A61233";
            config.CONTRACT_ADMIN = "axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky";
            config.PROVER_ADMIN = "axelar1l7vz4m5g92kvga050vk9ycjynywdlk4zhs07dv";
            config.DEPOSIT_VALUE = "100000000";
            config.REWARD_AMOUNT = "1000000uaxl";
            break;
        case "devnet-amplifier":
            config.GOVERNANCE_ADDRESS = "axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9";
            config.ADMIN_ADDRESS = "axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9";
            config.SERVICE_NAME = "validators";
            config.VOTING_THRESHOLD = '["6", "10"]';
            config.SIGNING_THRESHOLD = '["6", "10"]';
            config.CONFIRMATION_HEIGHT = "1";
            config.MINIMUM_ROTATION_DELAY = "0";
            config.DEPLOYMENT_TYPE = "create3";
            config.DEPLOYER = "0xba76c6980428A0b10CFC5d8ccb61949677A61233";
            config.CONTRACT_ADMIN = "axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9";
            config.PROVER_ADMIN = "axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9";
            config.DEPOSIT_VALUE = "100000000";
            config.REWARD_AMOUNT = "1000000uamplifier";
            break;
    }
}
/**
 * Function to validate private key
 */
function validatePrivateKey() {
    return __awaiter(this, void 0, void 0, function* () {
        while (true) {
            const privateKey = yield question("Enter Private Key (must start with 0x): ");
            if (/^0x[0-9a-fA-F]+$/.test(privateKey)) {
                config.TARGET_CHAIN_PRIVATE_KEY = privateKey;
                return;
            }
            else {
                console.log("❌ Invalid private key format. Make sure it starts with '0x' and contains only hexadecimal characters (0-9, a-f).");
            }
        }
    });
}
/**
 * Function to validate RPC URL
 */
function validateRpcUrl() {
    return __awaiter(this, void 0, void 0, function* () {
        while (true) {
            const rpcUrl = yield question("Enter RPC URL (must start with http:// or https://): ");
            if (/^https?:\/\//.test(rpcUrl)) {
                config.RPC_URL = rpcUrl;
                return;
            }
            else {
                console.log("❌ Invalid RPC URL format. It must start with 'http://' or 'https://'.");
            }
        }
    });
}
/**
 * Function to validate Axelar RPC Node URL
 */
function validateAxelarRpcUrl() {
    return __awaiter(this, void 0, void 0, function* () {
        while (true) {
            const axelarRpcUrl = yield question("Enter Axelar RPC Node URL (must start with http:// or https://): ");
            if (/^https?:\/\//.test(axelarRpcUrl)) {
                config.AXELAR_RPC_URL = axelarRpcUrl;
                return;
            }
            else {
                console.log("❌ Invalid Axelar RPC Node URL format. It must start with 'http://' or 'https://'.");
            }
        }
    });
}
/**
 * Function to use an existing wallet or create a new one if needed
 */
function setupWallet() {
    return __awaiter(this, void 0, void 0, function* () {
        const walletName = "amplifier";
        console.log(`⚡ Setting up wallet '${walletName}'...`);
        try {
            // First check if the wallet already exists
            try {
                const walletAddress = (0, child_process_1.execSync)(`axelard keys show "${walletName}" --keyring-backend test -a`, { stdio: 'pipe' }).toString().trim();
                if (walletAddress) {
                    console.log(`✅ Using existing wallet '${walletName}': ${walletAddress}`);
                    config.WALLET_ADDRESS = walletAddress;
                    return; // Exit function early since we're using an existing wallet
                }
            }
            catch (error) {
                // Wallet doesn't exist, we'll create it below
                console.log(`Wallet '${walletName}' not found, will create it...`);
            }
            // Only reach here if wallet doesn't exist
            // Clean up the mnemonic - remove any quotes
            const cleanMnemonic = config.MNEMONIC.replace(/^["'](.*)["']$/, '$1');
            // Create the wallet using spawn
            const axelardProcess = (0, child_process_1.spawn)('axelard', [
                'keys',
                'add',
                walletName,
                '--keyring-backend',
                'test',
                '--recover'
            ], {
                stdio: ['pipe', 'inherit', 'inherit']
            });
            // Send the mnemonic
            axelardProcess.stdin.write(cleanMnemonic + '\n');
            axelardProcess.stdin.end();
            // Wait for process completion
            yield new Promise((resolve, reject) => {
                axelardProcess.on('close', (code) => {
                    if (code === 0) {
                        resolve();
                    }
                    else {
                        reject(new Error(`Command failed with exit code ${code}`));
                    }
                });
            });
            // Verify wallet creation
            const walletAddress = (0, child_process_1.execSync)(`axelard keys show "${walletName}" --keyring-backend test -a`, { stdio: 'pipe' }).toString().trim();
            if (walletAddress) {
                console.log(`✅ Wallet successfully created! Address: ${walletAddress}`);
                config.WALLET_ADDRESS = walletAddress;
            }
            else {
                console.log("❌ Failed to create wallet!");
                process.exit(1);
            }
        }
        catch (error) {
            console.error(`Error setting up wallet: ${error}`);
            process.exit(1);
        }
    });
}
/**
 * Function to extract the Predicted Gateway Proxy Address
 */
function extractProxyGatewayAddress(output) {
    const match = output.match(/Predicted gateway proxy address: (0x[a-fA-F0-9]+)/);
    if (match && match[1]) {
        config.PROXY_GATEWAY_ADDRESS = match[1];
        console.log(`✅ Extracted and set PROXY_GATEWAY_ADDRESS: ${config.PROXY_GATEWAY_ADDRESS}`);
    }
    else {
        console.log("❌ Could not extract Predicted Gateway Proxy Address!");
        process.exit(1);
    }
}
/**
 * Function to generate the JSON config file
 */
function generateJsonConfig() {
    const jsonContent = {
        [config.CHAIN_NAME]: {
            name: config.CHAIN_NAME,
            id: config.CHAIN_ID,
            axelarId: config.CHAIN_NAME,
            chainId: parseInt(config.CHAIN_ID),
            rpc: config.RPC_URL,
            tokenSymbol: config.TOKEN_SYMBOL,
            confirmations: 1,
            gasOptions: {
                gasLimit: parseInt(config.GAS_LIMIT)
            }
        }
    };
    fs.writeFileSync("./config.json", JSON.stringify(jsonContent, null, 4));
    console.log("✅ Configuration saved to ./config.json");
}
/**
 * Function to insert the generated JSON into the network config file
 */
function insertIntoNetworkConfig() {
    const networkJsonPath = `../axelar-chains-config/info/${config.NAMESPACE}.json`;
    // Check if the network JSON file exists
    if (!fs.existsSync(networkJsonPath)) {
        console.log(`❌ Network JSON file not found: ${networkJsonPath}`);
        process.exit(1);
    }
    // Read the JSON file
    const existingJson = JSON.parse(fs.readFileSync(networkJsonPath, 'utf8'));
    // Check if "chains" exists in the JSON
    if (!existingJson.chains) {
        console.log(`❌ No 'chains' dictionary found in ${networkJsonPath}`);
        process.exit(1);
    }
    // Check if CHAIN_NAME already exists in "chains"
    if (existingJson.chains[config.CHAIN_NAME]) {
        console.log(`❌ Chain '${config.CHAIN_NAME}' already exists in ${networkJsonPath}! Aborting to prevent overwriting.`);
        process.exit(1);
    }
    // Insert the new chain object into "chains"
    const newChain = JSON.parse(fs.readFileSync('./config.json', 'utf8'));
    existingJson.chains = Object.assign(Object.assign({}, existingJson.chains), newChain);
    // Write back the updated JSON
    fs.writeFileSync(networkJsonPath, JSON.stringify(existingJson, null, 2));
    console.log(`✅ Successfully added '${config.CHAIN_NAME}' to ${networkJsonPath}`);
}
/**
 * Function to update VotingVerifier inside axelar.contracts in JSON config
 */
function updateVotingVerifierConfig() {
    var _a, _b;
    const networkJsonPath = `../axelar-chains-config/info/${config.NAMESPACE}.json`;
    // Ensure the JSON file exists
    if (!fs.existsSync(networkJsonPath)) {
        console.log(`❌ Network JSON file not found: ${networkJsonPath}`);
        process.exit(1);
    }
    // Read the existing JSON file
    const existingJson = JSON.parse(fs.readFileSync(networkJsonPath, 'utf8'));
    // Check if "axelar.contracts.VotingVerifier" exists in the JSON
    if (!((_b = (_a = existingJson.axelar) === null || _a === void 0 ? void 0 : _a.contracts) === null || _b === void 0 ? void 0 : _b.VotingVerifier)) {
        console.log(`❌ No 'VotingVerifier' section found inside axelar.contracts in ${networkJsonPath}!`);
        process.exit(1);
    }
    // Check if CHAIN_NAME already exists in VotingVerifier
    if (existingJson.axelar.contracts.VotingVerifier[config.CHAIN_NAME]) {
        console.log(`❌ Chain '${config.CHAIN_NAME}' already exists in VotingVerifier! Aborting to prevent overwriting.`);
        process.exit(1);
    }
    // Create the new chain entry
    const newChainEntry = {
        governanceAddress: config.GOVERNANCE_ADDRESS,
        serviceName: config.SERVICE_NAME,
        sourceGatewayAddress: config.PROXY_GATEWAY_ADDRESS,
        votingThreshold: JSON.parse(config.VOTING_THRESHOLD),
        blockExpiry: 10,
        confirmationHeight: parseInt(config.CONFIRMATION_HEIGHT),
        msgIdFormat: "hex_tx_hash_and_event_index",
        addressFormat: "eip55"
    };
    // Insert the new chain entry into axelar.contracts.VotingVerifier
    existingJson.axelar.contracts.VotingVerifier[config.CHAIN_NAME] = newChainEntry;
    // Write the updated JSON back to file
    fs.writeFileSync(networkJsonPath, JSON.stringify(existingJson, null, 2));
    console.log(`✅ Successfully added '${config.CHAIN_NAME}' to VotingVerifier inside axelar.contracts in ${networkJsonPath}`);
}
/**
 * Function to update the namespace JSON file with MultisigProver contract
 */
function updateMultisigProver() {
    var _a, _b;
    const namespaceJsonPath = `../axelar-chains-config/info/${config.NAMESPACE}.json`;
    // Check if the namespace JSON file exists
    if (!fs.existsSync(namespaceJsonPath)) {
        console.log(`❌ Namespace JSON file not found: ${namespaceJsonPath}`);
        process.exit(1);
    }
    // Read the existing JSON file
    const existingJson = JSON.parse(fs.readFileSync(namespaceJsonPath, 'utf8'));
    // Check if "axelar.contracts.MultisigProver" exists in the JSON
    if (!((_b = (_a = existingJson.axelar) === null || _a === void 0 ? void 0 : _a.contracts) === null || _b === void 0 ? void 0 : _b.MultisigProver)) {
        console.log(`❌ No 'MultisigProver' dictionary found in ${namespaceJsonPath}`);
        process.exit(1);
    }
    // Check if CHAIN_NAME already exists in "MultisigProver"
    if (existingJson.axelar.contracts.MultisigProver[config.CHAIN_NAME]) {
        console.log(`❌ Chain '${config.CHAIN_NAME}' already exists under 'MultisigProver' in ${namespaceJsonPath}! Aborting to prevent overwriting.`);
        process.exit(1);
    }
    // Create the new chain entry with updated environment variables
    const newMultisigProverEntry = {
        governanceAddress: config.GOVERNANCE_ADDRESS,
        adminAddress: config.ADMIN_ADDRESS,
        destinationChainID: config.CHAIN_ID,
        signingThreshold: JSON.parse(config.SIGNING_THRESHOLD),
        serviceName: config.SERVICE_NAME,
        verifierSetDiffThreshold: 0,
        encoder: "abi",
        keyType: "ecdsa"
    };
    // Insert the new chain entry into "MultisigProver"
    existingJson.axelar.contracts.MultisigProver[config.CHAIN_NAME] = newMultisigProverEntry;
    // Write back the updated JSON
    fs.writeFileSync(namespaceJsonPath, JSON.stringify(existingJson, null, 2));
    console.log(`✅ Successfully added '${config.CHAIN_NAME}' under 'MultisigProver' in ${namespaceJsonPath}`);
    // Confirm the new entry was added
    console.log("🔍 Verifying the new MultisigProver entry...");
    console.log(JSON.stringify(existingJson.axelar.contracts.MultisigProver, null, 2));
}
/**
 * Function to extract SALT value from the correct checksums file
 */
function extractSalt(contractName) {
    const checkSumFile = `${WASM_DIR}/${contractName}_checksums.txt`;
    if (!fs.existsSync(checkSumFile)) {
        console.log(`❌ Checksum file not found: ${checkSumFile}`);
        process.exit(1);
    }
    // Extract the correct checksum (SALT) for the contract
    const fileContent = fs.readFileSync(checkSumFile, 'utf8');
    const match = fileContent.match(new RegExp(`(\\S+)\\s+${contractName}\\.wasm`));
    if (!match || !match[1]) {
        console.log(`❌ Failed to extract SALT for ${contractName}!`);
        process.exit(1);
    }
    config.SALT = match[1];
    console.log(`✅ Extracted SALT: ${config.SALT}`);
}
/**
 * Extract ROUTER_ADDRESS from the namespace JSON file
 */
function extractRouterAddress() {
    var _a, _b, _c;
    const routerFile = `../axelar-chains-config/info/${config.NAMESPACE}.json`;
    if (!fs.existsSync(routerFile)) {
        console.log(`❌ Router config file not found: ${routerFile}`);
        process.exit(1);
    }
    const jsonContent = JSON.parse(fs.readFileSync(routerFile, 'utf8'));
    const routerAddress = (_c = (_b = (_a = jsonContent === null || jsonContent === void 0 ? void 0 : jsonContent.axelar) === null || _a === void 0 ? void 0 : _a.contracts) === null || _b === void 0 ? void 0 : _b.Router) === null || _c === void 0 ? void 0 : _c.address;
    if (!routerAddress) {
        console.log("❌ Could not extract ROUTER_ADDRESS!");
        process.exit(1);
    }
    config.ROUTER_ADDRESS = routerAddress;
    console.log(`✅ Extracted ROUTER_ADDRESS: ${config.ROUTER_ADDRESS}`);
}
/**
 * Extract GATEWAY_ADDRESS for the specified chain
 */
function extractGatewayAddress() {
    var _a, _b, _c, _d;
    const gatewayFile = `../axelar-chains-config/info/${config.NAMESPACE}.json`;
    if (!fs.existsSync(gatewayFile)) {
        console.log(`❌ Gateway config file not found: ${gatewayFile}`);
        process.exit(1);
    }
    const jsonContent = JSON.parse(fs.readFileSync(gatewayFile, 'utf8'));
    const gatewayAddress = (_d = (_c = (_b = (_a = jsonContent === null || jsonContent === void 0 ? void 0 : jsonContent.axelar) === null || _a === void 0 ? void 0 : _a.contracts) === null || _b === void 0 ? void 0 : _b.Gateway) === null || _c === void 0 ? void 0 : _c[config.CHAIN_NAME]) === null || _d === void 0 ? void 0 : _d.address;
    if (!gatewayAddress) {
        console.log(`❌ Could not extract GATEWAY_ADDRESS for ${config.CHAIN_NAME}!`);
        process.exit(1);
    }
    config.GATEWAY_ADDRESS = gatewayAddress;
    console.log(`✅ Extracted GATEWAY_ADDRESS: ${config.GATEWAY_ADDRESS}`);
}
/**
 * Function to build JSON command for chain registration
 */
function buildJsonCmdRegister() {
    const jsonCmdRegister = JSON.stringify({
        register_chain: {
            chain: config.CHAIN_NAME,
            gateway_address: config.GATEWAY_ADDRESS,
            msg_id_format: "hex_tx_hash_and_event_index"
        }
    });
    console.log(`✅ Built JSON_CMD_REGISTER: ${jsonCmdRegister}`);
    return jsonCmdRegister;
}
/**
 * Function to verify the transaction execution
 */
function verifyExecution() {
    return __awaiter(this, void 0, void 0, function* () {
        console.log("⚡ Verifying the transaction execution...");
        const jsonQuery = JSON.stringify({ chain_info: config.CHAIN_NAME });
        try {
            const { stdout } = yield execAsync(`axelard q wasm contract-state smart "${config.ROUTER_ADDRESS}" '${jsonQuery}' --node "${config.AXELAR_RPC_URL}"`);
            // Print raw output for debugging
            console.log("🔍 Verification Output:");
            console.log(stdout);
            // Extract Gateway Address - updated regex to match the actual output format
            // Looking for the "address:" field under the "gateway:" section
            const gatewayMatch = stdout.match(/gateway:[\s\S]*?address:\s+(\S+)/m);
            const verifiedGatewayAddress = gatewayMatch ? gatewayMatch[1] : null;
            // Ensure the gateway address matches expected value
            if (verifiedGatewayAddress && verifiedGatewayAddress === config.GATEWAY_ADDRESS) {
                console.log(`✅ Verification successful! Gateway address matches: ${verifiedGatewayAddress}`);
            }
            else {
                console.log(`❌ Verification failed! Expected: ${config.GATEWAY_ADDRESS}, Got: ${verifiedGatewayAddress || "address not found"}`);
                process.exit(1);
            }
        }
        catch (error) {
            console.error(`Error during verification: ${error}`);
            process.exit(1);
        }
    });
}
/**
 * Function to verify multisig
 */
function verifyMultisig() {
    return __awaiter(this, void 0, void 0, function* () {
        console.log("⚡ Verifying the transaction execution for MultisigProver...");
        const jsonQuery = JSON.stringify({
            is_caller_authorized: {
                contract_address: config.MULTISIG_PROVER_ADDRESS,
                chain_name: config.CHAIN_NAME
            }
        });
        try {
            const { stdout } = yield execAsync(`axelard q wasm contract-state smart "${config.MULTISIG_ADDRESS}" '${jsonQuery}' --node "${config.AXELAR_RPC_URL}"`);
            // Print raw output for debugging
            console.log("🔍 Verification Output:");
            console.log(stdout);
            // Check if the output contains "data: true" as plain text
            if (stdout.includes("data: true")) {
                console.log("✅ Verification successful! MultisigProver is authorized.");
            }
            else {
                console.log("❌ Verification failed! Expected 'data: true' but got:");
                console.log(stdout);
                process.exit(1);
            }
        }
        catch (error) {
            console.error(`Error during multisig verification: ${error}`);
            process.exit(1);
        }
    });
}
/**
 * Function to create reward pools
 */
function createRewardPools() {
    return __awaiter(this, void 0, void 0, function* () {
        console.log("⚡ Creating reward pools");
        if (isCustomDevnet()) {
            const params = JSON.stringify({
                epoch_duration: "10",
                rewards_per_epoch: "100",
                participation_threshold: ["9", "10"]
            });
            const jsonCreatePoolMultisig = JSON.stringify({
                create_pool: {
                    pool_id: {
                        chain_name: config.CHAIN_NAME,
                        contract: config.MULTISIG_ADDRESS
                    },
                    params: JSON.parse(params)
                }
            });
            const jsonCreatePoolVerifier = JSON.stringify({
                create_pool: {
                    pool_id: {
                        chain_name: config.CHAIN_NAME,
                        contract: config.VOTING_VERIFIER_ADDRESS
                    },
                    params: JSON.parse(params)
                }
            });
            try {
                yield execAsync(`axelard tx wasm execute "${config.REWARDS_ADDRESS}" '${jsonCreatePoolMultisig}' \
        --from amplifier \
        --gas auto \
        --gas-adjustment 2 \
        --node "${config.AXELAR_RPC_URL}" \
        --gas-prices 0.00005${config.TOKEN_DENOM} \
        --keyring-backend test \
        --chain-id "${config.NAMESPACE}"`);
                yield execAsync(`axelard tx wasm execute "${config.REWARDS_ADDRESS}" '${jsonCreatePoolVerifier}' \
        --from amplifier \
        --gas auto \
        --gas-adjustment 2 \
        --node "${config.AXELAR_RPC_URL}" \
        --gas-prices 0.00005${config.TOKEN_DENOM} \
        --keyring-backend test \
        --chain-id "${config.NAMESPACE}"`);
            }
            catch (error) {
                console.error(`Error creating reward pools: ${error}`);
                process.exit(1);
            }
        }
        else {
            // Logic for submitting proposals through the NodeJS script
            if (config.NAMESPACE === "devnet-amplifier") {
                try {
                    yield execAsync(`node ../cosmwasm/submit-proposal.js execute \
          -c Rewards \
          -t "Create pool for ${config.CHAIN_NAME} in ${config.CHAIN_NAME} voting verifier" \
          -d "Create pool for ${config.CHAIN_NAME} in ${config.CHAIN_NAME} voting verifier" \
          --runAs ${config.RUN_AS_ACCOUNT} \
          --deposit ${config.DEPOSIT_VALUE} \
          --msg "{ \\"create_pool\\": { \\"params\\": { \\"epoch_duration\\": \\"${config.EPOCH_DURATION}\\", \\"participation_threshold\\": [\\"7\\", \\"10\\"], \\"rewards_per_epoch\\": \\"100\\" }, \\"pool_id\\": { \\"chain_name\\": \\"${config.CHAIN_NAME}\\", \\"contract\\": \\"${config.VOTING_VERIFIER_ADDRESS}\\" } } }"`);
                }
                catch (error) {
                    console.error(`Error creating reward pools via proposal (devnet-amplifier): ${error}`);
                    process.exit(1);
                }
            }
            else {
                try {
                    yield execAsync(`node ../cosmwasm/submit-proposal.js execute \
          -c Rewards \
          -t "Create pool for ${config.CHAIN_NAME} in ${config.CHAIN_NAME} voting verifier" \
          -d "Create pool for ${config.CHAIN_NAME} in ${config.CHAIN_NAME} voting verifier" \
          --deposit ${config.DEPOSIT_VALUE} \
          --msg "{ \\"create_pool\\": { \\"params\\": { \\"epoch_duration\\": \\"${config.EPOCH_DURATION}\\", \\"participation_threshold\\": [\\"7\\", \\"10\\"], \\"rewards_per_epoch\\": \\"100\\" }, \\"pool_id\\": { \\"chain_name\\": \\"${config.CHAIN_NAME}\\", \\"contract\\": \\"${config.VOTING_VERIFIER_ADDRESS}\\" } } }"`);
                }
                catch (error) {
                    console.error(`Error creating reward pools via proposal: ${error}`);
                    process.exit(1);
                }
            }
        }
    });
}
/**
 * Function to add funds to reward pools
 */
function addFundsToRewardPools() {
    return __awaiter(this, void 0, void 0, function* () {
        if (!isCustomDevnet()) {
            console.log("⚡ Adding funds to reward pools...");
            try {
                const rewards = (0, child_process_1.execSync)(`cat ../axelar-chains-config/info/${config.NAMESPACE}.json | jq .axelar.contracts.Rewards.address | tr -d '"'`, { stdio: 'pipe' }).toString().trim();
                yield execAsync(`axelard tx wasm execute ${rewards} "{ \\"add_rewards\\": { \\"pool_id\\": { \\"chain_name\\": \\"${config.CHAIN_NAME}\\", \\"contract\\": \\"${config.MULTISIG_ADDRESS}\\" } } }" --amount ${config.REWARD_AMOUNT} --from ${config.WALLET_ADDRESS}`);
                yield execAsync(`axelard tx wasm execute ${rewards} "{ \\"add_rewards\\": { \\"pool_id\\": { \\"chain_name\\": \\"${config.CHAIN_NAME}\\", \\"contract\\": \\"${config.VOTING_VERIFIER_ADDRESS}\\" } } }" --amount ${config.REWARD_AMOUNT} --from ${config.WALLET_ADDRESS}`);
            }
            catch (error) {
                console.error(`Error adding funds to reward pools: ${error}`);
                process.exit(1);
            }
        }
    });
}
/**
 * Function to create genesis verifier set
 */
function createGenesisVerifierSet() {
    return __awaiter(this, void 0, void 0, function* () {
        try {
            yield execAsync(`axelard tx wasm execute ${config.MULTISIG_PROVER_ADDRESS} '"update_verifier_set"' \
      --from ${config.PROVER_ADMIN} \
      --gas auto \
      --gas-adjustment 2 \
      --node "${config.AXELAR_RPC_URL}" \
      --gas-prices 0.00005${config.TOKEN_DENOM} \
      --keyring-backend test \
      --chain-id "${config.NAMESPACE}"`);
            console.log("🔍 Querying multisig prover for active verifier set...");
            const { stdout } = yield execAsync(`axelard q wasm contract-state smart ${config.MULTISIG_PROVER_ADDRESS} "\"current_verifier_set\"" \
      --node "${config.AXELAR_RPC_URL}" \
      --chain-id "${config.NAMESPACE}"`);
            console.log(stdout);
        }
        catch (error) {
            console.error(`Error creating genesis verifier set: ${error}`);
            process.exit(1);
        }
    });
}
/**
 * Function to deploy gateway contract
 */
function deployGatewayContract() {
    return __awaiter(this, void 0, void 0, function* () {
        try {
            const setupOutput = (0, child_process_1.execSync)(`node ../evm/deploy-amplifier-gateway.js --env "${config.NAMESPACE}" -n "${config.CHAIN_NAME}" -m "${config.DEPLOYMENT_TYPE}" --minimumRotationDelay "${config.MINIMUM_ROTATION_DELAY}" -p "${config.TARGET_CHAIN_PRIVATE_KEY}"`, { stdio: 'pipe' }).toString();
            // Print output for debugging
            console.log(setupOutput);
            return setupOutput;
        }
        catch (error) {
            console.error(`Error deploying gateway contract: ${error}`);
            process.exit(1);
            return "";
        }
    });
}
/**
 * Function to determine the token denomination
 */
function getTokenDenomination() {
    return __awaiter(this, void 0, void 0, function* () {
        console.log("⚡ Querying wallet balance to determine token denomination...");
        try {
            const balanceOutput = (0, child_process_1.execSync)(`axelard q bank balances "${config.WALLET_ADDRESS}" --node "${config.AXELAR_RPC_URL}"`, { stdio: 'pipe' }).toString();
            console.log("🔍 Wallet Balance Output:");
            console.log(balanceOutput);
            // Extract the first token denomination found
            const tokenDenomMatch = balanceOutput.match(/denom:\s+(\S+)/);
            const tokenDenom = tokenDenomMatch ? tokenDenomMatch[1] : null;
            if (!tokenDenom) {
                console.log("❌ Could not determine token denomination! Check if wallet has funds.");
                process.exit(1);
            }
            config.TOKEN_DENOM = tokenDenom;
            console.log(`✅ Retrieved token denomination: ${tokenDenom}`);
        }
        catch (error) {
            console.error(`Error determining token denomination: ${error}`);
            process.exit(1);
        }
    });
}
/**
 * Function to print environment variables as JSON and exit
 */
function printEnvJsonAndExit() {
    console.log("🎉 Chain registration complete! Need to Update the Verifiers!");
    const configKeys = [
        'NAMESPACE', 'CHAIN_NAME', 'CHAIN_ID', 'TOKEN_SYMBOL', 'GAS_LIMIT',
        'TARGET_CHAIN_PRIVATE_KEY', 'RPC_URL', 'AXELAR_RPC_URL', 'MNEMONIC',
        'GOVERNANCE_ADDRESS', 'ADMIN_ADDRESS', 'SERVICE_NAME', 'VOTING_THRESHOLD',
        'SIGNING_THRESHOLD', 'CONFIRMATION_HEIGHT', 'MINIMUM_ROTATION_DELAY',
        'DEPLOYMENT_TYPE', 'DEPLOYER', 'CONTRACT_ADMIN', 'PROVER_ADMIN',
        'DEPOSIT_VALUE', 'REWARD_AMOUNT', 'TOKEN_DENOM', 'MULTISIG_ADDRESS',
        'VOTING_VERIFIER_ADDRESS', 'REWARDS_ADDRESS', 'ROUTER_ADDRESS',
        'GATEWAY_ADDRESS', 'MULTISIG_ADDRESS', 'MULTISIG_PROVER_ADDRESS',
        'COORDINATOR_ADDRESS'
    ];
    const configJson = {};
    for (const key of configKeys) {
        if (config[key]) {
            configJson[key] = config[key];
        }
    }
    const jsonString = JSON.stringify(configJson, null, 2);
    console.log(jsonString);
    fs.writeFileSync('deployment_config.json', jsonString);
    console.log("✅ JSON configuration saved to deployment_config.json. You can use it for resuming deployment.");
    process.exit(0);
}
/**
 * This is the continuation point if the script is resumed from JSON
 */
function gotoAfterChainRegistration() {
    return __awaiter(this, void 0, void 0, function* () {
        console.log("✅ Continuing deployment from saved state...");
        // Run the verification step that gateway router was registered
        yield verifyExecution();
        // Retrieve the Multisig Contract Address
        const multisigAddress = (0, child_process_1.execSync)(`cat ../axelar-chains-config/info/${config.NAMESPACE}.json | jq -rM '.axelar.contracts.Multisig.address'`, { stdio: 'pipe' }).toString().trim();
        config.MULTISIG_ADDRESS = multisigAddress;
        console.log(`✅ Retrieved MULTISIG_ADDRESS: ${multisigAddress}`);
        // Retrieve the Multisig Prover Contract Address
        const query = `.axelar.contracts.MultisigProver.${config.CHAIN_NAME}.address`;
        const multisigProverAddress = (0, child_process_1.execSync)(`cat ../axelar-chains-config/info/${config.NAMESPACE}.json | jq -rM '${query}'`, { stdio: 'pipe' }).toString().trim();
        config.MULTISIG_PROVER_ADDRESS = multisigProverAddress;
        console.log(`✅ Retrieved MULTISIG_PROVER_ADDRESS: ${multisigProverAddress}`);
        // Construct JSON Payload for the Execute Call
        const jsonCmdMultisig = JSON.stringify({
            authorize_callers: {
                contracts: {
                    [multisigProverAddress]: config.CHAIN_NAME
                }
            }
        });
        console.log(`📜 JSON Command: ${jsonCmdMultisig}`);
        const coordinatorAddress = (0, child_process_1.execSync)(`cat ../axelar-chains-config/info/${config.NAMESPACE}.json | jq -rM '.axelar.contracts.Coordinator.address'`, { stdio: 'pipe' }).toString().trim();
        config.COORDINATOR_ADDRESS = coordinatorAddress;
        console.log(coordinatorAddress);
        const jsonCmdMultisigProver = JSON.stringify({
            register_prover_contract: {
                chain_name: config.CHAIN_NAME,
                new_prover_addr: config.MULTISIG_PROVER_ADDRESS
            }
        });
        console.log(jsonCmdMultisigProver);
        if (isCustomDevnet()) {
            console.log("Register prover contract");
            try {
                yield execAsync(`axelard tx wasm execute "${config.COORDINATOR_ADDRESS}" '${jsonCmdMultisigProver}' \
        --from amplifier \
        --gas auto \
        --gas-adjustment 2 \
        --node "${config.AXELAR_RPC_URL}" \
        --gas-prices 0.00005${config.TOKEN_DENOM} \
        --keyring-backend test \
        --chain-id "${config.NAMESPACE}"`);
                // Execute the Transaction for Multisig Contract
                console.log("⚡ Executing authorize_callers for Multisig Contract...");
                yield execAsync(`axelard tx wasm execute "${config.MULTISIG_ADDRESS}" '${jsonCmdMultisig}' \
        --from amplifier \
        --gas auto \
        --gas-adjustment 2 \
        --node "${config.AXELAR_RPC_URL}" \
        --gas-prices 0.00005${config.TOKEN_DENOM} \
        --keyring-backend test \
        --chain-id "${config.NAMESPACE}"`);
            }
            catch (error) {
                console.error(`Error registering prover contract: ${error}`);
                process.exit(1);
            }
        }
        else {
            // Actual networks require proposal for chain integration
            try {
                if (config.NAMESPACE === "devnet-amplifier") {
                    yield execAsync(`node ../cosmwasm/submit-proposal.js execute \
          -c Coordinator \
          -t "Register Multisig Prover for ${config.CHAIN_NAME}" \
          -d "Register Multisig Prover address for ${config.CHAIN_NAME} at Coordinator contract" \
          --runAs ${config.RUN_AS_ACCOUNT} \
          --deposit ${config.DEPOSIT_VALUE} \
          --msg '${jsonCmdMultisigProver}'`);
                    yield execAsync(`node ../cosmwasm/submit-proposal.js execute \
          -c Multisig \
          -t "Authorize Multisig Prover for ${config.CHAIN_NAME}" \
          -d "Authorize Multisig Prover address for ${config.CHAIN_NAME} at Multisig contract" \
          --runAs ${config.RUN_AS_ACCOUNT} \
          --deposit ${config.DEPOSIT_VALUE} \
          --msg '${jsonCmdMultisig}'`);
                }
                else {
                    yield execAsync(`node ../cosmwasm/submit-proposal.js execute \
          -c Coordinator \
          -t "Register Multisig Prover for ${config.CHAIN_NAME}" \
          -d "Register Multisig Prover address for ${config.CHAIN_NAME} at Coordinator contract" \
          --deposit ${config.DEPOSIT_VALUE} \
          --msg '${jsonCmdMultisigProver}'`);
                    yield execAsync(`node ../cosmwasm/submit-proposal.js execute \
          -c Multisig \
          -t "Authorize Multisig Prover for ${config.CHAIN_NAME}" \
          -d "Authorize Multisig Prover address for ${config.CHAIN_NAME} at Multisig contract" \
          --deposit ${config.DEPOSIT_VALUE} \
          --msg '${jsonCmdMultisig}'`);
                }
            }
            catch (error) {
                console.error(`Error submitting proposals: ${error}`);
                process.exit(1);
            }
        }
        printEnvJsonAndExit();
        console.log("🔍 Wait for multisig proposals to be approved...");
    });
}
/**
 * Function to handle the state after multisig proposals have been approved
 */
function gotoAfterMultisigProposals() {
    return __awaiter(this, void 0, void 0, function* () {
        yield verifyMultisig();
        yield createRewardPools();
        yield addFundsToRewardPools();
        yield createGenesisVerifierSet();
        yield deployGatewayContract();
        console.log("🎉 Deployment complete!");
    });
}
/**
 * Get the latest version of a contract
 */
function getLatestVersion(contractDir) {
    return __awaiter(this, void 0, void 0, function* () {
        try {
            const baseUrl = `${BASE_URL}/${contractDir}/`;
            // This would need to be implemented with a HTTP request to fetch versions
            // For now, we'll just log that this needs to be implemented with a proper HTTP client
            console.log(`⚠️ getLatestVersion needs to be implemented with HTTP requests to ${baseUrl}`);
            console.log(`⚠️ For now, please provide a specific version number when prompted.`);
        }
        catch (error) {
            console.error(`Error getting latest version: ${error}`);
        }
    });
}
/**
 * Function to download contract files from remote source
 */
function downloadContractFiles(userVersion) {
    return __awaiter(this, void 0, void 0, function* () {
        console.log("⚡ Downloading contract files...");
        // First ensure axios is installed
        try {
            require.resolve('axios');
        }
        catch (e) {
            console.error("❌ axios is not installed. Please run 'npm install axios' first.");
            process.exit(1);
        }
        const axios = require('axios');
        // Ensure the directory for downloads exists
        fs.mkdirSync(WASM_DIR, { recursive: true });
        // List of contract directories to check
        const contractDirectories = [
            "gateway",
            "multisig-prover",
            "voting-verifier"
        ];
        // Map to store contract file information
        const contractFiles = new Map();
        // Loop through each contract directory and get the files
        for (const dir of contractDirectories) {
            const fileName = dir.replace(/-/g, "_"); // Convert hyphens to underscores
            const contractKey = dir.replace(/-/g, "_"); // Use as key for the map
            const wasmFilePath = path.join(WASM_DIR, `${fileName}.wasm`);
            const checksumFilePath = path.join(WASM_DIR, `${fileName}_checksums.txt`);
            if (!userVersion) {
                yield getLatestVersion(dir);
                console.error("❌ No version specified and getLatestVersion is not fully implemented.");
                process.exit(1);
            }
            else {
                const fileUrl = `${BASE_URL}/${dir}/${userVersion}/${fileName}.wasm`;
                const checksumUrl = `${BASE_URL}/${dir}/${userVersion}/checksums.txt`;
                console.log(`⬇️ Downloading ${fileUrl}...`);
                try {
                    // Download the WASM file
                    const wasmResponse = yield axios({
                        method: 'GET',
                        url: fileUrl,
                        responseType: 'arraybuffer'
                    });
                    // Write the WASM file to disk
                    fs.writeFileSync(wasmFilePath, Buffer.from(wasmResponse.data));
                    console.log(`✅ Downloaded ${fileName}.wasm successfully!`);
                    // Download checksum file
                    console.log(`⬇️ Downloading ${checksumUrl}...`);
                    const checksumResponse = yield axios({
                        method: 'GET',
                        url: checksumUrl,
                        responseType: 'text'
                    });
                    // Write the checksum file to disk
                    fs.writeFileSync(checksumFilePath, checksumResponse.data);
                    console.log(`✅ Downloaded checksums.txt successfully!`);
                    // Store the contract file information
                    contractFiles.set(contractKey, {
                        name: dir,
                        fileName: fileName,
                        filePath: wasmFilePath,
                        checksumPath: checksumFilePath
                    });
                }
                catch (error) {
                    console.error(`❌ Error downloading files: ${error}`);
                    console.error("Please ensure the version and URLs are correct.");
                    process.exit(1);
                }
            }
        }
        return contractFiles;
    });
}
/**
 * Function to deploy contracts
 */
function deployContracts(contractFiles) {
    return __awaiter(this, void 0, void 0, function* () {
        if (isCustomDevnet()) {
            // Extract SALT for "VotingVerifier"
            const votingVerifier = contractFiles.get("voting_verifier");
            if (!votingVerifier) {
                console.error("Missing voting_verifier contract files!");
                process.exit(1);
            }
            extractSalt("voting_verifier");
            // Run the deployment command with explicit file path
            console.log("⚡ Deploying VotingVerifier Contract...");
            try {
                yield execAsync(`node ../cosmwasm/deploy-contract.js upload-instantiate \
        -m "${config.MNEMONIC}" \
        -a "${votingVerifier.filePath}" \
        -c "VotingVerifier" \
        -e "${config.NAMESPACE}" \
        -n "${config.CHAIN_NAME}" \
        --admin "${config.CONTRACT_ADMIN}" \
        -y \
        --salt "${config.SALT}"`);
            }
            catch (error) {
                console.error(`Error deploying VotingVerifier: ${error}`);
                process.exit(1);
            }
            // Extract SALT for "Gateway"
            const gateway = contractFiles.get("gateway");
            if (!gateway) {
                console.error("Missing gateway contract files!");
                process.exit(1);
            }
            extractSalt("gateway");
            // Run the deployment command for Gateway contract with explicit file path
            console.log("⚡ Deploying Gateway Contract...");
            try {
                yield execAsync(`node ../cosmwasm/deploy-contract.js upload-instantiate \
        -m "${config.MNEMONIC}" \
        -a "${gateway.filePath}" \
        -c "Gateway" \
        -e "${config.NAMESPACE}" \
        -n "${config.CHAIN_NAME}" \
        --admin "${config.CONTRACT_ADMIN}" \
        -y \
        --salt "${config.SALT}"`);
            }
            catch (error) {
                console.error(`Error deploying Gateway: ${error}`);
                process.exit(1);
            }
            // Extract SALT for "MultisigProver"
            const multisigProver = contractFiles.get("multisig_prover");
            if (!multisigProver) {
                console.error("Missing multisig_prover contract files!");
                process.exit(1);
            }
            extractSalt("multisig_prover");
            // Run the deployment command for MultisigProver contract with explicit file path
            console.log("⚡ Deploying MultisigProver Contract...");
            try {
                yield execAsync(`node ../cosmwasm/deploy-contract.js upload-instantiate \
        -m "${config.MNEMONIC}" \
        -a "${multisigProver.filePath}" \
        -c "MultisigProver" \
        -e "${config.NAMESPACE}" \
        -n "${config.CHAIN_NAME}" \
        --admin "${config.CONTRACT_ADMIN}" \
        -y \
        --salt "${config.SALT}"`);
            }
            catch (error) {
                console.error(`Error deploying MultisigProver: ${error}`);
                process.exit(1);
            }
        }
        else {
            // Non-custom devnet logic
            try {
                yield execAsync(`node ./../cosmwasm/deploy-contract.js instantiate -c VotingVerifier --fetchCodeId --instantiate2 --admin ${config.CONTRACT_ADMIN}`);
                yield execAsync(`node ./../cosmwasm/deploy-contract.js instantiate -c Gateway --fetchCodeId --instantiate2 --admin ${config.CONTRACT_ADMIN}`);
                yield execAsync(`node ./../cosmwasm/deploy-contract.js instantiate -c MultisigProver --fetchCodeId --instantiate2 --admin ${config.CONTRACT_ADMIN}`);
            }
            catch (error) {
                console.error(`Error instantiating contracts: ${error}`);
                process.exit(1);
            }
        }
    });
}
/**
 * Main function to drive the deployment process
 */
function main() {
    return __awaiter(this, void 0, void 0, function* () {
        console.log("🚀 Welcome to Axelar Deployment Setup 🚀");
        // Ask user if this is a new deployment or continuation
        const deploymentType = yield question("Is this a new deployment? (yes/no): ");
        // Check if deployment is a continuation
        if (deploymentType.toLowerCase() === "no") {
            console.log("✅ Loading configuration from deployment_config.json...");
            if (!fs.existsSync("deployment_config.json")) {
                console.log("❌ Error: deployment_config.json not found. Cannot resume deployment.");
                process.exit(1);
            }
            // Load the configuration from JSON
            const savedConfig = JSON.parse(fs.readFileSync("deployment_config.json", 'utf8'));
            // Update our config object with saved values
            Object.assign(config, savedConfig);
            console.log("✅ Environment restored! Resuming deployment...");
            const verifiersRegistered = yield question("Have verifiers registered support for the chain? (yes/no): ");
            if (verifiersRegistered.toLowerCase() === "yes") {
                const multisigProposalsApproved = yield question("Have multisig proposals been approved? (yes/no): ");
                if (multisigProposalsApproved.toLowerCase() === "yes") {
                    yield gotoAfterMultisigProposals();
                }
                else {
                    yield gotoAfterChainRegistration();
                }
            }
            else {
                printEnvJsonAndExit();
            }
            rl.close();
            return;
        }
        // New deployment flow
        yield getNetworkName();
        config.CHAIN_NAME = yield question("Enter Chain Name: ");
        config.CHAIN_ID = yield question("Enter Chain ID: ");
        config.TOKEN_SYMBOL = yield question("Enter Token Symbol: ");
        config.GAS_LIMIT = yield question("Gas Limit: ");
        yield validatePrivateKey();
        yield validateRpcUrl();
        yield validateAxelarRpcUrl();
        config.MNEMONIC = yield question("Enter Axelar Network Wallet MNEMONIC: ");
        const userVersion = yield question("Enter version to retrieve (leave empty for latest): ");
        console.log("✅ Environment Variables Set:");
        console.log(`   NETWORK=${config.NAMESPACE}`);
        console.log(`   CHAIN_NAME=${config.CHAIN_NAME}`);
        console.log(`   CHAIN_ID=${config.CHAIN_ID}`);
        console.log(`   TOKEN_SYMBOL=${config.TOKEN_SYMBOL}`);
        console.log(`   GAS_LIMIT=${config.GAS_LIMIT}`);
        console.log(`   TARGET_CHAIN_PRIVATE_KEY=${config.TARGET_CHAIN_PRIVATE_KEY}`);
        console.log(`   MNEMONIC=${config.MNEMONIC}`);
        console.log(`   RPC_URL=${config.RPC_URL}`);
        console.log(`   AXELAR_RPC_URL=${config.AXELAR_RPC_URL}`);
        // Create entry into namespace json
        generateJsonConfig();
        insertIntoNetworkConfig();
        // Check if the namespace is a custom devnet
        if (isCustomDevnet()) {
            console.log("🔧 Custom devnet detected. Proceeding with full deployment flow...");
            // Proceed with contract deployment as usual
            yield setupWallet();
            // Download contract files and get the paths
            const contractFiles = yield downloadContractFiles(userVersion);
            // Run the command to get the governance address
            try {
                config.GOVERNANCE_ADDRESS = (0, child_process_1.execSync)(`jq -r '.axelar.contracts.ServiceRegistry.governanceAccount' ../axelar-chains-config/info/${config.NAMESPACE}.json`, { stdio: 'pipe' }).toString().trim();
                config.ADMIN_ADDRESS = config.GOVERNANCE_ADDRESS;
                config.CONTRACT_ADMIN = config.GOVERNANCE_ADDRESS;
                config.PROVER_ADMIN = config.GOVERNANCE_ADDRESS;
                config.DEPLOYER = config.GOVERNANCE_ADDRESS;
                config.SERVICE_NAME = "validators";
                config.VOTING_THRESHOLD = '["6", "10"]';
                config.SIGNING_THRESHOLD = '["6", "10"]';
                config.CONFIRMATION_HEIGHT = "1";
                config.MINIMUM_ROTATION_DELAY = "0";
                config.DEPLOYMENT_TYPE = "create";
                config.DEPOSIT_VALUE = "100000000";
                console.log(`✅ Extracted GOVERNANCE_ADDRESS: ${config.GOVERNANCE_ADDRESS}`);
                console.log(`✅ Extracted ADMIN_ADDRESS: ${config.ADMIN_ADDRESS}`);
            }
            catch (error) {
                console.error(`Error extracting governance address: ${error}`);
                process.exit(1);
            }
        }
        else {
            console.log(`🚀 Predefined network detected (${config.NAMESPACE}). Using existing governance and admin addresses.`);
            setPredefinedValues();
            // Display the reused values for confirmation
            console.log(`✅ Predefined values set for ${config.NAMESPACE}:`);
            console.log(`   GOVERNANCE_ADDRESS=${config.GOVERNANCE_ADDRESS}`);
            console.log(`   ADMIN_ADDRESS=${config.ADMIN_ADDRESS}`);
            console.log(`   SERVICE_NAME=${config.SERVICE_NAME}`);
            console.log(`   VOTING_THRESHOLD=${config.VOTING_THRESHOLD}`);
            console.log(`   SIGNING_THRESHOLD=${config.SIGNING_THRESHOLD}`);
            console.log(`   CONFIRMATION_HEIGHT=${config.CONFIRMATION_HEIGHT}`);
            console.log(`   MINIMUM_ROTATION_DELAY=${config.MINIMUM_ROTATION_DELAY}`);
            console.log(`   DEPLOYMENT_TYPE=${config.DEPLOYMENT_TYPE}`);
            console.log(`   DEPLOYER=${config.DEPLOYER}`);
        }
        // Run the deployment script and capture the output
        console.log("⚡ Running deploy-amplifier-gateway.js...");
        try {
            const setupOutput = (0, child_process_1.execSync)(`node ../evm/deploy-amplifier-gateway.js --env "${config.NAMESPACE}" -n "${config.CHAIN_NAME}" -m "${config.DEPLOYMENT_TYPE}" --minimumRotationDelay "${config.MINIMUM_ROTATION_DELAY}" --predictOnly -p "${config.TARGET_CHAIN_PRIVATE_KEY}"`, { stdio: 'pipe' }).toString();
            // Print output for debugging
            console.log(setupOutput);
            // Extract the predicted gateway proxy address
            extractProxyGatewayAddress(setupOutput);
        }
        catch (error) {
            console.error(`Error running deployment script: ${error}`);
            process.exit(1);
        }
        // Call the functions to update JSON
        updateVotingVerifierConfig();
        updateMultisigProver();
        if (isCustomDevnet()) {
            // Deploy contracts using full file paths
            const contractFiles = yield downloadContractFiles(userVersion || "latest");
            yield deployContracts(contractFiles);
            // Get wallet address and token denomination
            yield getTokenDenomination();
        }
        else {
            try {
                yield execAsync(`node ./../cosmwasm/deploy-contract.js instantiate -c VotingVerifier --fetchCodeId --instantiate2 --admin ${config.CONTRACT_ADMIN}`);
                yield execAsync(`node ./../cosmwasm/deploy-contract.js instantiate -c Gateway --fetchCodeId --instantiate2 --admin ${config.CONTRACT_ADMIN}`);
                yield execAsync(`node ./../cosmwasm/deploy-contract.js instantiate -c MultisigProver --fetchCodeId --instantiate2 --admin ${config.CONTRACT_ADMIN}`);
            }
            catch (error) {
                console.error(`Error instantiating contracts: ${error}`);
                process.exit(1);
            }
        }
        // Run the functions to extract values
        extractRouterAddress();
        extractGatewayAddress();
        const jsonCmdRegister = buildJsonCmdRegister();
        if (isCustomDevnet()) {
            // Run the command to register the chain
            console.log("⚡ Registering the chain...");
            try {
                yield execAsync(`axelard tx wasm execute "${config.ROUTER_ADDRESS}" '${jsonCmdRegister}' \
        --from amplifier \
        --gas auto \
        --gas-adjustment 2 \
        --node "${config.AXELAR_RPC_URL}" \
        --gas-prices 0.00005${config.TOKEN_DENOM} \
        --keyring-backend test \
        --chain-id "${config.NAMESPACE}"`);
            }
            catch (error) {
                console.error(`Error registering chain: ${error}`);
                process.exit(1);
            }
        }
        else {
            try {
                if (config.NAMESPACE === "devnet-amplifier") {
                    yield execAsync(`node ../cosmwasm/submit-proposal.js execute \
          -c Router \
          -t "Register Gateway for ${config.CHAIN_NAME}" \
          -d "Register Gateway address for ${config.CHAIN_NAME} at Router contract" \
          --runAs ${config.RUN_AS_ACCOUNT} \
          --deposit ${config.DEPOSIT_VALUE} \
          --msg '${jsonCmdRegister}'`);
                }
                else {
                    yield execAsync(`node ../cosmwasm/submit-proposal.js execute \
          -c Router \
          -t "Register Gateway for ${config.CHAIN_NAME}" \
          -d "Register Gateway address for ${config.CHAIN_NAME} at Router contract" \
          --deposit ${config.DEPOSIT_VALUE} \
          --msg '${jsonCmdRegister}'`);
                }
            }
            catch (error) {
                console.error(`Error submitting register gateway proposal: ${error}`);
                process.exit(1);
            }
        }
        // Generate extra envs for next steps needed as part of verifier set
        try {
            config.REWARDS_ADDRESS = (0, child_process_1.execSync)(`cat ../axelar-chains-config/info/${config.NAMESPACE}.json | jq -rM '.axelar.contracts.Rewards.address'`, { stdio: 'pipe' }).toString().trim();
            config.MULTISIG_ADDRESS = (0, child_process_1.execSync)(`cat ../axelar-chains-config/info/${config.NAMESPACE}.json | jq -rM '.axelar.contracts.Multisig.address'`, { stdio: 'pipe' }).toString().trim();
            const jsonPath = `.axelar.contracts.VotingVerifier.${config.CHAIN_NAME}.address`;
            config.VOTING_VERIFIER_ADDRESS = (0, child_process_1.execSync)(`cat ../axelar-chains-config/info/${config.NAMESPACE}.json | jq -rM '${jsonPath}'`, { stdio: 'pipe' }).toString().trim();
            console.log(config.VOTING_VERIFIER_ADDRESS);
        }
        catch (error) {
            console.error(`Error extracting addresses: ${error}`);
            process.exit(1);
        }
        console.log("🎉 Chain registration complete! Need to Update the Verifiers!");
        printEnvJsonAndExit();
        rl.close();
    });
}
// Run the main function
main().catch(error => {
    console.error(`Unhandled error in main: ${error}`);
    process.exit(1);
});
