const { ethers } = require('ethers');

// Convert formatted amount to atomic units (e.g. 1000000000). Default decimals is 9 for SUI
function getAtomicAmount(amount, decimals = 9) {
    return ethers.utils.parseUnits(amount, decimals).toBigInt();
}

// Convert atomic amount to formatted units (e.g. 1.0) with decimals. Default decimals is 9 for SUI
function getFormattedAmount(amount, decimals = 9) {
    return ethers.utils.formatUnits(amount, decimals);
}

module.exports = {
    getAtomicAmount,
    getFormattedAmount,
};
