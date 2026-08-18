'use strict';

const chai = require('chai');
const { expect } = chai;

const { validateAuthModuleOptions, authReadinessProblems, OLD_KEY_RETENTION } = require('../deploy-consensus-gateway');

const AUTH = '0x1111111111111111111111111111111111111111';
const PROXY = '0x2222222222222222222222222222222222222222';
const SEEDER = '0x3333333333333333333333333333333333333333';
const CODEHASH = '0xaaaa';

const ready = (overrides = {}) => ({
    auth: AUTH,
    codehash: CODEHASH,
    expectedCodehash: CODEHASH,
    owner: PROXY,
    proxy: PROXY,
    seeder: SEEDER,
    currentEpoch: 20,
    liveOperatorsEpoch: 20,
    ...overrides,
});

describe('validateAuthModuleOptions', () => {
    it('rejects --authModule without --reuseProxy', () => {
        expect(() => validateAuthModuleOptions({ authModule: AUTH })).to.throw('--authModule requires --reuseProxy');
    });

    it('rejects --authModule combined with --reuseAuth', () => {
        expect(() => validateAuthModuleOptions({ authModule: AUTH, reuseProxy: true, reuseAuth: true })).to.throw(
            '--authModule cannot be combined with --reuseAuth',
        );
    });

    it('rejects --authModule combined with --skipExisting', () => {
        expect(() => validateAuthModuleOptions({ authModule: AUTH, reuseProxy: true, skipExisting: true })).to.throw(
            '--authModule cannot be combined with --skipExisting',
        );
    });

    it('accepts --authModule with --reuseProxy and --reuseHelpers', () => {
        expect(() => validateAuthModuleOptions({ authModule: AUTH, reuseProxy: true, reuseHelpers: true })).to.not.throw();
    });

    it('ignores every other flag combination when --authModule is absent', () => {
        expect(() => validateAuthModuleOptions({ reuseAuth: true, skipExisting: true })).to.not.throw();
    });
});

describe('authReadinessProblems', () => {
    it('reports nothing for an auth owned by the proxy and holding the live set', () => {
        expect(authReadinessProblems(ready())).to.deep.equal([]);
    });

    it('accepts an auth still held by the seeding wallet before the handoff', () => {
        expect(authReadinessProblems(ready({ owner: SEEDER }))).to.deep.equal([]);
    });

    it('compares addresses case insensitively', () => {
        expect(authReadinessProblems(ready({ owner: PROXY.toUpperCase().replace('0X', '0x') }))).to.deep.equal([]);
    });

    it('rejects an owner that is neither the proxy nor the seeder', () => {
        const problems = authReadinessProblems(ready({ owner: '0x4444444444444444444444444444444444444444' }));
        expect(problems).to.have.lengthOf(1);
        expect(problems[0]).to.contain('is neither the gateway proxy');
    });

    it('rejects a codehash that does not match the compiled auth', () => {
        const problems = authReadinessProblems(ready({ codehash: '0xbbbb' }));
        expect(problems).to.have.lengthOf(1);
        expect(problems[0]).to.contain('codehash is 0xbbbb');
    });

    it('rejects an unseeded auth', () => {
        const problems = authReadinessProblems(ready({ currentEpoch: 0, liveOperatorsEpoch: 0 }));
        expect(problems).to.have.lengthOf(1);
        expect(problems[0]).to.contain('unseeded');
    });

    it('rejects an auth that does not hold the live operator set', () => {
        const problems = authReadinessProblems(ready({ liveOperatorsEpoch: 0 }));
        expect(problems).to.have.lengthOf(1);
        expect(problems[0]).to.contain('live operator set is not registered');
    });

    it('rejects a live operator set aged out of the retention window', () => {
        const problems = authReadinessProblems(ready({ currentEpoch: 20, liveOperatorsEpoch: 20 - OLD_KEY_RETENTION }));
        expect(problems).to.have.lengthOf(1);
        expect(problems[0]).to.contain('past OLD_KEY_RETENTION');
    });

    it('accepts a live operator set at the edge of the retention window', () => {
        expect(authReadinessProblems(ready({ currentEpoch: 20, liveOperatorsEpoch: 20 - OLD_KEY_RETENTION + 1 }))).to.deep.equal([]);
    });

    it('reports every problem at once', () => {
        expect(authReadinessProblems(ready({ codehash: '0xbbbb', owner: AUTH, currentEpoch: 0 }))).to.have.lengthOf(3);
    });
});
