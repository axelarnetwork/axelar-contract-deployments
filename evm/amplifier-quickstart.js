const { Command } = require('commander');
const { main: cosmwasmDeploy } = require('../cosmwasm/deploy-contract');
const { deployAmplifierGateway } = require('../evm/deploy-amplifier-gateway');
const { loadConfig } = require('../common/utils');

const { mainProcessor } = require('./utils');

const deployCosmWasmContract = async ({ contractName, chainName, salt, mnemonic, env, yes, codeId }) => {
    try {
        console.log(`Starting deployment for ${contractName} on ${chainName.name}`);
        await cosmwasmDeploy({
            contractName,
            chainName: chainName.axelarId,
            salt,
            mnemonic,
            env,
            yes,
            codeId,
        });
        console.log(`Deployment successful for ${contractName} on ${chainName.name}`);
    } catch (error) {
        console.error(`Error deploying ${contractName} on ${chainName.name}:`, error);
        throw error;
    }
};

const deployEvmContract = async (config, chainName, { salt, env, yes, privateKey }, predict) => {
    try {
        console.log(`Starting deployment for Ext. Gateway on ${chainName.name}`);
        const gateway = await deployAmplifierGateway(config, chainName, {
            salt,
            env,
            yes,
            privateKey,
            deployMethod: 'create3',
            previousSignersRetention: 15,
            minimumRotationDelay: 86400,
            predictOnly: predict,
            deployMethod: 'create3',
        });
        console.log(`Deployment successful for Gateway on ${chainName.name}`);
        return gateway;
    } catch (error) {
        console.error(`Error deploying Gateway on ${chainName.name}:`, error);
        throw error;
    }
};

async function processCommand(config, chainName, { salt, env, yes, privateKey, mnemonic }) {
    // Deployment for Verifier
    await deployCosmWasmContract({
        contractName: 'VotingVerifier',
        chainName,
        salt,
        mnemonic,
        env,
        yes,
        codeId: 626, // Hardcoded Code ID for Gateway
    });
    // // Deployment for Gateway
    await deployCosmWasmContract({
        contractName: 'Gateway',
        chainName,
        salt,
        mnemonic,
        env,
        yes,
        codeId: 616, // Hardcoded Code ID for Gateway
    });
    // Deployment for MultisigProver
    await deployCosmWasmContract({
        contractName: 'MultisigProver',
        chainName,
        salt,
        mnemonic,
        env,
        yes,
        codeId: 618, // Hardcoded Code ID for MultisigProver
    });

    // UPDATE VERIFIER SET
    


    // Deploy Gateway
    // await deployEvmContract({ config, chainName, salt, env, yes, privateKey }, false);
}

async function main(options) {
    //Create new chain in config
    const configOld = loadConfig(options.env);
    const newChainName = options.chainNames;
    configOld.chains[newChainName] = {
        name: newChainName,
        axelarId: newChainName,
        id: newChainName,
        rpc: options.rpc,
        contracts: {
            Create3Deployer: {
                address: options.create3Deployer,
                deploymentMethod: 'create2',
                salt: 'Create3Deployer',
            },
        },
    };

    const configNew = loadConfig(options.env);

    const newChain = configNew.chains[newChainName];

    //PREDICT GATEWAY ADDRESS TO BE PASSED INTO NEWLY BUILT CHAIN NAME OBJ FOR INTEGRATION
    const gateway = await deployEvmContract(configNew, newChain, { ...options }, true);

    configNew.chains[newChainName].contracts.AxelarGateway = {
        address: gateway,
    };

    const theNewChain = newChain.name;

    configNew.axelar.contracts.VotingVerifier[theNewChain] = {
        governanceAddress: 'axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9',
        serviceName: 'validators',
        sourceGatewayAddress: gateway,
        votingThreshold: ['6', '10'],
        blockExpiry: 10,
        confirmationHeight: 1,
        msgIdFormat: 'hex_tx_hash_and_event_index',
        addressFormat: 'eip55',
    };
    configNew.axelar.contracts.Gateway[theNewChain] = {
        governanceAddress: 'axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9',
    };
    configNew.axelar.contracts.MultisigProver[theNewChain] = {
        governanceAddress: 'axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9',
        adminAddress: options.admin,
        signingThreshold: ['6', '10'],
        serviceName: 'validators',
        verifierSetDiffThreshold: 0,
        encoder: 'abi',
        keyType: 'ecdsa',
    };

    await mainProcessor(options, processCommand);
}

const program = new Command();

program
    .option('-c, --chainNames <chainNames>', 'Chain name (e.g., avalanche-fuji)')
    .option('-ct --chainType <chainType>', 'Chain type (e.g. evm)')
    .option('-s, --salt <salt>', 'Base salt for deployment (e.g., "saltBase")')
    .option('-m, --mnemonic <mnemonic>', 'Mnemonic for the wallet')
    .option('-p, --privateKey <privateKey>', 'Private key for the wallet')
    .option('-e, --env <env>', 'Environment (e.g., devnet-amplifier)')
    .option('-y, --yes <yes>', 'Auto-confirm actions without prompt', false)
    .option('-r, --rpc <rpc>, RPC for new chain')
    .option('-cd --create3Deployer <create3Deployer>', 'Create3 Deployer')
    .option('-a --admin <admin>', 'Admin address');

program.action(async (options) => {
    try {
        await main(options);
        console.log('All deployments completed successfully!');
        console.log(
            'Submit the following form to get your contracts whitelisted on Axelar Devnet: https://docs.google.com/forms/d/e/1FAIpQLSchD7P1WfdSCQfaZAoqX7DyqJOqYKxXle47yrueTbOgkKQDiQ/viewform',
        );
    } catch (error) {
        console.error('One or more deployments failed:', error);
        process.exit(1);
    }
});

program.parse(process.argv);

//WORKING COMMAND
//node evm/amplifier-quickstart.js --chainNames "ben-eth" --salt f  --mnemonic "hint pause black nerve govern embody fade gesture fluid arrange soldier outdoor front risk scorpion narrow flower modify boat social theory real pluck lunch" --env devnet-amplifier --yes yes --privateKey "fca3e021285060f5918c19d475de4357b7be82281958a2401058d2b30759b92d" --chainType evm --rpc "https://1rpc.io/sepolia" --create3Deployer "0x6513Aedb4D1593BA12e50644401D976aebDc90d8" --admin axelar199g24qmzg4znysvnwfknqrmlupazxmfxjq7vsf
