const { providers: { Web3Provider }, Contract, getDefaultProvider, Wallet } = require('ethers');

const ganache = require('ganache');
const { getContractJSON, writeJSON } = require('./utils');
const IInterchainTokenService = getContractJSON('IInterchainTokenService');
const { createServer } = require('http');
const hasOwnProperty = Object.prototype.hasOwnProperty;
const { Command } = require('commander');
const { addBaseOptions } = require('./cli-utils');
const info = require('../axelar-chains-config/info/mainnet.json');
const fs = require('fs');
const toml = require('toml');
const RPCs = toml.parse(fs.readFileSync('./axelar-chains-config/info/rpcs.toml', 'utf-8'));

function createCORSResponseHeaders(method, requestHeaders) {
    // https://fetch.spec.whatwg.org/#http-requests
    const headers = {};
    const isCORSRequest = hasOwnProperty.call(requestHeaders, 'origin');
    if (isCORSRequest) {
        // OPTIONS preflight requests need a little extra treatment
        if (method === 'OPTIONS') {
            // we only allow POST requests, so it doesn't matter which method the request is asking for
            headers['Access-Control-Allow-Methods'] = 'POST';
            // echo all requested access-control-request-headers back to the response
            if (hasOwnProperty.call(requestHeaders, 'access-control-request-headers')) {
                headers['Access-Control-Allow-Headers'] = requestHeaders['access-control-request-headers'];
            }
            // Safari needs Content-Length = 0 for a 204 response otherwise it hangs forever
            // https://github.com/expressjs/cors/pull/121#issue-130260174
            headers['Content-Length'] = 0;

            // Make browsers and compliant clients cache the OPTIONS preflight response for 10
            // minutes (this is the maximum time Chromium allows)
            headers['Access-Control-Max-Age'] = 600; // seconds
        }

        // From the spec: https://fetch.spec.whatwg.org/#http-responses
        // "For a CORS-preflight request, request’s credentials mode is always "omit",
        // but for any subsequent CORS requests it might not be. Support therefore
        // needs to be indicated as part of the HTTP response to the CORS-preflight request as well.", so this
        // header is added to all requests.
        // Additionally, https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Allow-Credentials,
        // states that there aren't any HTTP Request headers that indicate you whether or not Request.withCredentials
        // is set. Because web3@1.0.0-beta.35-? always sets `request.withCredentials = true` while Safari requires it be
        // returned even when no credentials are set in the browser this header must always be return on all requests.
        // (I've found that Chrome and Firefox don't actually require the header when credentials aren't set)
        //  Regression Commit: https://github.com/ethereum/web3.js/pull/1722
        //  Open Web3 Issue: https://github.com/ethereum/web3.js/issues/1802
        headers['Access-Control-Allow-Credentials'] = 1;

        // From the spec: "It cannot be reliably identified as participating in the CORS protocol
        // as the `Origin` header is also included for all requests whose method is neither
        // `GET` nor `HEAD`."
        // Explicitly set the origin instead of using *, since credentials
        // can't be used in conjunction with *. This will always be set
        /// for valid preflight requests.
        headers['Access-Control-Allow-Origin'] = requestHeaders.origin;
    }
    return headers;
}

function sendResponse(response, statusCode, headers, data = null) {
    response.writeHead(statusCode, headers);
    response.end(data);
}

function rpcError(id, code, msg) {
    return JSON.stringify({
        jsonrpc: '2.0',
        id: id,
        error: {
            code: code,
            message: msg,
        },
    });
}

// eslint-disable-next-line @typescript-eslint/no-empty-function
function server(ganacheProvider, logger = { log: function (...args) {} }) {
    const server = createServer(function (request, response) {
        const method = request.method;
        const chunks = [];

        request
            .on('data', function (chunk) {
                chunks.push(chunk);
            })
            .on('end', async function () {
                const body = Buffer.concat(chunks).toString();
                // At this point, we have the headers, method, url and body, and can now
                // do whatever we need to in order to respond to this request.

                const headers = createCORSResponseHeaders(method, request.headers);
                const badRequest = () => {
                    headers['Content-Type'] = 'text/plain';
                    sendResponse(response, 400, headers, '400 Bad Request');
                };
                const url = request.url?.split('/');
                if (!url) return;
                url?.shift();
                switch (method) {
                    case 'POST':
                        // eslint-disable-next-line no-case-declarations
                        let payload;
                        try {
                            payload = JSON.parse(body);
                        } catch (e) {
                            badRequest();
                            return;
                        }

                        // Log messages that come into the TestRPC via http
                        if (payload instanceof Array) {
                            // Batch request
                            for (let i = 0; i < payload.length; i++) {
                                const item = payload[i];
                                logger.log(item.method);
                            }
                        } else {
                            logger.log(payload.method);
                        }

                        // http connections do not support subscriptions
                        if (payload.method === 'eth_subscribe' || payload.method === 'eth_unsubscribe') {
                            headers['Content-Type'] = 'application/json';
                            sendResponse(response, 400, headers, rpcError(payload.id, -32000, 'notifications not supported'));
                            break;
                        }

                        ganacheProvider.send(payload, function (_, result) {
                            headers['Content-Type'] = 'application/json';
                            sendResponse(response, 200, headers, JSON.stringify(result));
                        });

                        break;
                    case 'OPTIONS':
                        sendResponse(response, 204, headers);
                        break;
                    default:
                        badRequest();
                        break;
                }
            });
    });
    return server;
}

async function main(options) {
    const chainName = options.chainNames;
    const chainInfo = info.chains[chainName];
    const contracts = chainInfo.contracts;
    const rpc = RPCs.axelar_bridge_evm.find((chain) => chain.name.toLowerCase() === chainName).rpc_addr;
    const wallet = new Wallet(options.privateKey);

    let provider = getDefaultProvider(rpc);
    let its = new Contract(contracts.InterchainTokenService.address, IInterchainTokenService.abi, provider);
    const ownerAddress = await its.owner();

    const ganacheOptions = {
        wallet: {
            accounts: [
                { balance: 1e18, secretKey: options.privateKey }
            ],
            unlockedAccounts: [ownerAddress],
        },
        chain: {
            chainId: chainInfo.chainId,
            networkId: chainInfo.chainId,
            vmErrorsOnRPCResponse: true,
        },
        fork: {
            url: rpc,
        },
        logging: { quiet: true },
    };
    const ganacheProvider = ganache.provider(ganacheOptions);
    provider = new Web3Provider(ganacheProvider);
    const owner = provider.getSigner(ownerAddress);
    its = new Contract(contracts.InterchainTokenService.address, IInterchainTokenService.abi);
    await (await its.connect(owner).transferOwnership(wallet.address)).wait();
    server(ganacheProvider).listen(8500);
    chainInfo.rpc = `http://localhost:8500`;
    const forkedInfo = {
        chains: {
            [chainName]: chainInfo,
        },
    };
    writeJSON(forkedInfo, 'axelar-chains-config/info/local.json');
    console.log('listening');
};


if (require.main === module) {
    const program = new Command();

    program.name('fork').description('Script to fork an existing network')

    addBaseOptions(program, { address: true });

    program.action((chainName, options) => {
        main(chainName, options);
    });

    program.parse();
}



