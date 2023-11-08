const { ethers } = require('ethers');
const axios = require('axios').default;

const ERC20_ABI = ['function symbol() external view returns (string)', 'function decimals() external view returns (uint8)'];
const ITS_ABI = ['function validTokenAddress(bytes32 tokenId) external view returns (address)'];

const COIN_MARKET_QUOTES_URL = 'https://pro-api.coinmarketcap.com/v2/cryptocurrency/quotes/latest';
const PAGER_DUTY_ALERT_URL = 'https://events.pagerduty.com/v2/enqueue';

const Severity = {
    1: 'info',
    2: 'warning',
    3: 'critical',
};

const handleTokenTransferFn = async (context, event) => {
    if (!event || !event.logs || !context || !context.metadata) {
        throw new Error('INVALID_INPUT_FOR_ACTION');
    }

    const { tokenSent, tokenSentWithData, tokenReceived, tokenReceivedWithData } = await context.storage.getJson('EventsABI');

    const tokenSentHash = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(tokenSent));
    const tokenSentWithDataHash = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(tokenSentWithData));
    const tokenReceivedHash = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(tokenReceived));
    const tokenReceivedWithDataHash = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(tokenReceivedWithData));

    const chainName = context.metadata.getNetwork();
    const provider = new ethers.providers.JsonRpcProvider(await context.secrets.get(`RPC_${chainName.toUpperCase()}`));
    const its = new ethers.Contract(await context.storage.getStr('ITSContractAddress'), ITS_ABI, provider);

    const tokenTransferAmounts = [];
    const tokenIDs = [];

    event.logs.forEach(function (log) {
        if (
            log.topics[0] === tokenSentHash ||
            log.topics[0] === tokenSentWithDataHash ||
            log.topics[0] === tokenReceivedHash ||
            log.topics[0] === tokenReceivedWithDataHash
        ) {
            tokenIDs.push(log.topics[1]);
            tokenTransferAmounts.push(ethers.BigNumber.from(log.topics[log.topics.length - 1]));
        }
    });

    if (tokenIDs.length === 0) {
        throw new Error('NO_TOKEN_TRANSFER_DETECTED');
    }

    const amountsAboveThreshold = [];
    const prices = [];
    const symbols = [];
    let severity = 0;

    for (let index = 0; index < tokenIDs.length; index++) {
        const id = tokenIDs[index];

        const erc20Address = await its.validTokenAddress(id);
        const erc20 = new ethers.Contract(erc20Address, ERC20_ABI, provider);
        const symbol = resolveTokenSymbol(await erc20.symbol());
        const decimals = await erc20.decimals();

        const tokenTransferAmount = parseFloat(ethers.utils.formatUnits(tokenTransferAmounts[index], decimals));

        const params = {
            symbol,
        };
        const headers = {
            'X-CMC_PRO_API_KEY': await context.secrets.get('CMC_API_KEY'),
            Accept: 'application/json',
        };

        let tokenPrice, tokenThreshold, totalAmount;

        try {
            const result = await axios.get(COIN_MARKET_QUOTES_URL, { params, headers });

            if (result.data.data[symbol].length !== 0) {
                tokenPrice = result.data.data[symbol][0].quote.USD.price;
                tokenThreshold = await context.storage.getJson('TokenThresholdWithPrice');
                totalAmount = tokenTransferAmount * tokenPrice;
            } else {
                tokenThreshold = await context.storage.getJson('TokenThresholdWithoutPrice');
                totalAmount = tokenTransferAmount;
            }
        } catch (error) {
            console.log('CMC token quote requested: ', symbol);
            console.log('CMC error status: ', error.response.status);
            console.log('CMC error response: ', error.response);
            throw Error('ERROR_IN_FETCHING_PRICE');
        }

        let thresholdCrossed = 0;

        if (totalAmount > tokenThreshold[2]) {
            if (severity <= 2) {
                severity = 3;
            }

            thresholdCrossed = 1;
        } else if (totalAmount > tokenThreshold[1]) {
            if (severity <= 1) {
                severity = 2;
            }

            thresholdCrossed = 1;
        } else if (totalAmount > tokenThreshold[0]) {
            if (severity === 0) {
                severity = 1;
            }

            thresholdCrossed = 1;
        }

        if (thresholdCrossed === 1) {
            thresholdCrossed = 0;
            amountsAboveThreshold.push(totalAmount);
            prices.push(tokenPrice);
            symbols.push(symbol);
        }
    }

    if (severity) {
        try {
            await axios.post(
                PAGER_DUTY_ALERT_URL,
                {
                    routing_key: await context.secrets.get('PD_ROUTING_KEY'),
                    event_action: 'trigger',
                    payload: {
                        summary: 'Token tranfer amount crossed threshold',
                        source: `${chainName}-ITS-${its.address}`,
                        severity: Severity[severity],
                        custom_details: {
                            timestamp: Date.now(),
                            chain_name: chainName,
                            transaction: event.hash,
                            transfer_info: {
                                amounts: amountsAboveThreshold,
                                prices,
                                symbols,
                            },
                            payload: event,
                        },
                    },
                },
                {},
            );
        } catch (error) {
            console.log('PD error status: ', error.response.status);
            console.log('PD error response: ', error.response.data);
            throw Error('TOKEN_TRANSFER_ALERT_FAILED');
        }
    }
};

function resolveTokenSymbol(symbol) {
    if (!symbol) {
        throw new Error('NO_SYMBOL_DETECTED');
    }

    if (symbol.length >= 3 && symbol.substring(0, 3).toLowerCase() === 'axl') {
        return symbol.substring(3);
    }

    return symbol;
}

module.exports = { handleTokenTransferFn };
