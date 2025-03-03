const { isValidStellarAccount, isValidStellarContract } = require('../common/utils');

function runTest(name, fn) {
    try {
        fn();
        console.log(`✅ PASSED: ${name}`);
    } catch (error) {
        console.error(`❌ FAILED: ${name}`);
        console.error(`   ${error.message}`);
    }
}

function assertEqual(actual, expected) {
    if (actual !== expected) {
        throw new Error(`Expected ${expected}, but got ${actual}`);
    }
}

// Grouped test cases
function testIsValidStellarAccount() {
    runTest('Valid Stellar account address', () => {
        assertEqual(isValidStellarAccount('GC7Q6CGOL6I2HCTVUXLEFXC5WM2KKUGJ7TM5XMMKOAIIU7BY462JYD3R'), true); // public
        assertEqual(isValidStellarAccount('GAIH3ULLFQ4DGSECF2AR555KZ4KNDGEKN4AFI4SU2M7B43MGK3QJZNSR'), true); // testnet
    });

    runTest('Invalid Stellar account address (wrong prefix)', () => {
        assertEqual(isValidStellarAccount('CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75'), false);
    });

    runTest('Invalid Stellar account address (wrong length)', () => {
        assertEqual(isValidStellarAccount('GA3D5YFQVLXOI5Z2PEF2DYLOX2C5E5T6ZPCPZ7R4MCE3X5VH3TURT'), false);
    });

    runTest('Empty string', () => {
        assertEqual(isValidStellarAccount(''), false);
    });

    runTest('Random invalid string', () => {
        assertEqual(isValidStellarAccount('notARealStellarAddress'), false);
    });
}

function testIsValidStellarContract() {
    runTest('Valid Stellar contract address (Base32 C...)', () => {
        assertEqual(isValidStellarContract('CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75'), true); // public
        assertEqual(isValidStellarContract('CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC'), true); // testnet
    });

    runTest('Invalid Stellar contract address (wrong prefix)', () => {
        assertEqual(isValidStellarContract('GA6VHUEMCDSR4HUGZH7IGYXAPFSAVD7ET5OBODQKXZQTGJX7M7E2MPTA'), false);
    });

    runTest('Invalid Stellar contract address (wrong length)', () => {
        assertEqual(isValidStellarContract('CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7QUWUZPUTHXSTZLEO7SJMI75'), false);
    });

    runTest('Empty string', () => {
        assertEqual(isValidStellarContract(''), false);
    });

    runTest('Random invalid string', () => {
        assertEqual(isValidStellarContract('notARealContractAddress'), false);
    });
}

// Run all tests
console.log('Running tests for isValidStellarAccount...');
testIsValidStellarAccount();

console.log('\nRunning tests for isValidStellarContract...');
testIsValidStellarContract();
