import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { expect } from "chai"
import { Registry } from "../target/types/registry";
import { Gateway } from "../target/types/gateway";
import { AccountMeta, PublicKey, SystemProgram } from "@solana/web3.js";

describe("axelar gateway for solana", () => {
    type Bytes = Uint8Array
    
    const connection = new anchor.web3.Connection("http://localhost:8899", {
        commitment: "confirmed"
    });
    const wallet = anchor.Wallet.local();
    const provider = new anchor.AnchorProvider(connection, wallet, {
        commitment: "confirmed"
    });
    anchor.setProvider(provider)

    const registryProgram = anchor.workspace.Registry as Program<Registry>
    const gatewayProgram = anchor.workspace.Gateway as Program<Gateway>

    // create new account and fund it with SOL tokens
    async function new_keypair_with_sol(amount: number) {
        const account = anchor.web3.Keypair.generate();
        const airdrop_tx = await registryProgram.provider.connection.requestAirdrop(
            account.publicKey,
            anchor.web3.LAMPORTS_PER_SOL * amount
        );
        const latestBlockHash = await registryProgram.provider.connection.getLatestBlockhash();
        await registryProgram.provider.connection.confirmTransaction({
            blockhash: latestBlockHash.blockhash,
            lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
            signature: airdrop_tx
        });
        return account;
    }

    // generate dummy [u8;32]
    async function get_dummy_seeds_hash(val: number) {
        const fake_seeds_hash: number[] = [];
        for (let i = 0; i < 32; i++) {
            fake_seeds_hash.push(val);
        };
        return fake_seeds_hash;
    }

    async function generateRegistryPDAArray(amount: number) {
        const accounts: PublicKey[] = [];
        for (let i = 0; i < amount; i++) {
            const dummy_seeds_hash =  await get_dummy_seeds_hash((i + 100));
            const [state_pda] = anchor.web3.PublicKey.findProgramAddressSync(
                [Buffer.from(dummy_seeds_hash)],
                registryProgram.programId);
            accounts.push(state_pda);
        }

        return accounts
    }

    it("cpi-to-registry-get-is-command-executed", async () => {
        const authority_account = await new_keypair_with_sol(3);
        const expected_authority = authority_account.publicKey;
        const expected_value = true;

        const dummy_seeds_hash = await get_dummy_seeds_hash(7);
        const [state_pda] = anchor.web3.PublicKey.findProgramAddressSync(
            [Buffer.from(dummy_seeds_hash)],
            registryProgram.programId);

        await registryProgram.methods
            .initialize(dummy_seeds_hash, expected_value)
            .accounts({
                state: state_pda,
                authority: expected_authority,
                systemProgram: anchor.web3.SystemProgram.programId,
            })
            .signers([authority_account])
            .rpc();

        const initialized_state_data = await registryProgram.account.state.fetch(state_pda);
        expect(initialized_state_data.value).to.deep.equal(expected_value);

        const ret = await gatewayProgram.methods
            .isCommandExecuted(dummy_seeds_hash)
            .accounts({
                state: state_pda,
                registryProgram: registryProgram.programId}
            ).view()

        expect(ret).to.deep.equal(expected_value);
    });

    it("cpi-to-registry-get-is-contract-call-approved", async () => {
        const authority_account = await new_keypair_with_sol(3);
        const expected_authority = authority_account.publicKey;
        const expected_value = true;

        const dummy_seeds_hash = await get_dummy_seeds_hash(8);
        const [state_pda] = anchor.web3.PublicKey.findProgramAddressSync(
            [Buffer.from(dummy_seeds_hash)],
            registryProgram.programId);

        await registryProgram.methods
            .initialize(dummy_seeds_hash, expected_value)
            .accounts({
                state: state_pda,
                authority: expected_authority,
                systemProgram: anchor.web3.SystemProgram.programId,
            })
            .signers([authority_account])
            .rpc();

        const initialized_state_data = await registryProgram.account.state.fetch(state_pda);
        expect(initialized_state_data.value).to.deep.equal(expected_value);

        const ret = await gatewayProgram.methods
            .isContractCallApproved(dummy_seeds_hash)
            .accounts({
                state: state_pda,
                registryProgram: registryProgram.programId}
            ).view()

        expect(ret).to.deep.equal(expected_value);
    });

    it("cpi-to-registry-get-set-validate-contract-call", async () => {
        const authority_account = await new_keypair_with_sol(3);
        const authority_pubkey = authority_account.publicKey;
        const initial_value = true;
        const after_mutation_value = false;

        const dummy_seeds_hash = await get_dummy_seeds_hash(9);
        const [state_pda] = anchor.web3.PublicKey.findProgramAddressSync(
            [Buffer.from(dummy_seeds_hash)],
            registryProgram.programId);

        await registryProgram.methods
            .initialize(dummy_seeds_hash, initial_value)
            .accounts({
                state: state_pda,
                authority: authority_pubkey,
                systemProgram: anchor.web3.SystemProgram.programId,
            })
            .signers([authority_account])
            .rpc();

        const initialized_state_data = await registryProgram.account.state.fetch(state_pda);
        expect(initialized_state_data.value).to.deep.equal(initial_value);

        await gatewayProgram.methods
            .validateContractCall(dummy_seeds_hash)
            .accounts({
                state: state_pda,
                authority: authority_pubkey,
                registryProgram: registryProgram.programId}
            )
            .signers([authority_account])
            .rpc();

        const after_mutation_state_data = await registryProgram.account.state.fetch(state_pda);
        expect(after_mutation_state_data.value).to.deep.equal(after_mutation_value);
    });

    it("event-check-call-contract", async () => {
        type ContractCallEvent = {
            sender: {
              _bn: string;
            };
            destinationChain: string;
            destinationContractAddress: string;
            payloadHash: number[];
            payload: Buffer;
        };

        const sender_account = await new_keypair_with_sol(3);
        
        const expected_destination_chain = "AAA";
        const expected_destination_contract_address = "BBB";
        const expected_payload = Buffer.from("CCC");
        const expected_payload_hash = [
            42, 105, 111,  13, 166, 173, 112,
           213, 111,  84, 170,  83, 208,  40,
           125, 141, 127, 197, 162,  54,  99,
           108,  72, 219,  37, 215, 116, 133,
           241, 231, 111, 180
        ];

        let listener = null;
        let [event, _] = await new Promise<[ContractCallEvent, any]>((resolve, _reject) => {
            listener = gatewayProgram.addEventListener("ContractCallEvent", (event, slot) => {
                resolve([event, slot]);
            });
            gatewayProgram.methods
            .callContract(
                expected_destination_chain,
                expected_destination_contract_address,
                expected_payload)
            .accounts({sender: sender_account.publicKey})
            .signers([sender_account])
            .rpc();
        });
        await gatewayProgram.removeEventListener(listener);

        expect(event.destinationChain).to.deep.equal(expected_destination_chain);
        expect(event.destinationContractAddress).to.deep.equal(expected_destination_contract_address);
        expect(event.payload).to.deep.equal(expected_payload);
        expect(event.payloadHash).to.deep.equal(expected_payload_hash);
    });

    it("cpi-to-registry-get-set-execute-single-message", async() => {
        const authority_account = await new_keypair_with_sol(3)
    });
});