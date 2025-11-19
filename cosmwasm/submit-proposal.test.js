'use strict';

const { expect } = require('chai');

describe('instantiateChainContracts', () => {
    let mockClient;
    let mockConfig;
    let mockOptions;
    let mockFee;
    let originalExecute;
    let originalGetCodeId;
    let originalPrompt;
    let originalSubmitProposal;
    let originalPrintInfo;
    let executeCallCount;
    let executeCallArgs;

    const createMockConfig = () => {
        const mockGatewayConfig = {
            codeId: 1,
            contractAdmin: 'axelar1admin',
        };
        const mockVotingVerifierConfig = {
            codeId: 2,
            contractAdmin: 'axelar1admin',
            governanceAddress: 'axelar1governance',
            serviceName: 'service',
            sourceGatewayAddress: 'axelar1sourcegateway',
            votingThreshold: ['1', '2'],
            blockExpiry: 100,
            confirmationHeight: 10,
            msgIdFormat: 'hex',
            addressFormat: 'hex',
        };
        const mockMultisigProverConfig = {
            codeId: 3,
            contractAdmin: 'axelar1admin',
            governanceAddress: 'axelar1governance',
            adminAddress: 'axelar1admin',
            encoder: 'abi',
            keyType: 'secp256k1',
            verifierSetDiffThreshold: 1,
            signingThreshold: ['1', '2'],
        };

        const mockChainConfig = {
            chainType: 'evm',
            axelarId: 'ethereum',
            name: 'Ethereum',
        };

        const contracts = {
            Coordinator: {
                address: 'axelar1coordinator',
                deployments: {},
            },
            Rewards: {
                address: 'axelar1rewards',
            },
            Multisig: {
                address: 'axelar1multisig',
            },
            Router: {
                address: 'axelar1router',
            },
        };

        return {
            axelar: {
                contracts,
                chainId: 'axelar-dojo-1',
                cosmosSDK: 'v0.50.0',
            },
            chains: {
                ethereum: mockChainConfig,
            },
            getChainConfig: (chainName) => {
                if (chainName === 'ethereum') return mockChainConfig;
                throw new Error(`Chain ${chainName} not found`);
            },
            getContractConfig: (contractName) => {
                const contract = contracts[contractName];
                if (!contract) {
                    throw new Error(`Contract '${contractName}' not found in config`);
                }
                return contract;
            },
            getMultisigProverContractForChainType: () => 'MultisigProver',
            getVotingVerifierContractForChainType: () => 'VotingVerifier',
            getGatewayContractForChainType: () => 'Gateway',
            getGatewayContract: (chainName) => {
                if (chainName === 'ethereum') return mockGatewayConfig;
                throw new Error(`Gateway config for ${chainName} not found`);
            },
            getVotingVerifierContract: (chainName) => {
                if (chainName === 'ethereum') return mockVotingVerifierConfig;
                throw new Error(`VotingVerifier config for ${chainName} not found`);
            },
            getMultisigProverContract: (chainName) => {
                if (chainName === 'ethereum') return mockMultisigProverConfig;
                throw new Error(`MultisigProver config for ${chainName} not found`);
            },
            validateRequired: (value, message) => {
                if (!value && value !== 0) {
                    throw new Error(message);
                }
                return value;
            },
        };
    };

    beforeEach(() => {
        mockClient = { accounts: [{ address: 'axelar1test' }] };
        mockConfig = createMockConfig();
        mockOptions = {
            chainName: 'ethereum',
            salt: 'testsalt',
            admin: 'axelar1admin',
            gatewayCodeId: undefined,
            verifierCodeId: undefined,
            proverCodeId: undefined,
            fetchCodeId: false,
            yes: true,
        };
        mockFee = {
            amount: [{ denom: 'uaxl', amount: '1000' }],
            gas: '200000',
        };
        executeCallCount = 0;
        executeCallArgs = [];
    });

    const getMockedModule = () => {
        // Pre-load common modules before clearing cache to ensure TypeScript files are loaded with ts-node
        require('../common/utils');
        require('../common');

        delete require.cache[require.resolve('./submit-proposal')];
        delete require.cache[require.resolve('./utils')];
        // Don't clear common cache to avoid issues with TypeScript files that need ts-node

        const commonUtils = require('../common/utils');
        originalPrompt = commonUtils.prompt;
        originalPrintInfo = commonUtils.printInfo;
        commonUtils.prompt = () => false;
        commonUtils.printInfo = () => {};

        const utils = require('./utils');
        originalGetCodeId = utils.getCodeId;
        originalSubmitProposal = utils.submitProposal;
        utils.getCodeId = async (client, config, options) => {
            const { GATEWAY_CONTRACT_NAME, VERIFIER_CONTRACT_NAME } = require('../common/config');
            if (options.contractName === GATEWAY_CONTRACT_NAME) return 100;
            if (options.contractName === VERIFIER_CONTRACT_NAME) return 200;
            if (options.contractName === 'MultisigProver') return 300;
            throw new Error(`Unknown contract: ${options.contractName}`);
        };
        utils.submitProposal = async () => 'proposal-123';

        const submitProposal = require('./submit-proposal');
        originalExecute = submitProposal.execute;
        submitProposal.execute = async (...args) => {
            executeCallCount++;
            executeCallArgs.push(args);
            return 'proposal-123';
        };

        return submitProposal;
    };

    afterEach(() => {
        if (originalExecute) {
            const submitProposal = require('./submit-proposal');
            submitProposal.execute = originalExecute;
        }
        if (originalGetCodeId) {
            const utils = require('./utils');
            utils.getCodeId = originalGetCodeId;
        }
        if (originalPrompt) {
            const commonUtils = require('../common/utils');
            commonUtils.prompt = originalPrompt;
        }
        if (originalSubmitProposal) {
            const utils = require('./utils');
            utils.submitProposal = originalSubmitProposal;
        }
        if (originalPrintInfo) {
            const commonUtils = require('../common/utils');
            commonUtils.printInfo = originalPrintInfo;
        }
        delete require.cache[require.resolve('./submit-proposal')];
        delete require.cache[require.resolve('./utils')];
        // Don't clear common cache to avoid issues with TypeScript files that need to be built
    });

    describe('Error handling', () => {
        it('should throw error when Coordinator address is missing', async () => {
            mockConfig.axelar.contracts.Coordinator.address = undefined;
            const { instantiateChainContracts } = getMockedModule();

            await expect(instantiateChainContracts(mockClient, mockConfig, mockOptions, undefined, mockFee)).to.be.rejectedWith(
                'Coordinator contract address not found in config',
            );
        });

        it('should throw error when Coordinator contract is missing', async () => {
            mockConfig.axelar.contracts.Coordinator = undefined;
            const { instantiateChainContracts } = getMockedModule();

            await expect(instantiateChainContracts(mockClient, mockConfig, mockOptions, undefined, mockFee)).to.be.rejectedWith(
                'Coordinator contract address not found in config',
            );
        });

        it('should throw error when admin is missing', async () => {
            mockOptions.admin = undefined;
            const { instantiateChainContracts } = getMockedModule();

            await expect(instantiateChainContracts(mockClient, mockConfig, mockOptions, undefined, mockFee)).to.be.rejectedWith(
                'Admin address is required when instantiating chain contracts',
            );
        });

        it('should throw error when admin is empty string', async () => {
            mockOptions.admin = '';
            const { instantiateChainContracts } = getMockedModule();

            await expect(instantiateChainContracts(mockClient, mockConfig, mockOptions, undefined, mockFee)).to.be.rejectedWith(
                'Admin address is required when instantiating chain contracts',
            );
        });

        it('should throw error when salt is missing', async () => {
            mockOptions.salt = undefined;
            const { instantiateChainContracts } = getMockedModule();

            await expect(instantiateChainContracts(mockClient, mockConfig, mockOptions, undefined, mockFee)).to.be.rejectedWith(
                'Salt is required when instantiating chain contracts',
            );
        });

        it('should throw error when salt is empty string', async () => {
            mockOptions.salt = '';
            const { instantiateChainContracts } = getMockedModule();

            await expect(instantiateChainContracts(mockClient, mockConfig, mockOptions, undefined, mockFee)).to.be.rejectedWith(
                'Salt is required when instantiating chain contracts',
            );
        });

        it('should throw error when Gateway code ID is missing and fetchCodeId is false', async () => {
            mockConfig.getGatewayContract('ethereum').codeId = undefined;
            mockOptions.gatewayCodeId = undefined;
            mockOptions.fetchCodeId = false;
            const { instantiateChainContracts } = getMockedModule();

            await expect(instantiateChainContracts(mockClient, mockConfig, mockOptions, undefined, mockFee)).to.be.rejectedWith(
                'No Gateway code ID found. Use --gatewayCodeId or fetch the code ID from the network with --fetchCodeId',
            );
        });

        it('should throw error when VotingVerifier code ID is missing and fetchCodeId is false', async () => {
            mockConfig.getVotingVerifierContract('ethereum').codeId = undefined;
            mockOptions.verifierCodeId = undefined;
            mockOptions.fetchCodeId = false;
            const { instantiateChainContracts } = getMockedModule();

            await expect(instantiateChainContracts(mockClient, mockConfig, mockOptions, undefined, mockFee)).to.be.rejectedWith(
                'No VotingVerifier code ID found. Use --verifierCodeId or fetch the code ID from the network with --fetchCodeId',
            );
        });

        it('should throw error when MultisigProver code ID is missing and fetchCodeId is false', async () => {
            mockConfig.getMultisigProverContract('ethereum').codeId = undefined;
            mockOptions.proverCodeId = undefined;
            mockOptions.fetchCodeId = false;
            const { instantiateChainContracts } = getMockedModule();

            await expect(instantiateChainContracts(mockClient, mockConfig, mockOptions, undefined, mockFee)).to.be.rejectedWith(
                'No MultisigProver code ID found. Use --proverCodeId or fetch the code ID from the network with --fetchCodeId',
            );
        });
    });

    describe('Code ID handling', () => {
        it('should use provided gatewayCodeId when fetchCodeId is false', async () => {
            mockOptions.gatewayCodeId = 10;
            mockOptions.fetchCodeId = false;
            const { instantiateChainContracts } = getMockedModule();

            await instantiateChainContracts(mockClient, mockConfig, mockOptions, undefined, mockFee);

            expect(mockConfig.getGatewayContract('ethereum').codeId).to.equal(10);
            expect(mockConfig.getVotingVerifierContract('ethereum').codeId).to.equal(2);
            expect(mockConfig.getMultisigProverContract('ethereum').codeId).to.equal(3);
        });

        it('should use provided verifierCodeId when fetchCodeId is false', async () => {
            mockOptions.verifierCodeId = 20;
            mockOptions.fetchCodeId = false;
            const { instantiateChainContracts } = getMockedModule();

            await instantiateChainContracts(mockClient, mockConfig, mockOptions, undefined, mockFee);

            expect(mockConfig.getGatewayContract('ethereum').codeId).to.equal(1);
            expect(mockConfig.getVotingVerifierContract('ethereum').codeId).to.equal(20);
            expect(mockConfig.getMultisigProverContract('ethereum').codeId).to.equal(3);
        });

        it('should use provided proverCodeId when fetchCodeId is false', async () => {
            mockOptions.proverCodeId = 30;
            mockOptions.fetchCodeId = false;
            const { instantiateChainContracts } = getMockedModule();

            await instantiateChainContracts(mockClient, mockConfig, mockOptions, undefined, mockFee);

            expect(mockConfig.getGatewayContract('ethereum').codeId).to.equal(1);
            expect(mockConfig.getVotingVerifierContract('ethereum').codeId).to.equal(2);
            expect(mockConfig.getMultisigProverContract('ethereum').codeId).to.equal(30);
        });

        it('should use config code IDs when fetchCodeId is false and no code IDs provided', async () => {
            mockOptions.fetchCodeId = false;
            const { instantiateChainContracts } = getMockedModule();

            await instantiateChainContracts(mockClient, mockConfig, mockOptions, undefined, mockFee);

            expect(mockConfig.getGatewayContract('ethereum').codeId).to.equal(1);
            expect(mockConfig.getVotingVerifierContract('ethereum').codeId).to.equal(2);
            expect(mockConfig.getMultisigProverContract('ethereum').codeId).to.equal(3);
        });

        it('should fetch code IDs when fetchCodeId is true', async () => {
            mockOptions.fetchCodeId = true;
            mockConfig.getGatewayContract('ethereum').codeId = undefined;
            const { instantiateChainContracts } = getMockedModule();

            await instantiateChainContracts(mockClient, mockConfig, mockOptions, undefined, mockFee);

            expect(mockConfig.getGatewayContract('ethereum').codeId).to.equal(100);
            expect(mockConfig.getVotingVerifierContract('ethereum').codeId).to.equal(200);
            expect(mockConfig.getMultisigProverContract('ethereum').codeId).to.equal(300);
        });

        it('should prefer provided code IDs over fetching when fetchCodeId is true', async () => {
            mockOptions.fetchCodeId = true;
            mockOptions.gatewayCodeId = 1000;
            mockOptions.verifierCodeId = 2000;
            mockOptions.proverCodeId = 3000;
            const { instantiateChainContracts } = getMockedModule();

            await instantiateChainContracts(mockClient, mockConfig, mockOptions, undefined, mockFee);

            expect(mockConfig.getGatewayContract('ethereum').codeId).to.equal(1000);
            expect(mockConfig.getVotingVerifierContract('ethereum').codeId).to.equal(2000);
            expect(mockConfig.getMultisigProverContract('ethereum').codeId).to.equal(3000);
        });
    });

    describe('Successful execution', () => {
        it('should save deployment info to config', async () => {
            const { instantiateChainContracts } = getMockedModule();

            await instantiateChainContracts(mockClient, mockConfig, mockOptions, undefined, mockFee);

            expect(mockConfig.axelar.contracts.Coordinator.deployments).to.have.property('ethereum');
            const deployment = mockConfig.axelar.contracts.Coordinator.deployments.ethereum;
            expect(deployment).to.have.property('deploymentName');
            expect(deployment).to.have.property('salt', 'testsalt');
            expect(deployment).to.have.property('proposalId', 'proposal-123');
        });

        it('should initialize deployments object if it does not exist', async () => {
            delete mockConfig.axelar.contracts.Coordinator.deployments;
            const { instantiateChainContracts } = getMockedModule();

            await instantiateChainContracts(mockClient, mockConfig, mockOptions, undefined, mockFee);

            expect(mockConfig.axelar.contracts.Coordinator.deployments).to.be.an('object');
            expect(mockConfig.axelar.contracts.Coordinator.deployments).to.have.property('ethereum');
        });

        it('should preserve existing deployments when adding new one', async () => {
            mockConfig.axelar.contracts.Coordinator.deployments = {
                polygon: {
                    deploymentName: 'polygon-1-2-3',
                    salt: 'polygonsalt',
                    proposalId: 'proposal-456',
                },
            };
            const { instantiateChainContracts } = getMockedModule();

            await instantiateChainContracts(mockClient, mockConfig, mockOptions, undefined, mockFee);

            expect(mockConfig.axelar.contracts.Coordinator.deployments).to.have.property('polygon');
            expect(mockConfig.axelar.contracts.Coordinator.deployments).to.have.property('ethereum');
            expect(mockConfig.axelar.contracts.Coordinator.deployments.polygon.deploymentName).to.equal('polygon-1-2-3');
        });
    });

    describe('Edge cases', () => {
        it('should handle null fee', async () => {
            const { instantiateChainContracts } = getMockedModule();

            await instantiateChainContracts(mockClient, mockConfig, mockOptions, undefined, null);

            expect(mockConfig.axelar.contracts.Coordinator.deployments).to.have.property('ethereum');
        });

        it('should handle undefined fee', async () => {
            const { instantiateChainContracts } = getMockedModule();

            await instantiateChainContracts(mockClient, mockConfig, mockOptions, undefined, undefined);

            expect(mockConfig.axelar.contracts.Coordinator.deployments).to.have.property('ethereum');
        });
    });
});
