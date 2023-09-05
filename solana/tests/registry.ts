import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { assert, expect } from "chai"
import { Registry } from "../target/types/registry";

describe("axelar registry for solana", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.Registry as Program<Registry>;
  
  const account_not_initialized_expected_error = {
    error: {
      errorCode: { code: 'AccountNotInitialized', number: 3012 },
      errorMessage: 'The program expected this account to be already initialized',
      comparedValues: undefined,
      origin: 'state'
    }
  };

  const imposter_authority_expected_error_code_msg = {
    error: {
      errorCode: { code: 'ConstraintHasOne', number: 2001 },
      errorMessage: 'A has one constraint was violated',
    }
  }

  // create new account and fund it with SOL tokens
  async function new_keypair_with_sol(amount: number) {
    const account = anchor.web3.Keypair.generate();
    const airdrop_tx = await program.provider.connection.requestAirdrop(
      account.publicKey,
      anchor.web3.LAMPORTS_PER_SOL * amount
    );
    const latestBlockHash = await program.provider.connection.getLatestBlockhash();
    await program.provider.connection.confirmTransaction({
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

  it("get-not-initialized", async () => {
    try {
      const dummy_seeds_hash = await get_dummy_seeds_hash(0);
      const [state_pda] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from(dummy_seeds_hash)],
        program.programId);

      await program.methods
        .get(dummy_seeds_hash)
        .accounts({state: state_pda})
        .rpc();
    } catch (error) {
        expect(error.error).to.deep.equal(account_not_initialized_expected_error.error);
      };
  });

  it("initialize-confirm-value-and-authority", async () => {
    const authority_account = await new_keypair_with_sol(3);
    const expected_value = true;
    const expected_authority = authority_account.publicKey;
    const dummy_seeds_hash = await get_dummy_seeds_hash(1);
    const [state_pda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from(dummy_seeds_hash)],
      program.programId);

    try {  
      await program.methods
        .get(dummy_seeds_hash)
        .accounts({state: state_pda})
        .rpc();
    } catch (error) {
        expect(error.error).to.deep.equal(account_not_initialized_expected_error.error);
      };

    await program.methods
      .initialize(dummy_seeds_hash, expected_value)
      .accounts({
        state: state_pda,
        authority: expected_authority,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([authority_account])
      .rpc();

    const initialized_state_data = await program.account.state.fetch(state_pda);
    
    expect(initialized_state_data.value).to.deep.equal(expected_value);
    expect(initialized_state_data.authority).to.deep.equal(expected_authority)
  });

  it("re-set-value-with-correct-authority", async () => {
    const authority_account = await new_keypair_with_sol(3);
    const init_expected_value = true;
    const re_set_expected_value = false;
    const expected_authority = authority_account.publicKey;
    const dummy_seeds_hash = await get_dummy_seeds_hash(2);
    const [state_pda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from(dummy_seeds_hash)],
      program.programId);

    try {
      await program.methods
        .get(dummy_seeds_hash)
        .accounts({state: state_pda})
        .rpc();
    } catch (error) {
        expect(error.error).to.deep.equal(account_not_initialized_expected_error.error);
      };

      await program.methods
      .initialize(dummy_seeds_hash, init_expected_value)
      .accounts({
        state: state_pda,
        authority: expected_authority,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([authority_account])
      .rpc();

    const initialized_state_data = await program.account.state.fetch(state_pda);
    
    expect(initialized_state_data.value).to.deep.equal(init_expected_value);
    expect(initialized_state_data.authority).to.deep.equal(expected_authority)

    // re-set
    await program.methods
      .set(dummy_seeds_hash, re_set_expected_value)
      .accounts({
        state: state_pda,
        authority: expected_authority,
      })
      .signers([authority_account])
      .rpc();

    const re_set_state_data = await program.account.state.fetch(state_pda);

    expect(re_set_state_data.value).to.deep.equal(re_set_expected_value);
  });

  it("re-set-value-with-incorrect-authority", async () => {
    const authority_account = await new_keypair_with_sol(3);
    const imposter_authority_account = await new_keypair_with_sol(3);
    const init_expected_value = true;
    const re_set_expected_value = false;
    const expected_authority = authority_account.publicKey;
    const imposter_authority = imposter_authority_account.publicKey;
    const dummy_seeds_hash = await get_dummy_seeds_hash(3);
    const [state_pda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from(dummy_seeds_hash)],
      program.programId);

    try {
      await program.methods
        .get(dummy_seeds_hash)
        .accounts({state: state_pda})
        .rpc();
    } catch (error) {
        expect(error.error).to.deep.equal(account_not_initialized_expected_error.error);
      };

      await program.methods
      .initialize(dummy_seeds_hash, init_expected_value)
      .accounts({
        state: state_pda,
        authority: expected_authority,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([authority_account])
      .rpc();

    const initialized_state_data = await program.account.state.fetch(state_pda);
    
    expect(initialized_state_data.value).to.deep.equal(init_expected_value);
    expect(initialized_state_data.authority).to.deep.equal(expected_authority)

    // re-set with incorrect authority
    try {
      await program.methods
      .set(dummy_seeds_hash, re_set_expected_value)
      .accounts({
        state: state_pda,
        authority: imposter_authority,
      })
      .signers([imposter_authority_account])
      .rpc();
    } catch (error) {
        expect(error.error.errorCode)
          .to.deep.equal(imposter_authority_expected_error_code_msg.error.errorCode);
        expect(error.error.errorMessage)
          .to.deep.equal(imposter_authority_expected_error_code_msg.error.errorMessage);

        // make sure that value hasn't change
        const _state_data = await program.account.state.fetch(state_pda);
        expect(_state_data.value).to.deep.equal(init_expected_value);
    };
  });

  it("delete-with-correct-authority", async () => {
    const authority_account = await new_keypair_with_sol(3);
    const init_expected_value = true;
    const re_set_expected_value = false;
    const expected_authority = authority_account.publicKey;
    const dummy_seeds_hash = await get_dummy_seeds_hash(4);
    const [state_pda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from(dummy_seeds_hash)],
      program.programId);

    try {
      await program.methods
        .get(dummy_seeds_hash)
        .accounts({state: state_pda})
        .rpc();
    } catch (error) {
        expect(error.error).to.deep.equal(account_not_initialized_expected_error.error);
      };

      await program.methods
      .initialize(dummy_seeds_hash, init_expected_value)
      .accounts({
        state: state_pda,
        authority: expected_authority,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([authority_account])
      .rpc();

    const initialized_state_data = await program.account.state.fetch(state_pda);
    
    expect(initialized_state_data.value).to.deep.equal(init_expected_value);
    expect(initialized_state_data.authority).to.deep.equal(expected_authority)

    // re-set
    await program.methods
      .set(dummy_seeds_hash, re_set_expected_value)
      .accounts({
        state: state_pda,
        authority: expected_authority,
      })
      .signers([authority_account])
      .rpc();

    const re_set_state_data = await program.account.state.fetch(state_pda);
    expect(re_set_state_data.value).to.deep.equal(re_set_expected_value);

    // delete
    await program.methods
      .delete(dummy_seeds_hash)
      .accounts({
        state: state_pda,
        authority: expected_authority,
      })
      .signers([authority_account])
      .rpc();
    
    // confirm that it was deleted
    try {
      await program.methods
        .get(dummy_seeds_hash)
        .accounts({state: state_pda})
        .rpc();
    } catch (error) {
        expect(error.error).to.deep.equal(account_not_initialized_expected_error.error);
    };
  });

  it("delete-with-incorrect-authority", async () => {
    const authority_account = await new_keypair_with_sol(3);
    const imposter_authority_account = await new_keypair_with_sol(3);
    const init_expected_value = true;
    const expected_authority = authority_account.publicKey;
    const imposter_authority = authority_account.publicKey;
    const dummy_seeds_hash = await get_dummy_seeds_hash(5);
    const [state_pda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from(dummy_seeds_hash)],
      program.programId);

    try {
      await program.methods
        .get(dummy_seeds_hash)
        .accounts({state: state_pda})
        .rpc();
    } catch (error) {
        expect(error.error).to.deep.equal(account_not_initialized_expected_error.error);
      };

      await program.methods
      .initialize(dummy_seeds_hash, init_expected_value)
      .accounts({
        state: state_pda,
        authority: expected_authority,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([authority_account])
      .rpc();

    const initialized_state_data = await program.account.state.fetch(state_pda);
    
    expect(initialized_state_data.value).to.deep.equal(init_expected_value);
    expect(initialized_state_data.authority).to.deep.equal(expected_authority)

    // attempt to delete
    try {
      await program.methods
      .delete(dummy_seeds_hash)
      .accounts({
        state: state_pda,
        authority: imposter_authority,
      })
      .signers([imposter_authority_account])
      .rpc();
    } catch (error) {
        expect(error === "Error: unknown signer: 7BwWMqYckefLVmQGQRF5CsAR4VvYGah3aW5tL3ScLUEC");
    }
    
    // confirm that it wasn't deleted
    expect(initialized_state_data).to.deep.equal(await program.account.state.fetch(state_pda))
  });

  it("delete-non-existent-account", async () => {
    const authority_account = await new_keypair_with_sol(3);
    const dummy_seeds_hash = await get_dummy_seeds_hash(6);
    const [state_pda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from(dummy_seeds_hash)],
      program.programId);

    try {
      await program.methods
      .delete(dummy_seeds_hash)
      .accounts({
        state: state_pda,
        authority: authority_account.publicKey,
      })
      .signers([authority_account])
      .rpc();
    } catch (error) {
      expect(error.error).to.deep.equal(account_not_initialized_expected_error.error);
    }
  });
});