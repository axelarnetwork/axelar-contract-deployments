'use strict';

/**
 * @fileoverview Mocha Tests for Library Linking
 *
 * This file contains Mocha tests for the library linking functionality.
 * Tests both hash-based and old-style placeholders.
 */

const { expect } = require('chai');
const { linkLibrariesInBytecode } = require('../utils');

describe('Library Linking Tests', () => {
    describe('Old-style placeholder linking', () => {
        it('should successfully link old-style placeholders', () => {
            // Mock bytecode with old-style placeholder (padded to 40 characters)
            const oldStyleBytecode = '0xf3fe__TransceiverStructs____________________60806040';

            const libraries = {
                '@wormhole-foundation/native_token_transfer/libraries/TransceiverStructs.sol:TransceiverStructs':
                    '0x1234567890123456789012345678901234567890',
            };

            // Verify original bytecode contains placeholder
            expect(oldStyleBytecode).to.include('__TransceiverStructs____________________');

            // Perform linking
            const linkedBytecode = linkLibrariesInBytecode(oldStyleBytecode, libraries);

            // Verify placeholder was replaced
            expect(linkedBytecode).to.not.include('__TransceiverStructs____________________');
            expect(linkedBytecode).to.include('1234567890123456789012345678901234567890');

            // Verify bytecode length remains the same
            expect(linkedBytecode.length).to.equal(oldStyleBytecode.length);
        });

        it('should throw error when library address is missing', () => {
            const oldStyleBytecode = '0xf3fe__TransceiverStructs____________________60806040';

            const libraries = {
                DifferentLibrary: '0x1234567890123456789012345678901234567890',
            };

            expect(() => {
                linkLibrariesInBytecode(oldStyleBytecode, libraries);
            }).to.throw('Library placeholder not found for DifferentLibrary');
        });
    });

    describe('Hash-based placeholder linking', () => {
        it('should successfully link fully qualified name placeholders', () => {
            // Generate hash-based placeholder for fully qualified name
            const { ethers } = require('hardhat');
            const fullyQualifiedName = '@wormhole-foundation/native_token_transfer/libraries/TransceiverStructs.sol:TransceiverStructs';
            const fqHash = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(fullyQualifiedName));
            const fqPlaceholder = `__$${fqHash.slice(2, 36)}$__`;

            // Mock bytecode with fully qualified placeholder
            const fqBytecode = `0x608060405234801561001057__$3a35edd6b3039ed7bfdd08ed34efbcdda7$__6000f3fe`;

            const libraries = {
                [fullyQualifiedName]: '0x1234567890123456789012345678901234567890',
            };

            // Verify original bytecode contains placeholder
            expect(fqBytecode).to.include(fqPlaceholder);

            // Perform linking
            const linkedBytecode = linkLibrariesInBytecode(fqBytecode, libraries);

            // Verify placeholder was replaced
            expect(linkedBytecode).to.not.include(fqPlaceholder);
            expect(linkedBytecode).to.include('1234567890123456789012345678901234567890');

            // Verify bytecode length remains the same
            expect(linkedBytecode.length).to.equal(fqBytecode.length);
        });

        it('should throw error when hash-based placeholder is not found', () => {
            const { ethers } = require('hardhat');
            const libraryName = 'TransceiverStructs';
            const libraryNameHash = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(libraryName));
            const hashPlaceholder = `__$${libraryNameHash.slice(0, 34)}$__`;

            const hashBytecode = `0x608060405234801561001057__$3a35edd6b3039ed7bfdd08ed34efbcdda7$__6000f3fe`;

            const libraries = {
                TransceiverStructs: '0x1234567890123456789012345678901234567890',
            };

            expect(() => {
                linkLibrariesInBytecode(hashBytecode, libraries);
            }).to.throw('Library placeholder not found for TransceiverStructs');
        });
    });
});

