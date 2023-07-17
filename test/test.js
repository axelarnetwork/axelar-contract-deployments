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
const { deployCreate3Deployer } = require('../evm/deploy-create3-deployer.js');
const { deployITS } = require('../evm/deploy-its.js');
const { deployGatewayv5 } = require('../evm/deploy-gateway-v5.x.js');
const { Wallet } = ethers

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


    describe('Create3Deployer', () => {
        const privateKey = keccak256('0x123456');
        const chain = {
            contracts: {},
            tokenSymbol: 'ETH',
            name: 'Ethereum',
        }

        before(async () => {
            await deployConstAddressDeployer(wallet, chain, privateKey);
        })
        it('Should deploy the ConstAddressDeployer', async() => {
            await deployCreate3Deployer(wallet, chain);
            const contract = chain.contracts.Create3Deployer;
            console.log(contract);
            expect(contract.address).to.not.equal(undefined);
            expect(contract.salt).to.equal('Create3Deployer');
            expect(contract.deployer).to.equal(wallet.address);

        });
    });

    describe('Interchain Token Service', () => {
        const privateKey = keccak256('0x12345678');
        const chain = {
            contracts: {},
            tokenSymbol: 'ETH',
            name: 'Ethereum',
        };
        const deploymentKey = 'testKey';
        before(async () => {
            await deployConstAddressDeployer(wallet, chain, privateKey);
            await deployCreate3Deployer(wallet, chain, privateKey);  
            chain.contracts.AxelarGateway = {
                address: (new Wallet(keccak256('0x12345678'))).address
            }
            chain.contracts.AxelarGasService = {
                address: (new Wallet(keccak256('0x12345679'))).address
            }
        })
        it('Should deploy the ConstAddressDeployer', async() => {
            await deployITS(wallet, chain, deploymentKey, wallet.address);
            const contract = chain.contracts.InterchainTokenService;
            console.log(contract);
            expect(contract.salt).to.equal(deploymentKey);
            expect(contract.deployer).to.equal(wallet.address);
            expect(contract.tokenManagerDeployer).to.not.equal(undefined);
            expect(contract.standardizedTokenLockUnlock).to.not.equal(undefined);
            expect(contract.standardizedTokenMintBurn).to.not.equal(undefined);
            expect(contract.standardizedTokenDeployer).to.not.equal(undefined);
            expect(contract.remoteAddressValidatorImplementation).to.not.equal(undefined);
            expect(contract.remoteAddressValidator).to.not.equal(undefined);
            expect(contract.tokenManagerImplementations).to.not.equal(undefined);
            expect(contract.implementation).to.not.equal(undefined);
            expect(contract.address).to.not.equal(undefined);

        });
    });
});