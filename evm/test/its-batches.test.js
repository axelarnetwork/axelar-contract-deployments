const { expect } = require('chai');
const fs = require('fs');
const os = require('os');
const path = require('path');
const vm = require('vm');
const { createRequire } = require('module');
const { ethers } = require('hardhat');
const utils = require('../utils');
const governance = require('../governance');

const sourcePath = path.resolve(__dirname, '../its.js');
const realRequire = createRequire(sourcePath);
const iface = new ethers.utils.Interface(['function multicall(bytes[]) returns(bytes[])', 'function migrateInterchainToken(bytes32)']);
const address = (n) => ethers.utils.getAddress(ethers.utils.hexZeroPad(ethers.utils.hexlify(n), 20));
const tokenId = (n) => ethers.utils.hexZeroPad(ethers.utils.hexlify(n), 32);

describe('ITS per-chain migration proposals', () => {
    let directory;
    let manifest;
    let config;
    let submissions;
    let simulations;
    let estimates;
    let failure;
    let skip;
    let migrate;

    beforeEach(() => {
        directory = fs.mkdtempSync(path.join(os.tmpdir(), 'its-batches-'));
        manifest = path.join(directory, 'batches.json');
        config = { chains: {} };
        ['alpha', 'beta'].forEach((name, i) => {
            config.chains[name] = {
                name,
                chainType: 'evm',
                axelarId: `${name}-destination`,
                contracts: {
                    InterchainTokenService: { address: address(i + 1) },
                    AxelarServiceGovernance: { address: address(i + 11) },
                },
            };
        });
        submissions = [];
        simulations = [];
        estimates = [];
        failure = undefined;
        skip = undefined;
        class MockContract {
            constructor(target) {
                this.interface = iface;
                this.callStatic = {
                    multicall: async (calls, options) => {
                        simulations.push({ target, calls, options });
                        if (failure === target) {
                            throw new Error('Simulation reverted');
                        }
                    },
                };
                this.estimateGas = {
                    multicall: async (calls, options) => {
                        estimates.push({ target, calls, options });
                        return ethers.BigNumber.from(100000);
                    },
                };
            }
            connect() {
                return this;
            }
        }
        const replacements = {
            hardhat: { ethers: { ...ethers, Contract: MockContract, getDefaultProvider: () => ({}) } },
            './utils': {
                ...utils,
                loadConfig: () => config,
                printInfo: () => {},
                printWalletInfo: async () => ({ address: address(100) }),
                getGasOptions: async () => ({ gasLimit: 8000000 }),
                mainProcessor: async (options, callback, save) => {
                    expect(save).to.equal(false);
                    for (const name of options.chainNames.split(',')) {
                        if (name !== skip) {
                            await callback({}, config.chains[name], config.chains, options);
                        }
                    }
                },
            },
            './sign-utils': { getWallet: async () => ({}) },
            './governance': { ...governance, submitGatewayProposalToAxelar: async (proposal) => submissions.push(proposal) },
        };
        const sandbox = {
            require: (name) => replacements[name] || realRequire(name),
            module: { exports: {} },
            Buffer,
        };
        vm.runInNewContext(fs.readFileSync(sourcePath, 'utf8'), sandbox, { filename: sourcePath });
        migrate = sandbox.module.exports.migrateInterchainTokenBatches;
    });

    afterEach(() => fs.rmSync(directory, { recursive: true, force: true }));

    async function run(batches, options = {}) {
        fs.writeFileSync(manifest, JSON.stringify(batches));
        return migrate(manifest, {
            env: 'testnet',
            governanceContract: 'AxelarServiceGovernance',
            operatorProposal: true,
            ...options,
        });
    }

    async function rejects(batches, message) {
        let error;
        try {
            await run(batches);
        } catch (caught) {
            error = caught;
        }
        expect(error?.message).to.include(message);
        expect(submissions).to.have.length(0);
    }

    it('submits once with different destinations and batches, allowing an ID on both chains', async () => {
        const batches = { alpha: [tokenId(1), tokenId(2)], beta: [tokenId(2), tokenId(3), tokenId(4)] };
        await run(batches);
        expect(submissions).to.have.length(1);
        expect(submissions[0].msgs).to.have.length(2);
        expect(simulations).to.have.length(2);
        expect(estimates).to.have.length(2);
        Object.entries(batches).forEach(([name, ids], i) => {
            const call = submissions[0].msgs[i].call_contract;
            expect(call.destination_chain).to.equal(config.chains[name].axelarId);
            expect(call.destination_address).to.equal(address(i + 11));
            const [command, target, data, value, eta] = ethers.utils.defaultAbiCoder.decode(
                ['uint256', 'address', 'bytes', 'uint256', 'uint256'],
                `0x${call.payload}`,
            );
            expect(command.toNumber()).to.equal(2);
            expect(target).to.equal(address(i + 1));
            expect(value.toNumber()).to.equal(0);
            expect(eta.toNumber()).to.equal(0);
            const calls = iface.decodeFunctionData('multicall', data)[0];
            expect(calls.map((data) => iface.decodeFunctionData('migrateInterchainToken', data)[0])).to.deep.equal(ids);
            expect(simulations[i].options.from).to.equal(address(i + 11));
            expect(simulations[i].options.gasLimit).to.equal(undefined);
        });
    });

    it('generates one preview without calling the submitter', async () => {
        const output = path.join(directory, 'proposal.json');
        await run({ alpha: [tokenId(1)], beta: [tokenId(2)] }, { generateOnly: output });
        expect(submissions).to.have.length(0);
        expect(JSON.parse(fs.readFileSync(output)).msgs).to.have.length(2);
    });

    it('does not submit a partial proposal when the second chain simulation fails', async () => {
        failure = address(2);
        await rejects({ alpha: [tokenId(1)], beta: [tokenId(2)] }, 'Simulation reverted');
        expect(simulations).to.have.length(2);
    });

    it('rejects incomplete preparation even if a chain is silently skipped', async () => {
        skip = 'beta';
        await rejects({ alpha: [tokenId(1)], beta: [tokenId(2)] }, 'Every chain must produce');
    });

    it('validates every chain before beginning simulations', async () => {
        await rejects({ alpha: [tokenId(1)], unknown: [tokenId(2)] }, 'Unknown EVM chain');
        expect(simulations).to.have.length(0);
    });

    it('rejects duplicate IDs within a batch without treating cross-chain IDs as duplicates', async () => {
        await rejects({ alpha: [tokenId(1), tokenId(1)] }, 'unique token IDs');
    });

    it('rejects aliases targeting the same destination', async () => {
        config.chains.beta.axelarId = config.chains.alpha.axelarId;
        await rejects({ alpha: [tokenId(1)], beta: [tokenId(2)] }, 'Duplicate destination');
    });

    it('accepts 400 IDs per chain and rejects 401 before any simulation', async () => {
        await run({ alpha: Array.from({ length: 400 }, (_, i) => tokenId(i)) });
        expect(submissions).to.have.length(1);
        submissions.length = 0;
        simulations.length = 0;
        await rejects({ alpha: Array.from({ length: 401 }, (_, i) => tokenId(i)) }, 'between 1 and 400');
        expect(simulations).to.have.length(0);
    });

    for (const input of [null, [], {}, { alpha: [] }, { alpha: 'not-an-array' }, { alpha: ['0x1234'] }]) {
        it(`rejects invalid manifest ${JSON.stringify(input)}`, async () => {
            await rejects(input, '');
            expect(simulations).to.have.length(0);
        });
    }
});