describe('Multiple library placeholder handling', () => {
    it('should replace all occurrences of hash-based placeholders', () => {
        // Create bytecode with multiple occurrences of the same placeholder
        const { ethers } = require('hardhat');
        const fullyQualifiedName = 'contracts/libraries/TransceiverStructs.sol:TransceiverStructs';
        const libraryNameHash = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(fullyQualifiedName));
        const hashPlaceholder = `__$${libraryNameHash.slice(2, 36)}$__`;

        // Create bytecode with 3 occurrences of the placeholder
        const bytecode = `0xf3fe${hashPlaceholder}33333333${hashPlaceholder}608033333333${hashPlaceholder}`;

        const libraries = {
            [fullyQualifiedName]: '0x1234567890123456789012345678901234567890',
        };

        // Perform linking
        const linkedBytecode = linkLibrariesInBytecode(bytecode, libraries);

        // Verify all placeholders were replaced
        expect(linkedBytecode).to.not.include(hashPlaceholder);

        // Count replacement occurrences
        const replacementAddress = '1234567890123456789012345678901234567890';
        const replacementOccurrences = (linkedBytecode.match(new RegExp(replacementAddress, 'g')) || []).length;
        expect(replacementOccurrences).to.equal(3);

        // Verify bytecode length is correct (each placeholder is 40 chars, replacement is 40 chars)
        expect(linkedBytecode.length).to.equal(bytecode.length);
    });

    it('should replace all occurrences of old-style placeholders', () => {
        // Create bytecode with multiple occurrences of old-style placeholder
        const oldStylePlaceholder = `__TransceiverStructs`.padEnd(40, '_');

        // Create bytecode with 2 occurrences of the placeholder
        const bytecode = `0xf3fe${oldStylePlaceholder}33333333${oldStylePlaceholder}608033333333`;

        const libraries = {
            'contracts/libraries/TransceiverStructs.sol:TransceiverStructs': '0x1234567890123456789012345678901234567890',
        };

        // Perform linking
        const linkedBytecode = linkLibrariesInBytecode(bytecode, libraries);

        // Verify all placeholders were replaced
        expect(linkedBytecode).to.not.include(oldStylePlaceholder);

        // Count replacement occurrences
        const replacementAddress = '1234567890123456789012345678901234567890';
        const replacementOccurrences = (linkedBytecode.match(new RegExp(replacementAddress, 'g')) || []).length;
        expect(replacementOccurrences).to.equal(2);

        // Verify bytecode length is correct
        expect(linkedBytecode.length).to.equal(bytecode.length);
    });

    it('should handle multiple libraries with different placeholder types', () => {
        // Create bytecode with different libraries using different placeholder types
        const { ethers } = require('hardhat');

        // Library 1: Hash-based placeholder
        const fullyQualifiedName1 = 'contracts/libraries/TransceiverStructs.sol:TransceiverStructs';
        const libraryNameHash1 = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(fullyQualifiedName1));
        const hashPlaceholder = `__$${libraryNameHash1.slice(2, 36)}$__`;

        // Library 2: Old-style placeholder
        const oldStylePlaceholder = `__HelperLibrary`.padEnd(40, '_');

        // Create bytecode with both types of placeholders
        const bytecode = `0x23232323${hashPlaceholder}33333333${oldStylePlaceholder}55555555`;

        const libraries = {
            [fullyQualifiedName1]: '0x1234567890123456789012345678901234567890',
            'contracts/libraries/HelperLibrary.sol:HelperLibrary': '0xabcdefabcdefabcdefabcdefabcdefabcdefabcd',
        };

        // Perform linking
        const linkedBytecode = linkLibrariesInBytecode(bytecode, libraries);

        // Verify all placeholders were replaced
        expect(linkedBytecode).to.not.include(hashPlaceholder);
        expect(linkedBytecode).to.not.include(oldStylePlaceholder);

        // Count replacement occurrences
        const replacementAddress1 = '1234567890123456789012345678901234567890';
        const replacementAddress2 = 'abcdefabcdefabcdefabcdefabcdefabcdefabcd';
        const replacementOccurrences1 = (linkedBytecode.match(new RegExp(replacementAddress1, 'g')) || []).length;
        const replacementOccurrences2 = (linkedBytecode.match(new RegExp(replacementAddress2, 'g')) || []).length;
        expect(replacementOccurrences1).to.equal(1);
        expect(replacementOccurrences2).to.equal(1);

        // Verify bytecode length is correct
        expect(linkedBytecode.length).to.equal(bytecode.length);
    });
});

