const { ethers } = require('ethers');
// console.log("random,", ethers.Wallet.createRandom().mnemonic)
// Replace this seed with your own for deterministic behavior
const seed = 'axelar dust wallet random private key generation using ethers library for use';
// Create an HDNode from the seed
const mnemonic = 'abandon ability able about above absent absorb abstract absurd abuse access accident';
console.log('random menmonic', ethers.utils.isValidMnemonic(mnemonic));
const hdNode = ethers.utils.HDNode.fromMnemonic(mnemonic);
// Derive a wallet from the HDNode (without connecting to a provider)
const wallet = new ethers.Wallet(hdNode.privateKey);
// const wallet = new ethers.Wallet.fromMnemonic('account address collect dust give exact key private same that word unique')
// Get the private key
const privateKey = wallet.privateKey;
// Get the Ethereum address
const address = wallet.address;
console.log('Seed:', seed);
console.log('Private Key:', privateKey);
console.log('Address:', address);
