// cosmwasm-stargate imports protobufjs which does not have a default export
// Therefore, import SigningCosmWasmClient using CommonJS to avoid error TS1192
// eslint-disable-next-line @typescript-eslint/no-require-imports
export const { SigningCosmWasmClient } = require('@cosmjs/cosmwasm-stargate');

export interface ChainEndpoint {
    name: string;
    gateway: {
        address: string;
    };
}

export async function queryChainsFromRouter(client: typeof SigningCosmWasmClient, router_address: string): Promise<ChainEndpoint[]> {
    try {
        const res: ChainEndpoint[] = await client.queryContractSmart(router_address, { chains: {} });
        return res;
    } catch (error) {
        throw error;
    }
}
