const { Relayer } = require('@openzeppelin/defender-relay-client');
const { ethers } = require('ethers');

const API_KEY = '2iQcxEoebWTEiSpf8utv1d1z3msak6E4';
const SECRET = '3s9oQFKNTTcunWyyXW87WhhBmuSL9EKktX4k8171bQwmawYw87CJsDfjrVqHfNFr';

async function signMessage() {
    const relayer = new Relayer({ apiKey: API_KEY, apiSecret: SECRET });

    const str =
        '[arbiscan.io 12/01/2024 13:57:42] I, hereby verify that I am the owner/creator of the address [0x23ee2343B892b1BB63503a4FAbc840E0e2C6810f]';

    const message = ethers.utils.solidityKeccak256(['string'], [str]);

    const signature = await relayer.sign({ message });
    console.log(`Signature   =  `, signature);

    console.log('utils sig value', ethers.utils.joinSignature(signature));
}

signMessage();
