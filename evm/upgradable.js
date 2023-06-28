'use strict';

const {
    Contract,
    ContractFactory,
    utils: { keccak256 },
} = require('ethers');
const { deployContractConstant, deployAndInitContractConstant, deployCreate3Contract } = require('@axelar-network/axelar-gmp-sdk-solidity');
const IUpgradable = require('@axelar-network/axelar-gmp-sdk-solidity/dist/IUpgradable.json');

const { verifyContract } = require('./utils');

async function deployCreateUpgradable(
    wallet,
    implementationJson,
    proxyJson,
    implementationConstructorArgs = [],
    proxyConstructorArgs = [],
    setupParams = '0x',
    gasOptions = null,
    env = 'testnet',
    chain = 'ethereum',
    verify = false,
) {
    const implementationFactory = new ContractFactory(implementationJson.abi, implementationJson.bytecode, wallet);

    const proxyFactory = new ContractFactory(proxyJson.abi, proxyJson.bytecode, wallet);

    const implementation = await implementationFactory.deploy(...implementationConstructorArgs);
    await implementation.deployed();

    const proxy = await proxyFactory.deploy(...proxyConstructorArgs, gasOptions);
    await proxy.deployed();

    await proxy.init(implementation.address, wallet.address, setupParams).then((tx) => tx.wait());

    if (verify) {
        await verifyContract(env, chain, implementation.address, implementationConstructorArgs);
        await verifyContract(env, chain, proxy.address, proxyConstructorArgs);
    }

    return new Contract(proxy.address, implementationJson.abi, wallet);
}

async function deployCreate2(
    constAddressDeployerAddress,
    wallet,
    contractJson,
    key = Date.now(),
    args = [],
    gasOptions = null,
    env = 'testnet',
    chain = 'ethereum',
    verify = false,
) {
    const contract = await deployContractConstant(constAddressDeployerAddress, wallet, contractJson, key, args, gasOptions?.gasLimit);

    if (verify) {
        await verifyContract(env, chain, contract.address, args);
    }

    return new Contract(contract.address, contractJson.abi, wallet);
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
    env = 'testnet',
    chain = 'ethereum',
    verify = false,
) {
    const implementationFactory = new ContractFactory(implementationJson.abi, implementationJson.bytecode, wallet);

    const implementation = await implementationFactory.deploy(...implementationConstructorArgs);
    await implementation.deployed();

    const proxy = await deployAndInitContractConstant(
        constAddressDeployerAddress,
        wallet,
        proxyJson,
        key,
        proxyConstructorArgs,
        [implementation.address, wallet.address, setupParams],
        gasOptions?.gasLimit,
    );

    if (verify) {
        await verifyContract(env, chain, implementation.address, implementationConstructorArgs);
        await verifyContract(env, chain, proxy.address, proxyConstructorArgs);
    }

    return new Contract(proxy.address, implementationJson.abi, wallet);
}

async function deployCreate3(
    create3DeployerAddress,
    wallet,
    contractJson,
    key = Date.now(),
    args = [],
    gasOptions = null,
    env = 'testnet',
    chain = 'ethereum',
    verify = false,
) {
    const implementation = await deployCreate3Contract(create3DeployerAddress, wallet, contractJson, key, args, gasOptions?.gasLimit);

    if (verify) {
        await verifyContract(env, chain, implementation.address, args);
    }

    return new Contract(implementation.address, contractJson.abi, wallet);
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
    env = 'testnet',
    chain = 'ethereum',
    verify = false,
) {
    const implementationFactory = new ContractFactory(implementationJson.abi, implementationJson.bytecode, wallet);

    const implementation = await implementationFactory.deploy(...implementationConstructorArgs);
    await implementation.deployed();

    const proxy = await deployCreate3Contract(
        create3DeployerAddress,
        wallet,
        proxyJson,
        key,
        [implementation.address, wallet.address, setupParams, ...additionalProxyConstructorArgs],
        gasOptions?.gasLimit,
    );

    if (verify) {
        await verifyContract(env, chain, implementation.address, implementationConstructorArgs);
        await verifyContract(env, chain, proxy.address, additionalProxyConstructorArgs);
    }

    return new Contract(proxy.address, implementationJson.abi, wallet);
}

async function upgradeUpgradable(
    proxyAddress,
    wallet,
    contractJson,
    implementationConstructorArgs = [],
    setupParams = '0x',
    gasOptions = null,
    env = 'testnet',
    chain = 'ethereum',
    verify = false,
) {
    const proxy = new Contract(proxyAddress, IUpgradable.abi, wallet);

    const implementationFactory = new ContractFactory(contractJson.abi, contractJson.bytecode, wallet);

    const implementation = await implementationFactory.deploy(...implementationConstructorArgs, gasOptions);
    await implementation.deployed();

    const implementationCode = await wallet.provider.getCode(implementation.address);
    const implementationCodeHash = keccak256(implementationCode);

    const tx = await proxy.upgrade(implementation.address, implementationCodeHash, setupParams);
    await tx.wait();

    if (verify) {
        await verifyContract(env, chain, implementation.address, implementationConstructorArgs);
    }

    return tx;
}

module.exports = {
    deployCreate2,
    deployCreateUpgradable,
    deployCreate2Upgradable,
    deployCreate3,
    deployCreate3Upgradable,
    upgradeUpgradable,
};
