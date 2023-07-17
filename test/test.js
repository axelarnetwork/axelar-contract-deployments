'use strict';

const chai = require('chai');
const { expect } = chai;
const { ethers } = require('hardhat');
const {
  deployCreate3Contract,
  deployCreate3AndInitContract,
  getCreate3Address,
} = require('../index.js');
const { deployConstAddressDeployer } = require('../evm/deploy-const-address-deployer.js');
const { keccak256 } = require('ethers/lib/utils.js');

describe('Contracts deployments', () => {
    let wallet;

    before(async () => {
        [ wallet ] = await ethers.getSigners();
    });

    describe('ConstAddressDeployer', () => {
        const privateKey = keccak256('0x1234');
        const chain = {
            contracts: {},
            tokenSymbol: 'ETH',
            name: 'Ethereum',
        }
        it('Should deploy the ConstAddressDeployer', async() => {
            await deployConstAddressDeployer(wallet, chain, privateKey);
            expect(chain.contracts.ConstAddressDeployer.address).to.not.equal(undefined);
            expect(chain.contracts.ConstAddressDeployer.deployer).to.not.equal(undefined);

        });
    });
});