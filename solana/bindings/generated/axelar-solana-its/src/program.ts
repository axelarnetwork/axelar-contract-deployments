import { PublicKey } from "@solana/web3.js";
import { Program, AnchorProvider } from "@coral-xyz/anchor";

import { AxelarSolanaItsCoder } from "./coder";

export const AXELAR_SOLANA_ITS_PROGRAM_ID = new PublicKey(
  "itsbPmAntHfec9PpLDoh9y3UiAEPT7DnzSvoJzdzZqd"
);

interface GetProgramParams {
  programId?: PublicKey;
  provider?: AnchorProvider;
}

export function axelarSolanaItsProgram(
  params?: GetProgramParams
): Program<AxelarSolanaIts> {
  return new Program<AxelarSolanaIts>(
    IDL,
    params?.programId ?? AXELAR_SOLANA_ITS_PROGRAM_ID,
    params?.provider,
    new AxelarSolanaItsCoder(IDL)
  );
}

type AxelarSolanaIts = {
  version: "0.1.0";
  name: "axelar_solana_its";
  instructions: [
    {
      name: "initialize";
      accounts: [
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "programDataAddress";
          isMut: false;
          isSigner: false;
        },
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "itsRootPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "operator";
          isMut: false;
          isSigner: false;
        },
        {
          name: "userRolesPda";
          isMut: true;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "chainName";
          type: "string";
        },
        {
          name: "itsHubAddress";
          type: "string";
        }
      ];
    },
    {
      name: "setPauseStatus";
      accounts: [
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "programDataAddress";
          isMut: false;
          isSigner: false;
        },
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "itsRootPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "paused";
          type: "bool";
        }
      ];
    },
    {
      name: "setTrustedChain";
      accounts: [
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "programDataAddress";
          isMut: false;
          isSigner: false;
        },
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "itsRootPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "chainName";
          type: "string";
        }
      ];
    },
    {
      name: "removeTrustedChain";
      accounts: [
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "programDataAddress";
          isMut: false;
          isSigner: false;
        },
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "itsRootPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "chainName";
          type: "string";
        }
      ];
    },
    {
      name: "approveDeployRemoteInterchainToken";
      accounts: [
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "tokenManagerPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "rolesPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "deployApprovalPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "deployer";
          type: "publicKey";
        },
        {
          name: "salt";
          type: {
            array: ["u8", 32];
          };
        },
        {
          name: "destinationChain";
          type: "string";
        },
        {
          name: "destinationMinter";
          type: "bytes";
        }
      ];
    },
    {
      name: "revokeDeployRemoteInterchainToken";
      accounts: [
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "deployApprovalPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "deployer";
          type: "publicKey";
        },
        {
          name: "salt";
          type: {
            array: ["u8", 32];
          };
        },
        {
          name: "destinationChain";
          type: "string";
        }
      ];
    },
    {
      name: "registerCanonicalInterchainToken";
      accounts: [
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "tokenMetadataAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "itsRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "tokenManagerPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "mint";
          isMut: true;
          isSigner: false;
        },
        {
          name: "tokenManagerAta";
          isMut: true;
          isSigner: false;
        },
        {
          name: "tokenProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "splAssociatedTokenAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "itsUserRolesPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "rent";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [];
    },
    {
      name: "deployRemoteCanonicalInterchainToken";
      accounts: [
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "mint";
          isMut: false;
          isSigner: false;
        },
        {
          name: "metadataAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "sysvarInstructions";
          isMut: false;
          isSigner: false;
        },
        {
          name: "mplTokenMetadata";
          isMut: false;
          isSigner: false;
        },
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "axelarSolanaGateway";
          isMut: false;
          isSigner: false;
        },
        {
          name: "gasConfigPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "gasService";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "itsRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "callContractSigningPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "id";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "destinationChain";
          type: "string";
        },
        {
          name: "gasValue";
          type: "u64";
        },
        {
          name: "signingPdaBump";
          type: "u8";
        }
      ];
    },
    {
      name: "interchainTransfer";
      accounts: [
        {
          name: "payer";
          isMut: false;
          isSigner: true;
        },
        {
          name: "authority";
          isMut: false;
          isSigner: true;
        },
        {
          name: "sourceAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "mint";
          isMut: true;
          isSigner: false;
        },
        {
          name: "tokenManagerPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "tokenManagerAta";
          isMut: true;
          isSigner: false;
        },
        {
          name: "tokenProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "flowSlotPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "axelarSolanaGateway";
          isMut: false;
          isSigner: false;
        },
        {
          name: "gasConfigPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "gasService";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "itsRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "callContractSigningPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "id";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "tokenId";
          type: {
            array: ["u8", 32];
          };
        },
        {
          name: "destinationChain";
          type: "string";
        },
        {
          name: "destinationAddress";
          type: "bytes";
        },
        {
          name: "amount";
          type: "u64";
        },
        {
          name: "gasValue";
          type: "u64";
        },
        {
          name: "signingPdaBump";
          type: "u8";
        }
      ];
    },
    {
      name: "deployInterchainToken";
      accounts: [
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "itsRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "tokenManagerPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "mint";
          isMut: true;
          isSigner: false;
        },
        {
          name: "tokenManagerAta";
          isMut: true;
          isSigner: false;
        },
        {
          name: "tokenProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "splAssociatedTokenAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "itsUserRolesPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "rent";
          isMut: false;
          isSigner: false;
        },
        {
          name: "sysvarInstructions";
          isMut: false;
          isSigner: false;
        },
        {
          name: "mplTokenMetadata";
          isMut: false;
          isSigner: false;
        },
        {
          name: "metadataAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "minter";
          isMut: false;
          isSigner: false;
        },
        {
          name: "minterRolesPda";
          isMut: true;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "salt";
          type: {
            array: ["u8", 32];
          };
        },
        {
          name: "name";
          type: "string";
        },
        {
          name: "symbol";
          type: "string";
        },
        {
          name: "decimals";
          type: "u8";
        }
      ];
    },
    {
      name: "deployRemoteInterchainToken";
      accounts: [
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "mint";
          isMut: false;
          isSigner: false;
        },
        {
          name: "metadataAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "sysvarInstructions";
          isMut: false;
          isSigner: false;
        },
        {
          name: "mplTokenMetadata";
          isMut: false;
          isSigner: false;
        },
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "axelarSolanaGateway";
          isMut: false;
          isSigner: false;
        },
        {
          name: "gasConfigPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "gasService";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "itsRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "callContractSigningPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "id";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "salt";
          type: {
            array: ["u8", 32];
          };
        },
        {
          name: "destinationChain";
          type: "string";
        },
        {
          name: "gasValue";
          type: "u64";
        },
        {
          name: "signingPdaBump";
          type: "u8";
        }
      ];
    },
    {
      name: "deployRemoteInterchainTokenWithMinter";
      accounts: [
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "mint";
          isMut: false;
          isSigner: false;
        },
        {
          name: "metadataAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "minter";
          isMut: false;
          isSigner: false;
        },
        {
          name: "deployApproval";
          isMut: true;
          isSigner: false;
        },
        {
          name: "minterRolesPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "tokenManagerPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "sysvarInstructions";
          isMut: false;
          isSigner: false;
        },
        {
          name: "mplTokenMetadata";
          isMut: false;
          isSigner: false;
        },
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "axelarSolanaGateway";
          isMut: false;
          isSigner: false;
        },
        {
          name: "gasConfigPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "gasService";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "itsRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "callContractSigningPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "id";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "salt";
          type: {
            array: ["u8", 32];
          };
        },
        {
          name: "destinationChain";
          type: "string";
        },
        {
          name: "destinationMinter";
          type: "bytes";
        },
        {
          name: "gasValue";
          type: "u64";
        },
        {
          name: "signingPdaBump";
          type: "u8";
        }
      ];
    },
    {
      name: "registerTokenMetadata";
      accounts: [
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "mint";
          isMut: false;
          isSigner: false;
        },
        {
          name: "tokenProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "axelarSolanaGateway";
          isMut: false;
          isSigner: false;
        },
        {
          name: "gasConfigPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "gasService";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "itsRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "callContractSigningPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "id";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "gasValue";
          type: "u64";
        },
        {
          name: "signingPdaBump";
          type: "u8";
        }
      ];
    },
    {
      name: "registerCustomToken";
      accounts: [
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "tokenMetadataAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "itsRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "tokenManagerPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "mint";
          isMut: true;
          isSigner: false;
        },
        {
          name: "tokenManagerAta";
          isMut: true;
          isSigner: false;
        },
        {
          name: "tokenProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "splAssociatedTokenAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "itsUserRolesPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "rent";
          isMut: false;
          isSigner: false;
        },
        {
          name: "operator";
          isMut: true;
          isSigner: false;
          isOptional: true;
        },
        {
          name: "operatorRolesPda";
          isMut: true;
          isSigner: false;
          isOptional: true;
        }
      ];
      args: [
        {
          name: "salt";
          type: {
            array: ["u8", 32];
          };
        },
        {
          name: "tokenManagerType";
          type: {
            defined: "Type";
          };
        },
        {
          name: "operator";
          type: {
            option: "publicKey";
          };
        }
      ];
    },
    {
      name: "linkToken";
      accounts: [
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "tokenManagerPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "axelarSolanaGateway";
          isMut: false;
          isSigner: false;
        },
        {
          name: "gasConfigPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "gasService";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "itsRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "callContractSigningPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "id";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "salt";
          type: {
            array: ["u8", 32];
          };
        },
        {
          name: "destinationChain";
          type: "string";
        },
        {
          name: "destinationTokenAddress";
          type: "bytes";
        },
        {
          name: "tokenManagerType";
          type: {
            defined: "Type";
          };
        },
        {
          name: "linkParams";
          type: "bytes";
        },
        {
          name: "gasValue";
          type: "u64";
        },
        {
          name: "signingPdaBump";
          type: "u8";
        }
      ];
    },
    {
      name: "callContractWithInterchainToken";
      accounts: [
        {
          name: "payer";
          isMut: false;
          isSigner: true;
        },
        {
          name: "authority";
          isMut: false;
          isSigner: true;
        },
        {
          name: "sourceAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "mint";
          isMut: true;
          isSigner: false;
        },
        {
          name: "tokenManagerPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "tokenManagerAta";
          isMut: true;
          isSigner: false;
        },
        {
          name: "tokenProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "flowSlotPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "axelarSolanaGateway";
          isMut: false;
          isSigner: false;
        },
        {
          name: "gasConfigPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "gasService";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "itsRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "callContractSigningPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "id";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "tokenId";
          type: {
            array: ["u8", 32];
          };
        },
        {
          name: "destinationChain";
          type: "string";
        },
        {
          name: "destinationAddress";
          type: "bytes";
        },
        {
          name: "amount";
          type: "u64";
        },
        {
          name: "data";
          type: "bytes";
        },
        {
          name: "gasValue";
          type: "u64";
        },
        {
          name: "signingPdaBump";
          type: "u8";
        }
      ];
    },
    {
      name: "callContractWithInterchainTokenOffchainData";
      accounts: [
        {
          name: "payer";
          isMut: false;
          isSigner: true;
        },
        {
          name: "authority";
          isMut: false;
          isSigner: true;
        },
        {
          name: "sourceAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "mint";
          isMut: true;
          isSigner: false;
        },
        {
          name: "tokenManagerPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "tokenManagerAta";
          isMut: true;
          isSigner: false;
        },
        {
          name: "tokenProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "flowSlotPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "axelarSolanaGateway";
          isMut: false;
          isSigner: false;
        },
        {
          name: "gasConfigPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "gasService";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "itsRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "callContractSigningPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "id";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "tokenId";
          type: {
            array: ["u8", 32];
          };
        },
        {
          name: "destinationChain";
          type: "string";
        },
        {
          name: "destinationAddress";
          type: "bytes";
        },
        {
          name: "amount";
          type: "u64";
        },
        {
          name: "payloadHash";
          type: {
            array: ["u8", 32];
          };
        },
        {
          name: "gasValue";
          type: "u64";
        },
        {
          name: "signingPdaBump";
          type: "u8";
        }
      ];
    },
    {
      name: "setFlowLimit";
      accounts: [
        {
          name: "payer";
          isMut: false;
          isSigner: true;
        },
        {
          name: "itsRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "tokenManagerPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "itsUserRolesPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "tokenManagerUserRolesPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "flowLimit";
          type: "u64";
        }
      ];
    },
    {
      name: "operatorTransferOperatorship";
      accounts: [
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "payerRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "resource";
          isMut: false;
          isSigner: false;
        },
        {
          name: "destinationUserAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "destinationRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "originUserAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "originRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "proposalAccount";
          isMut: true;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "inputs";
          type: {
            defined: "RoleManagementInstructionInputs";
          };
        }
      ];
    },
    {
      name: "operatorProposeOperatorship";
      accounts: [
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "payerRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "resource";
          isMut: false;
          isSigner: false;
        },
        {
          name: "destinationUserAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "destinationRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "originUserAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "originRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "proposalAccount";
          isMut: true;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "inputs";
          type: {
            defined: "RoleManagementInstructionInputs";
          };
        }
      ];
    },
    {
      name: "operatorAcceptOperatorship";
      accounts: [
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "payerRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "resource";
          isMut: false;
          isSigner: false;
        },
        {
          name: "destinationUserAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "destinationRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "originUserAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "originRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "proposalAccount";
          isMut: true;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "inputs";
          type: {
            defined: "RoleManagementInstructionInputs";
          };
        }
      ];
    },
    {
      name: "tokenManagerAddFlowLimiter";
      accounts: [
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "payerRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "resource";
          isMut: false;
          isSigner: false;
        },
        {
          name: "destinationUserAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "destinationRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "originUserAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "originRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "proposalAccount";
          isMut: true;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "inputs";
          type: {
            defined: "RoleManagementInstructionInputs";
          };
        }
      ];
    },
    {
      name: "tokenManagerRemoveFlowLimiter";
      accounts: [
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "payerRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "resource";
          isMut: false;
          isSigner: false;
        },
        {
          name: "destinationUserAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "destinationRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "originUserAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "originRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "proposalAccount";
          isMut: true;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "inputs";
          type: {
            defined: "RoleManagementInstructionInputs";
          };
        }
      ];
    },
    {
      name: "tokenManagerSetFlowLimit";
      accounts: [
        {
          name: "payer";
          isMut: false;
          isSigner: true;
        },
        {
          name: "itsRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "tokenManagerPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "tokenManagerUserRolesPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "itsUserRolesPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "flowLimit";
          type: "u64";
        }
      ];
    },
    {
      name: "tokenManagerTransferOperatorship";
      accounts: [
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "payerRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "resource";
          isMut: false;
          isSigner: false;
        },
        {
          name: "destinationUserAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "destinationRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "originUserAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "originRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "proposalAccount";
          isMut: true;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "inputs";
          type: {
            defined: "RoleManagementInstructionInputs";
          };
        }
      ];
    },
    {
      name: "tokenManagerProposeOperatorship";
      accounts: [
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "payerRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "resource";
          isMut: false;
          isSigner: false;
        },
        {
          name: "destinationUserAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "destinationRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "originUserAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "originRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "proposalAccount";
          isMut: true;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "inputs";
          type: {
            defined: "RoleManagementInstructionInputs";
          };
        }
      ];
    },
    {
      name: "tokenManagerAcceptOperatorship";
      accounts: [
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "payerRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "resource";
          isMut: false;
          isSigner: false;
        },
        {
          name: "destinationUserAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "destinationRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "originUserAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "originRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "proposalAccount";
          isMut: true;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "inputs";
          type: {
            defined: "RoleManagementInstructionInputs";
          };
        }
      ];
    },
    {
      name: "tokenManagerHandOverMintAuthority";
      accounts: [
        {
          name: "payer";
          isMut: false;
          isSigner: true;
        },
        {
          name: "mint";
          isMut: true;
          isSigner: false;
        },
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "itsRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "tokenManagerPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "minterRolesPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "tokenProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "tokenId";
          type: {
            array: ["u8", 32];
          };
        }
      ];
    },
    {
      name: "interchainTokenMint";
      accounts: [
        {
          name: "mint";
          isMut: true;
          isSigner: false;
        },
        {
          name: "destinationAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "itsRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "tokenManagerPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "minter";
          isMut: false;
          isSigner: false;
        },
        {
          name: "minterRolesPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "amount";
          type: "u64";
        }
      ];
    },
    {
      name: "interchainTokenTransferMintership";
      accounts: [
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "payerRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "resource";
          isMut: false;
          isSigner: false;
        },
        {
          name: "destinationUserAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "destinationRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "originUserAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "originRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "proposalAccount";
          isMut: true;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "inputs";
          type: {
            defined: "RoleManagementInstructionInputs";
          };
        }
      ];
    },
    {
      name: "interchainTokenProposeMintership";
      accounts: [
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "payerRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "resource";
          isMut: false;
          isSigner: false;
        },
        {
          name: "destinationUserAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "destinationRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "originUserAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "originRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "proposalAccount";
          isMut: true;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "inputs";
          type: {
            defined: "RoleManagementInstructionInputs";
          };
        }
      ];
    },
    {
      name: "interchainTokenAcceptMintership";
      accounts: [
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "payer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "payerRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "resource";
          isMut: false;
          isSigner: false;
        },
        {
          name: "destinationUserAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "destinationRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "originUserAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "originRolesAccount";
          isMut: false;
          isSigner: false;
        },
        {
          name: "proposalAccount";
          isMut: true;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "inputs";
          type: {
            defined: "RoleManagementInstructionInputs";
          };
        }
      ];
    }
  ];
  types: [
    {
      name: "RoleManagementInstructionInputs";
      type: {
        kind: "struct";
        fields: [
          {
            name: "roles";
            type: {
              defined: "Roles";
            };
          },
          {
            name: "destinationRolesPdaBump";
            type: "u8";
          },
          {
            name: "proposalPdaBump";
            type: {
              option: "u8";
            };
          }
        ];
      };
    },
    {
      name: "Type";
      type: {
        kind: "enum";
        variants: [
          {
            name: "NativeInterchainToken";
          },
          {
            name: "MintBurnFrom";
          },
          {
            name: "LockUnlock";
          },
          {
            name: "LockUnlockFee";
          },
          {
            name: "MintBurn";
          }
        ];
      };
    },
    {
      name: "Roles";
      type: {
        kind: "enum";
        variants: [
          {
            name: "Minter";
          },
          {
            name: "Operator";
          },
          {
            name: "FlowLimiter";
          }
        ];
      };
    }
  ];
};

