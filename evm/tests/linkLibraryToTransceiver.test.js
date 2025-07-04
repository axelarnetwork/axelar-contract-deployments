const { expect } = require('chai');
const { ethers } = require('hardhat');
const { linkLibraryToTransceiver } = require('../deploy-contract');

describe('linkLibraryToTransceiver', () => {
    it('should replace the correct placeholder with the library address', () => {
        const libraryName = 'TransceiverStructs';
        const libraryNameHash = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(libraryName));
        const placeholder = `__$${libraryNameHash.slice(2, 36)}__`;
        const fakeBytecode = `0x6000${placeholder}6000`;
        const fakeJson = { bytecode: fakeBytecode };
        const address = '0x1234567890123456789012345678901234567890';
        const linked = linkLibraryToTransceiver(fakeJson, address);
        expect(linked.bytecode).to.include(address.replace('0x', ''));
        expect(linked.bytecode).to.not.include(placeholder);
    });

    it('should throw if the placeholder is not found', () => {
        const fakeJson = { bytecode: '0x60006000' };
        const address = '0x1234567890123456789012345678901234567890';
        expect(() => linkLibraryToTransceiver(fakeJson, address)).to.throw('Library placeholder');
    });
});
