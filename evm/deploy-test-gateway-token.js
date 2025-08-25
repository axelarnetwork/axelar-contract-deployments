'use strict';

const { ethers } = require('hardhat');
const {
    Contract,
    Wallet,
    providers: { JsonRpcProvider },
    utils: { parseEther, keccak256, defaultAbiCoder, arrayify, hexlify, randomBytes },
    constants: { AddressZero },
} = ethers;
const { Command, Option } = require('commander');

const { mainProcessor, getContractJSON, printInfo } = require('./utils');
const { addBaseOptions } = require('./cli-utils');

async function getCommandId(gateway) {
    return hexlify(randomBytes(32));
}

async function processCommand(_, chain, options) {
    printInfo('This script is for testing deployment, not for deployment in production');
    const provider = new JsonRpcProvider(chain.rpc);
    const wallet = new Wallet(options.privateKey, provider);
    const gatewayAddress = chain.contracts.AxelarGateway.address;
    const gateway = new Contract(gatewayAddress, getContractJSON('AxelarGateway').abi, provider);

    const commandID = await getCommandId(gateway);
    const chainId = chain.chainId;
    const command = 'deployToken';
    const params = defaultAbiCoder.encode(
        ['string', 'string', 'uint256', 'uint256', 'address', 'uint256'],
        ['WrappedNativeToken', `W${chain.tokenSymbol}`, 18, parseEther('100'), AddressZero, parseEther('10')],
    );
    const data = defaultAbiCoder.encode(['uint256', 'bytes32[]', 'string[]', 'bytes[]'], [chainId, [commandID], [command], [params]]);

    const dataHash = arrayify(keccak256(data));
    const signature = await wallet.signMessage(dataHash);
    const proof = defaultAbiCoder.encode(['address[]', 'uint256[]', 'uint256', 'bytes[]'], [[wallet.address], [1], 1, [signature]]);
    const input = defaultAbiCoder.encode(['bytes', 'bytes'], [data, proof]);

    await gateway.connect(wallet).execute(input, chain.gasOptions);
}

async function main(options) {
    await mainProcessor(options, processCommand, false);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('deploy-test-gateway-token')
        .description('Deploy a native wrapped token and integrate with AxelarGateway in order to test AxelarDepositService deployment');
    program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));
    addBaseOptions(program);

    program.action((options) => {
        main(options);
    });

    program.parse();
}
