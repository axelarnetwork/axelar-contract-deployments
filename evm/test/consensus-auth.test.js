'use strict';

const { expect } = require('chai');
const { ethers } = require('hardhat');
const {
    utils: { defaultAbiCoder, keccak256 },
    BigNumber,
} = ethers;

const {
    resolveAuthAddress,
    blocksPerDay,
    selectSetsToSeed,
    recentOperatorSets,
    encodeOperatorSet,
    operatorsHash,
    OLD_KEY_RETENTION,
} = require('../consensus-auth');

// chai-as-promised is not a dependency here, so assert rejections directly
const expectRejection = async (promise, message) => {
    try {
        await promise;
    } catch (error) {
        expect(error.message).to.contain(message);
        return;
    }

    throw new Error(`expected a rejection containing "${message}"`);
};

const AUTH = '0x1111111111111111111111111111111111111111';
const OPERATORS = ['0x00000000000000000000000000000000000000a1', '0x00000000000000000000000000000000000000b2'];
const WEIGHTS = [1, 2];
const THRESHOLD = 2;

// stands in for an auth module that already knows the given encoded sets
const fakeAuth = (registered = []) => ({
    epochForHash: async (hash) => BigNumber.from(registered.some((set) => operatorsHash(set) === hash) ? 1 : 0),
});

describe('resolveAuthAddress', () => {
    it('rejects --salt and --authModule together', async () => {
        await expectRejection(
            resolveAuthAddress(null, null, { salt: 'v6.5.0', authModule: AUTH }),
            'Provide exactly one of --salt or --authModule',
        );
    });

    it('rejects neither being provided', async () => {
        await expectRejection(resolveAuthAddress(null, null, {}), 'Provide exactly one of --salt or --authModule');
    });

    it('returns --authModule without deriving an address', async () => {
        expect(await resolveAuthAddress(null, null, { authModule: AUTH })).to.equal(AUTH);
    });
});

describe('encodeOperatorSet', () => {
    it('round trips through the encoding the auth module decodes', () => {
        const encoded = encodeOperatorSet(OPERATORS, WEIGHTS, THRESHOLD);
        const [operators, weights, threshold] = defaultAbiCoder.decode(['address[]', 'uint256[]', 'uint256'], encoded);

        expect(operators.map((operator) => operator.toLowerCase())).to.deep.equal(OPERATORS);
        expect(weights.map((weight) => weight.toNumber())).to.deep.equal(WEIGHTS);
        expect(threshold.toNumber()).to.equal(THRESHOLD);
    });

    it('hashes the way _transferOperatorship does', () => {
        const encoded = encodeOperatorSet(OPERATORS, WEIGHTS, THRESHOLD);

        expect(operatorsHash(encoded)).to.equal(
            keccak256(defaultAbiCoder.encode(['address[]', 'uint256[]', 'uint256'], [OPERATORS, WEIGHTS, THRESHOLD])),
        );
    });
});

describe('selectSetsToSeed', () => {
    const a = encodeOperatorSet(OPERATORS, [1, 1], 2);
    const b = encodeOperatorSet(OPERATORS, [1, 2], 2);
    const c = encodeOperatorSet(OPERATORS, [1, 3], 2);

    it('keeps every unknown set in the order given', async () => {
        expect(await selectSetsToSeed(fakeAuth(), [a, b, c])).to.deep.equal([a, b, c]);
    });

    it('skips sets the auth module already holds', async () => {
        expect(await selectSetsToSeed(fakeAuth([b]), [a, b, c])).to.deep.equal([a, c]);
    });

    it('drops duplicates within the candidate list', async () => {
        expect(await selectSetsToSeed(fakeAuth(), [a, b, a])).to.deep.equal([a, b]);
    });

    it('returns nothing when every set is already registered', async () => {
        expect(await selectSetsToSeed(fakeAuth([a, b]), [a, b])).to.deep.equal([]);
    });
});

describe('recentOperatorSets', () => {
    it('copies nothing when asked for zero epochs', async () => {
        expect(await recentOperatorSets(null, null, { copyEpochs: '0' })).to.deep.equal([]);
    });

    it('rejects a count at or beyond the retention window', async () => {
        await expectRejection(recentOperatorSets(null, null, { copyEpochs: String(OLD_KEY_RETENTION) }), '--copyEpochs must be');
    });

    it('rejects a negative count', async () => {
        await expectRejection(recentOperatorSets(null, null, { copyEpochs: '-1' }), '--copyEpochs must be');
    });

    it('rejects a non integer count', async () => {
        await expectRejection(recentOperatorSets(null, null, { copyEpochs: 'two' }), '--copyEpochs must be');
    });
});

describe('blocksPerDay', () => {
    // block n is mined at n * seconds, so the sampled average is exactly `seconds`
    const providerWithBlockTime = (seconds) => ({ getBlock: async (number) => ({ timestamp: number * seconds }) });

    it('derives a day of 12 second blocks', async () => {
        expect(await blocksPerDay(providerWithBlockTime(12), 20000)).to.equal(7200);
    });

    it('derives a day of 2 second blocks', async () => {
        expect(await blocksPerDay(providerWithBlockTime(2), 20000)).to.equal(43200);
    });

    it('samples a short chain without reading a negative block number', async () => {
        expect(await blocksPerDay(providerWithBlockTime(12), 500)).to.equal(7200);
    });

    it('rejects a chain with no history', async () => {
        await expectRejection(blocksPerDay(providerWithBlockTime(12), 0), 'no history');
    });

    it('rejects a non advancing timestamp rather than guessing', async () => {
        await expectRejection(blocksPerDay({ getBlock: async () => ({ timestamp: 1 }) }, 20000), 'non positive block time');
    });
});
