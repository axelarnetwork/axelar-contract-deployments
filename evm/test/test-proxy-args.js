'use strict';

const { expect } = require('chai');
const { getConstructorArgs } = require('../deploy-contract');

describe('ERC1967Proxy argument handling', () => {
    const mockWallet = {
        address: '0x1234567890123456789012345678901234567890',
    };
    const mockChain = {
        contracts: {
            AxelarTransceiver: {
                address: '0xabcdefabcdefabcdefabcdefabcdefabcdefabcd',
            },
            AxelarGateway: {
                address: '0x1111111111111111111111111111111111111111',
            },
            AxelarGasService: {
                address: '0x2222222222222222222222222222222222222222',
            },
        },
    };

    it('should use forContract option and default proxyData', async () => {
        const args = await getConstructorArgs('ERC1967Proxy', mockChain.contracts, mockChain.contracts.AxelarTransceiver, mockWallet, {
            forContract: 'AxelarTransceiver',
            proxyData: '0x',
        });
        expect(args).to.deep.equal([mockChain.contracts.AxelarTransceiver.address, '0x']);
    });

    it('should use forContract option and custom proxyData', async () => {
        const args = await getConstructorArgs('ERC1967Proxy', mockChain.contracts, mockChain.contracts.AxelarTransceiver, mockWallet, {
            forContract: 'AxelarTransceiver',
            proxyData: '0x12345678',
        });
        expect(args).to.deep.equal([mockChain.contracts.AxelarTransceiver.address, '0x12345678']);
    });

    it('should use explicit args', async () => {
        const args = await getConstructorArgs('ERC1967Proxy', mockChain.contracts, mockChain.contracts.AxelarTransceiver, mockWallet, {
            args: JSON.stringify(['0x3333333333333333333333333333333333333333', '0x12345678']),
        });
        expect(args).to.deep.equal(['0x3333333333333333333333333333333333333333', '0x12345678']);
    });

    it('should throw if forContract does not exist', async () => {
        try {
            await getConstructorArgs('ERC1967Proxy', mockChain.contracts, mockChain.contracts.AxelarTransceiver, mockWallet, {
                forContract: 'NonExistentContract',
            });
            expect.fail('Should have thrown an error');
        } catch (error) {
            expect(error.message).to.include('NonExistentContract');
        }
    });

    it('should throw if no args provided and forContract is missing', async () => {
        try {
            await getConstructorArgs('ERC1967Proxy', mockChain.contracts, mockChain.contracts.AxelarTransceiver, mockWallet, {});
            expect.fail('Should have thrown an error');
        } catch (error) {
            expect(error.message).to.include('requires implementation address');
        }
    });
});

describe('ERC1967Proxy error handling', () => {
    const mockWallet = {
        address: '0x1234567890123456789012345678901234567890',
    };
    const mockChain = {
        contracts: {
            AxelarTransceiver: {
                address: '0xabcdefabcdefabcdefabcdefabcdefabcdefabcd',
            },
            AxelarGateway: {
                address: '0x1111111111111111111111111111111111111111',
            },
            AxelarGasService: {
                address: '0x2222222222222222222222222222222222222222',
            },
        },
    };

    it('should throw error when forContract is missing', async () => {
        try {
            await getConstructorArgs('ERC1967Proxy', mockChain.contracts, mockChain.contracts.AxelarTransceiver, mockWallet, {});
            expect.fail('Should have thrown an error');
        } catch (error) {
            expect(error.message).to.include('ERC1967Proxy requires implementation address and init data');
        }
    });

    it('should throw error when forContract does not exist in config', async () => {
        try {
            await getConstructorArgs('ERC1967Proxy', mockChain.contracts, mockChain.contracts.AxelarTransceiver, mockWallet, {
                forContract: 'NonExistentContract',
            });
            expect.fail('Should have thrown an error');
        } catch (error) {
            expect(error.message).to.include('Proxy for NonExistentContract requires implementation address to be present in the config');
        }
    });

    it('should handle undefined contractConfig gracefully', async () => {
        try {
            await getConstructorArgs('ERC1967Proxy', mockChain.contracts, undefined, mockWallet, {
                forContract: 'AxelarTransceiver',
            });
            expect.fail('Should have thrown an error');
        } catch (error) {
            expect(error.message).to.include('Contract configuration is undefined for ERC1967Proxy');
        }
    });

    it('should work correctly when forContract exists and has address', async () => {
        const args = await getConstructorArgs('ERC1967Proxy', mockChain.contracts, mockChain.contracts.AxelarTransceiver, mockWallet, {
            forContract: 'AxelarTransceiver',
            proxyData: '0x',
        });
        expect(args).to.deep.equal([mockChain.contracts.AxelarTransceiver.address, '0x']);
    });

    it('should work correctly with explicit args', async () => {
        const args = await getConstructorArgs('ERC1967Proxy', mockChain.contracts, mockChain.contracts.AxelarTransceiver, mockWallet, {
            args: JSON.stringify(['0x3333333333333333333333333333333333333333', '0x12345678']),
        });
        expect(args).to.deep.equal(['0x3333333333333333333333333333333333333333', '0x12345678']);
    });
});
