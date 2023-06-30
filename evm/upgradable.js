'use strict';

const {
    Contract,
    ContractFactory,
    utils: { keccak256 },
} = require('ethers');
const { deployAndInitContractConstant, deployCreate3Contract } = require('@axelar-network/axelar-gmp-sdk-solidity');
const IUpgradable = require('@axelar-network/axelar-gmp-sdk-solidity/dist/IUpgradable.json');

const { verifyContract, deployContract } = require('./utils');

async function deployUpgradable(
    wallet,
    implementationJson,
    proxyJson,
    implementationConstructorArgs = [],
    proxyConstructorArgs = [],
    setupParams = '0x',
    gasOptions = null,
    verifyOptions = null,
) {
    const implementationFactory = new ContractFactory(implementationJson.abi, implementationJson.bytecode, wallet);

    const proxyFactory = new ContractFactory(proxyJson.abi, proxyJson.bytecode, wallet);

    const implementation = await implementationFactory.deploy(...implementationConstructorArgs);
    await implementation.deployed();

    const proxy = await proxyFactory.deploy(...proxyConstructorArgs, gasOptions);
    await proxy.deployed();

    await proxy.init(implementation.address, wallet.address, setupParams).then((tx) => tx.wait());

    if (verifyOptions) {
        await verifyContract(verifyOptions.env, verifyOptions.chain, proxy.address, proxyConstructorArgs);
    }

    return new Contract(proxy.address, implementationJson.abi, wallet);
}

async function deployCreate2Upgradable(
    constAddressDeployerAddress,
    wallet,
    implementationJson,
    proxyJson,
    implementationConstructorArgs = [],
    proxyConstructorArgs = [],
    setupParams = '0x',
    key = Date.now(),
    gasOptions = null,
    verifyOptions,
) {
    const implementation = await deployContract(wallet, implementationJson, implementationConstructorArgs, {}, verifyOptions);

    const proxy = await deployAndInitContractConstant(
        constAddressDeployerAddress,
        wallet,
        proxyJson,
        key,
        proxyConstructorArgs,
        [implementation.address, wallet.address, setupParams],
        gasOptions?.gasLimit,
    );

    if (verifyOptions) {
        await verifyContract(verifyOptions.env, verifyOptions.chain, proxy.address, proxyConstructorArgs);
    }

    return new Contract(proxy.address, implementationJson.abi, wallet);
}

async function deployCreate3Upgradable(
    create3DeployerAddress,
    wallet,
    implementationJson,
    proxyJson,
    implementationConstructorArgs = [],
    additionalProxyConstructorArgs = [],
    setupParams = '0x',
    key = Date.now().toString(),
    gasOptions = null,
    verifyOptions = null,
) {
    const implementation = await deployContract(wallet, implementationJson, implementationConstructorArgs, {}, verifyOptions);

    const proxyConstructorArgs = [implementation.address, wallet.address, setupParams, ...additionalProxyConstructorArgs];
    const proxy = await deployCreate3Contract(create3DeployerAddress, wallet, proxyJson, key, proxyConstructorArgs, gasOptions?.gasLimit);

    if (verifyOptions) {
        await verifyContract(verifyOptions.env, verifyOptions.chain, proxy.address, proxyConstructorArgs);
    }

    return new Contract(proxy.address, implementationJson.abi, wallet);
}

async function upgradeUpgradable(
    proxyAddress,
    wallet,
    contractJson,
    implementationConstructorArgs = [],
    implementationDeploymentOptions = null,
    setupParams = '0x',
    verifyOptions = null,
) {
    const proxy = new Contract(proxyAddress, IUpgradable.abi, wallet);

    const implementation = await deployContract(
        wallet,
        contractJson,
        implementationConstructorArgs,
        implementationDeploymentOptions,
        verifyOptions,
    );

    const implementationCode = await wallet.provider.getCode(implementation.address);
    const implementationCodeHash = keccak256(implementationCode);

    const tx = await proxy.upgrade(implementation.address, implementationCodeHash, setupParams);
    await tx.wait();

    return tx;
}

module.exports = {
    deployUpgradable,
    deployCreate2Upgradable,
    deployCreate3Upgradable,
    upgradeUpgradable,
};
