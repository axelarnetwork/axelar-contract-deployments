'use strict';

// See https://docs.hedera.com/hedera/core-concepts/smart-contracts/system-smart-contracts#iexchangerate.sol
// for some context on tinycents and tinybars.

const TINY_PARTS_PER_WHOLE = 100_000_000;

const DEFAULT_TOKEN_CREATION_PRICE_USD = 1;
const DEFAULT_TOKEN_CREATION_PRICE_TINY_CENTS = DEFAULT_TOKEN_CREATION_PRICE_USD * 100 * TINY_PARTS_PER_WHOLE;

module.exports = {
    TINY_PARTS_PER_WHOLE,
    DEFAULT_TOKEN_CREATION_PRICE_USD,
    DEFAULT_TOKEN_CREATION_PRICE_TINY_CENTS,
};
