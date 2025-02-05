'use strict';

const { Command } = require('commander');
const { ethers } = require('hardhat');
const {
    utils: { hexlify },
} = ethers;

async function stellarAddressToBytes(address) {
    console.log(hexlify(Buffer.from(address, 'ascii')));
}

if (require.main === module) {
    const program = new Command();

    program.name('cli-utils').description('Stellar CLI Utils.');

    program
        .command('stellar-address-to-bytes <address>')
        .description('stellar address to bytes format')
        .action((address) => {
            stellarAddressToBytes(address);
        });

    program.parse();
}
