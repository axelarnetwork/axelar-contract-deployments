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
