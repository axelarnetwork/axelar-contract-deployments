# Contract deployments for Axelar

Install and copy default setting with

```
npm ci && cp example.env .env
```

To test the Interchain Token Service deployment

```
node evm/deploy-its -n ${chain-name}
```

You can change `.env` to run the above script to testnet instead of local. Change the `SALT` to get a new address.