describe('Library name handling', () => {
    it('should handle library names with colons correctly', () => {
        const { ethers } = require('hardhat');
        const fullyQualifiedName = 'contracts/libraries/TransceiverStructs.sol:TransceiverStructs';
        const libraryNameHash = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(fullyQualifiedName));
        const hashPlaceholder = `__$${libraryNameHash.slice(2, 36)}$__`;

        // Create bytecode with hash placeholder only
        const bytecode = `0x608060405234801561001057600080fd5b5060405161029e38038061029e83398101604081905261002f91610054565b600080546001600160a01b0319166001600160a01b039290921691909117905550610084565b60006020828403121561006657600080fd5b81516001600160a01b038116811461007d57600080fd5b9392505050565b610207806100936000396000f3fe${hashPlaceholder}608060405234801561001057600080fd5b5060405161029e38038061029e83398101604081905261002f91610054565b600080546001600160a01b0319166001600160a01b039290921691909117905550610084565b60006020828403121561006657600080fd5b81516001600160a01b038116811461007d57600080fd5b9392505050565b610207806100936000396000f3fe`;

        const libraries = {
            [fullyQualifiedName]: '0x1234567890123456789012345678901234567890',
        };

        // Perform linking
        const linkedBytecode = linkLibrariesInBytecode(bytecode, libraries);

        // Verify hash placeholder was replaced
        expect(linkedBytecode).to.not.include(hashPlaceholder);

        // Count replacement occurrences
        const replacementAddress = '1234567890123456789012345678901234567890';
        const replacementOccurrences = (linkedBytecode.match(new RegExp(replacementAddress, 'g')) || []).length;
        expect(replacementOccurrences).to.equal(1);
    });

    it('should handle library names without colons correctly', () => {
        const { ethers } = require('hardhat');
        const simpleLibraryName = 'SimpleLibrary';
        const libraryNameHash = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(simpleLibraryName));
        const hashPlaceholder = `__$${libraryNameHash.slice(2, 36)}$__`;

        // Create bytecode with hash placeholder only
        const bytecode = `0x608060405234801561001057600080fd5b5060405161029e38038061029e83398101604081905261002f91610054565b600080546001600160a01b0319166001600160a01b039290921691909117905550610084565b60006020828403121561006657600080fd5b81516001600160a01b038116811461007d57600080fd5b9392505050565b610207806100936000396000f3fe${hashPlaceholder}608060405234801561001057600080fd5b5060405161029e38038061029e83398101604081905261002f91610054565b600080546001600160a01b0319166001600160a01b039290921691909117905550610084565b60006020828403121561006657600080fd5b81516001600160a01b038116811461007d57600080fd5b9392505050565b610207806100936000396000f3fe`;

        const libraries = {
            [simpleLibraryName]: '0xabcdefabcdefabcdefabcdefabcdefabcdefabcd',
        };

        // Perform linking
        const linkedBytecode = linkLibrariesInBytecode(bytecode, libraries);

        // Verify hash placeholder was replaced
        expect(linkedBytecode).to.not.include(hashPlaceholder);

        // Count replacement occurrences
        const replacementAddress = 'abcdefabcdefabcdefabcdefabcdefabcdefabcd';
        const replacementOccurrences = (linkedBytecode.match(new RegExp(replacementAddress, 'g')) || []).length;
        expect(replacementOccurrences).to.equal(1);
    });

    it('should handle library names with multiple colons correctly', () => {
        const { ethers } = require('hardhat');
        const complexLibraryName = 'contracts/libraries/MyLibrary.sol:MyLibrary:Extra';
        const libraryNameHash = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(complexLibraryName));
        const hashPlaceholder = `__$${libraryNameHash.slice(2, 36)}$__`;

        // Create bytecode with hash placeholder only
        const bytecode = `0x608060405234801561001057600080fd5b5060405161029e38038061029e83398101604081905261002f91610054565b600080546001600160a01b0319166001600160a01b039290921691909117905550610084565b60006020828403121561006657600080fd5b81516001600160a01b038116811461007d57600080fd5b9392505050565b610207806100936000396000f3fe${hashPlaceholder}608060405234801561001057600080fd5b5060405161029e38038061029e83398101604081905261002f91610054565b600080546001600160a01b0319166001600160a01b039290921691909117905550610084565b60006020828403121561006657600080fd5b81516001600160a01b038116811461007d57600080fd5b9392505050565b610207806100936000396000f3fe`;

        const libraries = {
            [complexLibraryName]: '0x1111111111111111111111111111111111111111',
        };

        // Perform linking
        const linkedBytecode = linkLibrariesInBytecode(bytecode, libraries);

        // Verify hash placeholder was replaced
        expect(linkedBytecode).to.not.include(hashPlaceholder);

        // Count replacement occurrences
        const replacementAddress = '1111111111111111111111111111111111111111';
        const replacementOccurrences = (linkedBytecode.match(new RegExp(replacementAddress, 'g')) || []).length;
        expect(replacementOccurrences).to.equal(1);
    });

    it('should handle empty library names gracefully', () => {
        const { ethers } = require('hardhat');
        const emptyLibraryName = '';
        const libraryNameHash = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(emptyLibraryName));
        const hashPlaceholder = `__$${libraryNameHash.slice(2, 36)}$__`;

        // Create bytecode with hash placeholder only
        const bytecode = `0x608060405234801561001057600080fd5b5060405161029e38038061029e83398101604081905261002f91610054565b600080546001600160a01b0319166001600160a01b039290921691909117905550610084565b60006020828403121561006657600080fd5b81516001600160a01b038116811461007d57600080fd5b9392505050565b610207806100936000396000f3fe${hashPlaceholder}608060405234801561001057600080fd5b5060405161029e38038061029e83398101604081905261002f91610054565b600080546001600160a01b0319166001600160a01b039290921691909117905550610084565b60006020828403121561006657600080fd5b81516001600160a01b038116811461007d57600080fd5b9392505050565b610207806100936000396000f3fe`;

        const libraries = {
            [emptyLibraryName]: '0x2222222222222222222222222222222222222222',
        };

        // Perform linking
        const linkedBytecode = linkLibrariesInBytecode(bytecode, libraries);

        // Verify hash placeholder was replaced
        expect(linkedBytecode).to.not.include(hashPlaceholder);

        // Count replacement occurrences
        const replacementAddress = '2222222222222222222222222222222222222222';
        const replacementOccurrences = (linkedBytecode.match(new RegExp(replacementAddress, 'g')) || []).length;
        expect(replacementOccurrences).to.equal(1);
    });

    it('should handle old-style placeholders for library names without colons', () => {
        const simpleLibraryName = 'SimpleLibrary';
        const oldStylePlaceholder = `__SimpleLibrary`.padEnd(40, '_');

        // Create bytecode with old-style placeholder only
        const bytecode = `0x608060405234801561001057600080fd5b5060405161029e38038061029e83398101604081905261002f91610054565b600080546001600160a01b0319166001600160a01b039290921691909117905550610084565b60006020828403121561006657600080fd5b81516001600160a01b038116811461007d57600080fd5b9392505050565b610207806100936000396000f3fe${oldStylePlaceholder}608060405234801561001057600080fd5b5060405161029e38038061029e83398101604081905261002f91610054565b600080546001600160a01b0319166001600160a01b039290921691909117905550610084565b60006020828403121561006657600080fd5b81516001600160a01b038116811461007d57600080fd5b9392505050565b610207806100936000396000f3fe`;

        const libraries = {
            [simpleLibraryName]: '0xabcdefabcdefabcdefabcdefabcdefabcdefabcd',
        };

        // Perform linking
        const linkedBytecode = linkLibrariesInBytecode(bytecode, libraries);

        // Verify old-style placeholder was replaced
        expect(linkedBytecode).to.not.include(oldStylePlaceholder);

        // Count replacement occurrences
        const replacementAddress = 'abcdefabcdefabcdefabcdefabcdefabcdefabcd';
        const replacementOccurrences = (linkedBytecode.match(new RegExp(replacementAddress, 'g')) || []).length;
        expect(replacementOccurrences).to.equal(1);
    });
});
