const { Cl } = require('@stacks/transactions');
const { ethers } = require('hardhat');
const {
    BigNumber,
    utils: { hexlify, arrayify },
} = ethers;

const encodeAmplifierVerifiersForStacks = (verifierSet, signers) => {
    const weightedSigners = signers
        .map((signer) => ({
            pub_key: arrayify(`0x${signer.pub_key.ecdsa}`),
            weight: Number(signer.weight),
        }))
        .sort((a, b) => hexlify(a.pub_key).localeCompare(hexlify(b.pub_key)))
        .map((signer) => ({
            signer: hexlify(signer.pub_key),
            weight: signer.weight,
        }));

    const clarityWeightedSigners = weightedSigners.map((signer) =>
        Cl.tuple({
            signer: Cl.bufferFromHex(signer.signer),
            weight: Cl.uint(signer.weight),
        }),
    );

    const nonce = ethers.utils.hexZeroPad(BigNumber.from(verifierSet.created_at).toHexString(), 32);

    return {
        claritySigners: Cl.serialize(
            Cl.tuple({
                signers: Cl.list(clarityWeightedSigners),
                threshold: Cl.uint(Number(verifierSet.threshold)),
                nonce: Cl.bufferFromHex(nonce),
            }),
        ),
        weightedSigners,
        threshold: Number(verifierSet.threshold),
        nonce,
    };
};

module.exports = {
    encodeAmplifierVerifiersForStacks,
};
