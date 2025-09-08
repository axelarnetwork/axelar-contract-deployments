const { expect } = require('chai');
const { getChains } = require('../utils');

describe('getChains', () => {
    let mockConfig;

    beforeEach(() => {
        mockConfig = {
            chains: {
                ethereum: {
                    chainType: 'evm',
                    axelarId: 'ethereum',
                    name: 'Ethereum',
                },
                polygon: {
                    chainType: 'evm',
                    axelarId: 'polygon',
                    name: 'Polygon',
                },
                avalanche: {
                    chainType: 'evm',
                    axelarId: 'avalanche',
                    name: 'Avalanche',
                },
                cosmos: {
                    chainType: 'cosmos',
                    axelarId: 'cosmos',
                    name: 'Cosmos Hub',
                },
            },
        };
    });

    describe('Basic functionality', () => {
        it('should return all EVM chains when chainNames is "all"', () => {
            const result = getChains(mockConfig, 'all', null, null);
            expect(result).to.have.length(3);
            expect(result.map((chain) => chain.axelarId)).to.include.members(['ethereum', 'polygon', 'avalanche']);
        });

        it('should return specific chains when chainNames is a comma-separated string', () => {
            const result = getChains(mockConfig, 'ethereum,polygon', null, null);
            expect(result).to.have.length(2);
            expect(result.map((chain) => chain.axelarId)).to.include.members(['ethereum', 'polygon']);
        });

        it('should return single chain when chainNames is a single chain name', () => {
            const result = getChains(mockConfig, 'ethereum', null, null);
            expect(result).to.have.length(1);
            expect(result[0].axelarId).to.equal('ethereum');
        });
    });

    describe('Chain filtering and validation', () => {
        it('should filter out non-EVM chains when using "all"', () => {
            const result = getChains(mockConfig, 'all', null, null);
            expect(result).to.have.length(3);
            result.forEach((chain) => expect(chain.chainType).to.equal('evm'));
        });

        it('should throw error when chain is not defined in config', () => {
            expect(() => {
                getChains(mockConfig, 'ethereum,unknown-chain', null, null);
            }).to.throw('Chain "unknown-chain" is not defined in the config file');
        });

        it('should throw error when chain is not an EVM chain', () => {
            expect(() => {
                getChains(mockConfig, 'cosmos', null, null);
            }).to.throw('Chain "cosmos" is not an EVM chain');
        });
    });

    describe('Skip chains functionality', () => {
        it('should skip specified chains', () => {
            const result = getChains(mockConfig, 'all', 'ethereum,polygon', null);
            expect(result).to.have.length(1);
            expect(result[0].axelarId).to.equal('avalanche');
        });

        it('should skip single chain', () => {
            const result = getChains(mockConfig, 'all', 'ethereum', null);
            expect(result).to.have.length(2);
            expect(result.map((chain) => chain.axelarId)).to.include.members(['polygon', 'avalanche']);
        });

        it('should throw error when skip chain is not found in chain list', () => {
            expect(() => {
                getChains(mockConfig, 'ethereum', 'polygon', null);
            }).to.throw('Chains to skip "polygon" not found in the list of chains to process');
        });

        it('should handle empty skip chains string', () => {
            const result = getChains(mockConfig, 'all', '', null);
            expect(result).to.have.length(3);
        });

        it('should handle null skip chains', () => {
            const result = getChains(mockConfig, 'all', null, null);
            expect(result).to.have.length(3);
        });
    });

    describe('Start from chain functionality', () => {
        it('should start from specified chain', () => {
            const result = getChains(mockConfig, 'all', null, 'polygon');
            expect(result).to.have.length(2);
            expect(result.map((chain) => chain.axelarId)).to.deep.equal(['polygon', 'avalanche']);
        });

        it('should start from first chain when startFromChain is first in list', () => {
            const result = getChains(mockConfig, 'all', null, 'ethereum');
            expect(result).to.have.length(3);
            expect(result.map((chain) => chain.axelarId)).to.deep.equal(['ethereum', 'polygon', 'avalanche']);
        });

        it('should start from last chain when startFromChain is last in list', () => {
            const result = getChains(mockConfig, 'all', null, 'avalanche');
            expect(result).to.have.length(1);
            expect(result[0].axelarId).to.equal('avalanche');
        });

        it('should throw error when startFromChain is not found in chain list', () => {
            expect(() => {
                getChains(mockConfig, 'ethereum,polygon', null, 'avalanche');
            }).to.throw('Chain to start from "avalanche" not found in the list of chains to process');
        });

        it('should handle null startFromChain', () => {
            const result = getChains(mockConfig, 'all', null, null);
            expect(result).to.have.length(3);
        });
    });

    describe('Edge cases', () => {
        it('should throw error when chain names is empty string', () => {
            expect(() => {
                getChains(mockConfig, '', null, null);
            }).to.throw('Chain "" is not defined in the config file');
        });

        it('should throw error when chain is not an EVM chain', () => {
            expect(() => {
                getChains(mockConfig, 'cosmos', null, null);
            }).to.throw('Chain "cosmos" is not an EVM chain');
        });

        it('should handle config with no chains', () => {
            const emptyConfig = { chains: {} };
            expect(() => {
                getChains(emptyConfig, 'all', null, null);
            }).to.throw('No valid chains found');
        });

        it('should handle config with only non-EVM chains', () => {
            const nonEVMConfig = {
                chains: {
                    cosmos: {
                        chainType: 'cosmos',
                        axelarId: 'cosmos',
                        name: 'Cosmos Hub',
                    },
                },
            };
            expect(() => {
                getChains(nonEVMConfig, 'all', null, null);
            }).to.throw('No valid chains found');
        });
    });

    describe('Combined functionality', () => {
        it('should handle skip chains and start from chain together', () => {
            // When using 'all' and starting from 'polygon', ethereum is not in the list to process
            // So we can't skip it - we need to only skip chains that are actually in the list
            const result = getChains(mockConfig, 'all', 'polygon', 'polygon');
            expect(result).to.have.length(1);
            expect(result[0].axelarId).to.equal('avalanche');
        });

        it('should handle specific chain names with skip and start from', () => {
            // When starting from 'polygon', ethereum is not in the list to process
            // So we can't skip it - we need to only skip chains that are actually in the list
            const result = getChains(mockConfig, 'ethereum,polygon,avalanche', 'polygon', 'polygon');
            expect(result).to.have.length(1);
            expect(result[0].axelarId).to.equal('avalanche');
        });

        it('should handle complex chain filtering scenario', () => {
            const complexConfig = {
                chains: {
                    chain1: { chainType: 'evm', axelarId: 'chain1' },
                    chain2: { chainType: 'evm', axelarId: 'chain2' },
                    chain3: { chainType: 'evm', axelarId: 'chain3' },
                    chain4: { chainType: 'evm', axelarId: 'chain4' },
                    chain5: { chainType: 'evm', axelarId: 'chain5' },
                },
            };

            // Start from chain2, so the list to process is chain2, chain3, chain4, chain5
            // Skip chain3 (which is in the list), result should be chain2, chain4, chain5
            const result = getChains(complexConfig, 'chain1,chain2,chain3,chain4,chain5', 'chain3', 'chain2');
            expect(result).to.have.length(3);
            expect(result.map((chain) => chain.axelarId)).to.deep.equal(['chain2', 'chain4', 'chain5']);
        });
    });

    describe('Return value format', () => {
        it('should return chain objects, not just names', () => {
            const result = getChains(mockConfig, 'ethereum', null, null);
            expect(result[0]).to.be.an('object');
            expect(result[0]).to.have.property('axelarId', 'ethereum');
            expect(result[0]).to.have.property('chainType', 'evm');
            expect(result[0]).to.have.property('name', 'Ethereum');
        });

        it('should preserve all chain properties', () => {
            const customConfig = {
                chains: {
                    custom: {
                        chainType: 'evm',
                        axelarId: 'custom',
                        name: 'Custom Chain',
                        customProperty: 'customValue',
                        nestedProperty: { key: 'value' },
                    },
                },
            };

            const result = getChains(customConfig, 'custom', null, null);
            expect(result[0]).to.have.property('customProperty', 'customValue');
            expect(result[0]).to.have.property('nestedProperty');
            expect(result[0].nestedProperty).to.deep.equal({ key: 'value' });
        });
    });
});
