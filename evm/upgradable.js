'use strict';

const { ethers } = require('hardhat');
const { Contract, ContractFactory } = ethers;
const { deployAndInitContractConstant, create3DeployAndInitContract } = require('@axelar-network/axelar-gmp-sdk-solidity');
const IUpgradable = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IUpgradable.json');

const { verifyContract, deployCreate, getBytecodeHash, deployContract, printInfo, getDeployedAddress, isContract } = require('./utils');

async function deployUpgradable(
    wallet,
    implementationJson,
    proxyJson,
    implementationConstructorArgs = [],
    proxyConstructorArgs = [],
    setupParams = '0x',
    txOptions = null,
    verifyOptions = null,
) {
    const implementationFactory = new ContractFactory(implementationJson.abi, implementationJson.bytecode, wallet);

    const proxyFactory = new ContractFactory(proxyJson.abi, proxyJson.bytecode, wallet);

    const implementation = await implementationFactory.deploy(...implementationConstructorArgs);
    await implementation.deployed();

    const proxy = await proxyFactory.deploy(...proxyConstructorArgs, txOptions);
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
    salt,
    txOptions = null,
    verifyOptions,
) {
    const implementation = await deployCreate(wallet, implementationJson, implementationConstructorArgs, {}, verifyOptions);

    const proxy = await deployAndInitContractConstant(
        constAddressDeployerAddress,
        wallet,
        proxyJson,
        salt,
        proxyConstructorArgs,
        [implementation.address, wallet.address, setupParams],
        txOptions,
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
    proxyConstructorArgs = [],
    setupParams = '0x',
    salt,
    txOptions = null,
    verifyOptions = null,
) {
    const implementation = await deployCreate(wallet, implementationJson, implementationConstructorArgs, {}, verifyOptions);
    const proxyInitArgs = [implementation.address, wallet.address, setupParams];
    const proxy = await create3DeployAndInitContract(
        create3DeployerAddress,
        wallet,
        proxyJson,
        salt,
        proxyConstructorArgs,
        proxyInitArgs,
        txOptions,
    );

    if (verifyOptions) {
        await verifyContract(verifyOptions.env, verifyOptions.chain, proxy.address, proxyConstructorArgs);
    }

    return new Contract(proxy.address, implementationJson.abi, wallet);
}

async function upgradeUpgradable(
    deployMethod,
    proxyAddress,
    wallet,
    contractJson,
    implementationConstructorArgs,
    setupParams,
    deployOptions = {},
    gasOptions = {},
    verifyOptions = null,
    chain = '',
) {
    const proxy = new Contract(proxyAddress, IUpgradable.abi, wallet);

    const predictedAddress = await getDeployedAddress(wallet.address, deployMethod, {
        ...deployOptions,
        contractJson,
        constructorArgs: implementationConstructorArgs,
        provider: wallet.provider,
    });

    printInfo('Predicted Implementation Address', predictedAddress);
    let implementation;

    if (await isContract(predictedAddress, wallet.provider)) {
        printInfo('New Implementation already deployed', predictedAddress);
        implementation = new Contract(predictedAddress, contractJson.abi, wallet);
    } else {
        implementation = await deployContract(
            deployMethod,
            wallet,
            contractJson,
            implementationConstructorArgs,
            deployOptions,
            gasOptions,
            verifyOptions,
            chain,
        );
        printInfo('New Implementation', implementation.address);
    }

    const implementationCodeHash = await getBytecodeHash(implementation, chain);
    printInfo('New Implementation Code Hash', implementationCodeHash);

    const tx = await proxy.upgrade(implementation.address, implementationCodeHash, setupParams, gasOptions);
    await tx.wait();

    return tx;
}

module.exports = {
    deployUpgradable,
    deployCreate2Upgradable,
    deployCreate3Upgradable,
    upgradeUpgradable,
};