const IDL: AxelarSolanaIts = {
  version: "0.1.0",
  name: "axelar_solana_its",
  instructions: [
    {
      name: "initialize",
      accounts: [
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "programDataAddress",
          isMut: false,
          isSigner: false,
        },
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "itsRootPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "operator",
          isMut: false,
          isSigner: false,
        },
        {
          name: "userRolesPda",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "chainName",
          type: "string",
        },
        {
          name: "itsHubAddress",
          type: "string",
        },
      ],
    },
    {
      name: "setPauseStatus",
      accounts: [
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "programDataAddress",
          isMut: false,
          isSigner: false,
        },
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "itsRootPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "paused",
          type: "bool",
        },
      ],
    },
    {
      name: "setTrustedChain",
      accounts: [
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "programDataAddress",
          isMut: false,
          isSigner: false,
        },
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "itsRootPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "chainName",
          type: "string",
        },
      ],
    },
    {
      name: "removeTrustedChain",
      accounts: [
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "programDataAddress",
          isMut: false,
          isSigner: false,
        },
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "itsRootPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "chainName",
          type: "string",
        },
      ],
    },
    {
      name: "approveDeployRemoteInterchainToken",
      accounts: [
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "tokenManagerPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "rolesPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "deployApprovalPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "deployer",
          type: "publicKey",
        },
        {
          name: "salt",
          type: {
            array: ["u8", 32],
          },
        },
        {
          name: "destinationChain",
          type: "string",
        },
        {
          name: "destinationMinter",
          type: "bytes",
        },
      ],
    },
    {
      name: "revokeDeployRemoteInterchainToken",
      accounts: [
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "deployApprovalPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "deployer",
          type: "publicKey",
        },
        {
          name: "salt",
          type: {
            array: ["u8", 32],
          },
        },
        {
          name: "destinationChain",
          type: "string",
        },
      ],
    },
    {
      name: "registerCanonicalInterchainToken",
      accounts: [
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "tokenMetadataAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "itsRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "tokenManagerPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "mint",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenManagerAta",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "splAssociatedTokenAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "itsUserRolesPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "rent",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [],
    },
    {
      name: "deployRemoteCanonicalInterchainToken",
      accounts: [
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "mint",
          isMut: false,
          isSigner: false,
        },
        {
          name: "metadataAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "sysvarInstructions",
          isMut: false,
          isSigner: false,
        },
        {
          name: "mplTokenMetadata",
          isMut: false,
          isSigner: false,
        },
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "axelarSolanaGateway",
          isMut: false,
          isSigner: false,
        },
        {
          name: "gasConfigPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "gasService",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "itsRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "callContractSigningPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "id",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "destinationChain",
          type: "string",
        },
        {
          name: "gasValue",
          type: "u64",
        },
        {
          name: "signingPdaBump",
          type: "u8",
        },
      ],
    },
    {
      name: "interchainTransfer",
      accounts: [
        {
          name: "payer",
          isMut: false,
          isSigner: true,
        },
        {
          name: "authority",
          isMut: false,
          isSigner: true,
        },
        {
          name: "sourceAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "mint",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenManagerPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "tokenManagerAta",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "flowSlotPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "axelarSolanaGateway",
          isMut: false,
          isSigner: false,
        },
        {
          name: "gasConfigPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "gasService",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "itsRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "callContractSigningPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "id",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "tokenId",
          type: {
            array: ["u8", 32],
          },
        },
        {
          name: "destinationChain",
          type: "string",
        },
        {
          name: "destinationAddress",
          type: "bytes",
        },
        {
          name: "amount",
          type: "u64",
        },
        {
          name: "gasValue",
          type: "u64",
        },
        {
          name: "signingPdaBump",
          type: "u8",
        },
      ],
    },
    {
      name: "deployInterchainToken",
      accounts: [
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "itsRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "tokenManagerPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "mint",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenManagerAta",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "splAssociatedTokenAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "itsUserRolesPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "rent",
          isMut: false,
          isSigner: false,
        },
        {
          name: "sysvarInstructions",
          isMut: false,
          isSigner: false,
        },
        {
          name: "mplTokenMetadata",
          isMut: false,
          isSigner: false,
        },
        {
          name: "metadataAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "minter",
          isMut: false,
          isSigner: false,
        },
        {
          name: "minterRolesPda",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "salt",
          type: {
            array: ["u8", 32],
          },
        },
        {
          name: "name",
          type: "string",
        },
        {
          name: "symbol",
          type: "string",
        },
        {
          name: "decimals",
          type: "u8",
        },
      ],
    },
    {
      name: "deployRemoteInterchainToken",
      accounts: [
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "mint",
          isMut: false,
          isSigner: false,
        },
        {
          name: "metadataAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "sysvarInstructions",
          isMut: false,
          isSigner: false,
        },
        {
          name: "mplTokenMetadata",
          isMut: false,
          isSigner: false,
        },
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "axelarSolanaGateway",
          isMut: false,
          isSigner: false,
        },
        {
          name: "gasConfigPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "gasService",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "itsRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "callContractSigningPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "id",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "salt",
          type: {
            array: ["u8", 32],
          },
        },
        {
          name: "destinationChain",
          type: "string",
        },
        {
          name: "gasValue",
          type: "u64",
        },
        {
          name: "signingPdaBump",
          type: "u8",
        },
      ],
    },
    {
      name: "deployRemoteInterchainTokenWithMinter",
      accounts: [
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "mint",
          isMut: false,
          isSigner: false,
        },
        {
          name: "metadataAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "minter",
          isMut: false,
          isSigner: false,
        },
        {
          name: "deployApproval",
          isMut: true,
          isSigner: false,
        },
        {
          name: "minterRolesPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "tokenManagerPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "sysvarInstructions",
          isMut: false,
          isSigner: false,
        },
        {
          name: "mplTokenMetadata",
          isMut: false,
          isSigner: false,
        },
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "axelarSolanaGateway",
          isMut: false,
          isSigner: false,
        },
        {
          name: "gasConfigPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "gasService",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "itsRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "callContractSigningPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "id",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "salt",
          type: {
            array: ["u8", 32],
          },
        },
        {
          name: "destinationChain",
          type: "string",
        },
        {
          name: "destinationMinter",
          type: "bytes",
        },
        {
          name: "gasValue",
          type: "u64",
        },
        {
          name: "signingPdaBump",
          type: "u8",
        },
      ],
    },
    {
      name: "registerTokenMetadata",
      accounts: [
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "mint",
          isMut: false,
          isSigner: false,
        },
        {
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "axelarSolanaGateway",
          isMut: false,
          isSigner: false,
        },
        {
          name: "gasConfigPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "gasService",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "itsRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "callContractSigningPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "id",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "gasValue",
          type: "u64",
        },
        {
          name: "signingPdaBump",
          type: "u8",
        },
      ],
    },
    {
      name: "registerCustomToken",
      accounts: [
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "tokenMetadataAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "itsRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "tokenManagerPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "mint",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenManagerAta",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "splAssociatedTokenAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "itsUserRolesPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "rent",
          isMut: false,
          isSigner: false,
        },
        {
          name: "operator",
          isMut: true,
          isSigner: false,
          isOptional: true,
        },
        {
          name: "operatorRolesPda",
          isMut: true,
          isSigner: false,
          isOptional: true,
        },
      ],
      args: [
        {
          name: "salt",
          type: {
            array: ["u8", 32],
          },
        },
        {
          name: "tokenManagerType",
          type: {
            defined: "Type",
          },
        },
        {
          name: "operator",
          type: {
            option: "publicKey",
          },
        },
      ],
    },
    {
      name: "linkToken",
      accounts: [
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "tokenManagerPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "axelarSolanaGateway",
          isMut: false,
          isSigner: false,
        },
        {
          name: "gasConfigPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "gasService",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "itsRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "callContractSigningPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "id",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "salt",
          type: {
            array: ["u8", 32],
          },
        },
        {
          name: "destinationChain",
          type: "string",
        },
        {
          name: "destinationTokenAddress",
          type: "bytes",
        },
        {
          name: "tokenManagerType",
          type: {
            defined: "Type",
          },
        },
        {
          name: "linkParams",
          type: "bytes",
        },
        {
          name: "gasValue",
          type: "u64",
        },
        {
          name: "signingPdaBump",
          type: "u8",
        },
      ],
    },
    {
      name: "callContractWithInterchainToken",
      accounts: [
        {
          name: "payer",
          isMut: false,
          isSigner: true,
        },
        {
          name: "authority",
          isMut: false,
          isSigner: true,
        },
        {
          name: "sourceAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "mint",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenManagerPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "tokenManagerAta",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "flowSlotPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "axelarSolanaGateway",
          isMut: false,
          isSigner: false,
        },
        {
          name: "gasConfigPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "gasService",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "itsRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "callContractSigningPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "id",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "tokenId",
          type: {
            array: ["u8", 32],
          },
        },
        {
          name: "destinationChain",
          type: "string",
        },
        {
          name: "destinationAddress",
          type: "bytes",
        },
        {
          name: "amount",
          type: "u64",
        },
        {
          name: "data",
          type: "bytes",
        },
        {
          name: "gasValue",
          type: "u64",
        },
        {
          name: "signingPdaBump",
          type: "u8",
        },
      ],
    },
    {
      name: "callContractWithInterchainTokenOffchainData",
      accounts: [
        {
          name: "payer",
          isMut: false,
          isSigner: true,
        },
        {
          name: "authority",
          isMut: false,
          isSigner: true,
        },
        {
          name: "sourceAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "mint",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenManagerPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "tokenManagerAta",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "flowSlotPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "axelarSolanaGateway",
          isMut: false,
          isSigner: false,
        },
        {
          name: "gasConfigPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "gasService",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "itsRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "callContractSigningPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "id",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "tokenId",
          type: {
            array: ["u8", 32],
          },
        },
        {
          name: "destinationChain",
          type: "string",
        },
        {
          name: "destinationAddress",
          type: "bytes",
        },
        {
          name: "amount",
          type: "u64",
        },
        {
          name: "payloadHash",
          type: {
            array: ["u8", 32],
          },
        },
        {
          name: "gasValue",
          type: "u64",
        },
        {
          name: "signingPdaBump",
          type: "u8",
        },
      ],
    },
    {
      name: "setFlowLimit",
      accounts: [
        {
          name: "payer",
          isMut: false,
          isSigner: true,
        },
        {
          name: "itsRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "tokenManagerPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "itsUserRolesPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "tokenManagerUserRolesPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "flowLimit",
          type: "u64",
        },
      ],
    },
    {
      name: "operatorTransferOperatorship",
      accounts: [
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "payerRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "resource",
          isMut: false,
          isSigner: false,
        },
        {
          name: "destinationUserAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "destinationRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "originUserAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "originRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "proposalAccount",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "inputs",
          type: {
            defined: "RoleManagementInstructionInputs",
          },
        },
      ],
    },
    {
      name: "operatorProposeOperatorship",
      accounts: [
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "payerRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "resource",
          isMut: false,
          isSigner: false,
        },
        {
          name: "destinationUserAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "destinationRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "originUserAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "originRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "proposalAccount",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "inputs",
          type: {
            defined: "RoleManagementInstructionInputs",
          },
        },
      ],
    },
    {
      name: "operatorAcceptOperatorship",
      accounts: [
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "payerRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "resource",
          isMut: false,
          isSigner: false,
        },
        {
          name: "destinationUserAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "destinationRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "originUserAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "originRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "proposalAccount",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "inputs",
          type: {
            defined: "RoleManagementInstructionInputs",
          },
        },
      ],
    },
    {
      name: "tokenManagerAddFlowLimiter",
      accounts: [
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "payerRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "resource",
          isMut: false,
          isSigner: false,
        },
        {
          name: "destinationUserAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "destinationRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "originUserAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "originRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "proposalAccount",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "inputs",
          type: {
            defined: "RoleManagementInstructionInputs",
          },
        },
      ],
    },
    {
      name: "tokenManagerRemoveFlowLimiter",
      accounts: [
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "payerRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "resource",
          isMut: false,
          isSigner: false,
        },
        {
          name: "destinationUserAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "destinationRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "originUserAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "originRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "proposalAccount",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "inputs",
          type: {
            defined: "RoleManagementInstructionInputs",
          },
        },
      ],
    },
    {
      name: "tokenManagerSetFlowLimit",
      accounts: [
        {
          name: "payer",
          isMut: false,
          isSigner: true,
        },
        {
          name: "itsRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "tokenManagerPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenManagerUserRolesPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "itsUserRolesPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "flowLimit",
          type: "u64",
        },
      ],
    },
    {
      name: "tokenManagerTransferOperatorship",
      accounts: [
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "payerRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "resource",
          isMut: false,
          isSigner: false,
        },
        {
          name: "destinationUserAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "destinationRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "originUserAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "originRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "proposalAccount",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "inputs",
          type: {
            defined: "RoleManagementInstructionInputs",
          },
        },
      ],
    },
    {
      name: "tokenManagerProposeOperatorship",
      accounts: [
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "payerRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "resource",
          isMut: false,
          isSigner: false,
        },
        {
          name: "destinationUserAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "destinationRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "originUserAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "originRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "proposalAccount",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "inputs",
          type: {
            defined: "RoleManagementInstructionInputs",
          },
        },
      ],
    },
    {
      name: "tokenManagerAcceptOperatorship",
      accounts: [
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "payerRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "resource",
          isMut: false,
          isSigner: false,
        },
        {
          name: "destinationUserAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "destinationRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "originUserAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "originRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "proposalAccount",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "inputs",
          type: {
            defined: "RoleManagementInstructionInputs",
          },
        },
      ],
    },
    {
      name: "tokenManagerHandOverMintAuthority",
      accounts: [
        {
          name: "payer",
          isMut: false,
          isSigner: true,
        },
        {
          name: "mint",
          isMut: true,
          isSigner: false,
        },
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "itsRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "tokenManagerPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "minterRolesPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "tokenId",
          type: {
            array: ["u8", 32],
          },
        },
      ],
    },
    {
      name: "interchainTokenMint",
      accounts: [
        {
          name: "mint",
          isMut: true,
          isSigner: false,
        },
        {
          name: "destinationAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "itsRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "tokenManagerPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "minter",
          isMut: false,
          isSigner: false,
        },
        {
          name: "minterRolesPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "amount",
          type: "u64",
        },
      ],
    },
    {
      name: "interchainTokenTransferMintership",
      accounts: [
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "payerRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "resource",
          isMut: false,
          isSigner: false,
        },
        {
          name: "destinationUserAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "destinationRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "originUserAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "originRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "proposalAccount",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "inputs",
          type: {
            defined: "RoleManagementInstructionInputs",
          },
        },
      ],
    },
    {
      name: "interchainTokenProposeMintership",
      accounts: [
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "payerRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "resource",
          isMut: false,
          isSigner: false,
        },
        {
          name: "destinationUserAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "destinationRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "originUserAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "originRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "proposalAccount",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "inputs",
          type: {
            defined: "RoleManagementInstructionInputs",
          },
        },
      ],
    },
    {
      name: "interchainTokenAcceptMintership",
      accounts: [
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "payer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "payerRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "resource",
          isMut: false,
          isSigner: false,
        },
        {
          name: "destinationUserAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "destinationRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "originUserAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "originRolesAccount",
          isMut: false,
          isSigner: false,
        },
        {
          name: "proposalAccount",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "inputs",
          type: {
            defined: "RoleManagementInstructionInputs",
          },
        },
      ],
    },
  ],
  types: [
    {
      name: "RoleManagementInstructionInputs",
      type: {
        kind: "struct",
        fields: [
          {
            name: "roles",
            type: {
              defined: "Roles",
            },
          },
          {
            name: "destinationRolesPdaBump",
            type: "u8",
          },
          {
            name: "proposalPdaBump",
            type: {
              option: "u8",
            },
          },
        ],
      },
    },
    {
      name: "Type",
      type: {
        kind: "enum",
        variants: [
          {
            name: "NativeInterchainToken",
          },
          {
            name: "MintBurnFrom",
          },
          {
            name: "LockUnlock",
          },
          {
            name: "LockUnlockFee",
          },
          {
            name: "MintBurn",
          },
        ],
      },
    },
    {
      name: "Roles",
      type: {
        kind: "enum",
        variants: [
          {
            name: "Minter",
          },
          {
            name: "Operator",
          },
          {
            name: "FlowLimiter",
          },
        ],
      },
    },
  ],
};
