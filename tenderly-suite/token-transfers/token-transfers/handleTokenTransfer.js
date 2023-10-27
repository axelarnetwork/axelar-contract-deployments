const { ethers } = require('ethers');
const axios = require('axios').default;

const ERC20_ABI = ['function symbol() external view returns (string)', 'function decimals() external view returns (uint8)'];
const ITS_ABI = ['function getTokenAddress(bytes32 tokenId) external view returns (address)'];

const COIN_MARKET_QUOTES_URL = 'https://pro-api.coinmarketcap.com/v2/cryptocurrency/quotes/latest';
const PAGER_DUTY_ALERT_URL = 'https://events.pagerduty.com/v2/enqueue';

const TOKEN_THRESHOLD_WITH_PRICE = [50000, 100000, 500000]; // [info, warning, critical]
const TOKEN_THRESHOLD_WITHOUT_PRICE = [500000, 1000000, 5000000]; // [info, warning, critical]

const TOPIC_0_TOKEN_SENT = '0x9df6ec8cb445f82179f3b208e5fba6df6aa89854dfccb3af0bf74791dba47929';
const TOPIC_0_TOKEN_SENT_WITH_DATA = '0x875cab82a677ce38c76127a24fa89a67df04116c1a0bf652b61abb89b289f433';
const TOPIC_0_TOKEN_RECIEVED = '0xa5392cc9825f3ea9fa772e43f2392ca1a9e97db3619eac789383aaaaabb467c4';
const TOPIC_0_TOKEN_RECIEVED_WITH_DATA = '0x35f4643275e22b7f12d809c70f685b292b1ade91c4033884bdd0a49bfbe737c3';

const handleTokenTransferFn = async (context, event) => {
    const chainName = context.metadata.getNetwork();
    const provider = new ethers.providers.JsonRpcProvider(context.gateways.getGateway(chainName));

    let its;

    const tokenTransferAmounts = [];
    const tokenIDs = [];

    event.logs.forEach(function (log) {
        if (
            log.topics[0] === TOPIC_0_TOKEN_SENT ||
            log.topics[0] === TOPIC_0_TOKEN_SENT_WITH_DATA ||
            log.topics[0] === TOPIC_0_TOKEN_RECIEVED ||
            log.topics[0] === TOPIC_0_TOKEN_RECIEVED_WITH_DATA
        ) {
            if(!its){
                its = new ethers.Contract(log.address, ITS_ABI, provider);
            }

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

        const erc20Address = await its.getTokenAddress(id);
        const erc20 = new ethers.Contract(erc20Address, ERC20_ABI, provider);
        const symbol = (await erc20.symbol()).replace('axl', '');
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
                tokenThreshold = TOKEN_THRESHOLD_WITH_PRICE;
                totalAmount = tokenTransferAmount * tokenPrice;
            } else {
                tokenThreshold = TOKEN_THRESHOLD_WITHOUT_PRICE;
                totalAmount = tokenTransferAmount;
            }
        } catch (error) {
            console.log('CMC token quote requested: ', symbol);
            console.log('CMC error status: ', error.response.status);
            console.log('CMC error response: ', error.response);
            throw Error('ERROR_IN_FETCHING_PRICE');
        }

        let tempSeverity = 0;

        if (totalAmount > tokenThreshold[2]) {
            tempSeverity = 3;
        } else if (totalAmount > tokenThreshold[1]) {
            tempSeverity = 2;
        } else if (totalAmount > tokenThreshold[0]) {
            tempSeverity = 1;
        }

        if (tempSeverity) {
            if (tempSeverity > severity) {
                severity = tempSeverity;
            }

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
                        severity: getSeverityString(severity),
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

function getSeverityString(severity) {
    if (severity === 3) {
        return 'critical';
    }

    if (severity === 2) {
        return 'warning';
    }

    if (severity === 1) {
        return 'info';
    }

    return '';
}

module.exports = { handleTokenTransferFn };
