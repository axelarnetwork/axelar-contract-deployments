import { PublicKey } from "@solana/web3.js";
import { Program, AnchorProvider } from "@coral-xyz/anchor";

import { AxelarSolanaMemoProgramCoder } from "./coder";

export const AXELAR_SOLANA_MEMO_PROGRAM_PROGRAM_ID = new PublicKey(
  "mem7LhKWbKydCPk1TwNzeCvVSpoVx2mqxNuvjGgWAbG"
);

interface GetProgramParams {
  programId?: PublicKey;
  provider?: AnchorProvider;
}

export function axelarSolanaMemoProgramProgram(
  params?: GetProgramParams
): Program<AxelarSolanaMemoProgram> {
  return new Program<AxelarSolanaMemoProgram>(
    IDL,
    params?.programId ?? AXELAR_SOLANA_MEMO_PROGRAM_PROGRAM_ID,
    params?.provider,
    new AxelarSolanaMemoProgramCoder(IDL)
  );
}

type AxelarSolanaMemoProgram = {
  version: "0.1.0";
  name: "axelar_solana_memo_program";
  instructions: [
    {
      name: "initialize";
      accounts: [
        {
          name: "payer";
          isMut: true;
          isSigner: false;
        },
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "counterPda";
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
          name: "counterPdaBump";
          type: "u8";
        }
      ];
    },
    {
      name: "processMemo";
      accounts: [
        {
          name: "counterPda";
          isMut: true;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "memo";
          type: "string";
        }
      ];
    },
    {
      name: "sendToGateway";
      accounts: [
        {
          name: "id";
          isMut: false;
          isSigner: false;
        },
        {
          name: "memoCounterPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "signingPda0";
          isMut: false;
          isSigner: false;
        },
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "gatewayProgram";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "memo";
          type: "string";
        },
        {
          name: "destinationChain";
          type: "string";
        },
        {
          name: "destinationAddress";
          type: "string";
        }
      ];
    },
    {
      name: "sendToGatewayOffchainMemo";
      accounts: [
        {
          name: "id";
          isMut: false;
          isSigner: false;
        },
        {
          name: "memoCounterPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "signingPda0";
          isMut: false;
          isSigner: false;
        },
        {
          name: "gatewayRootPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "gatewayProgram";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [
        {
          name: "memoHash";
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
          type: "string";
        }
      ];
    }
  ];
  accounts: [
    {
      name: "counter";
      type: {
        kind: "struct";
        fields: [
          {
            name: "counter";
            type: "u64";
          },
          {
            name: "bump";
            type: "u8";
          }
        ];
      };
    }
  ];
};

const IDL: AxelarSolanaMemoProgram = {
  version: "0.1.0",
  name: "axelar_solana_memo_program",
  instructions: [
    {
      name: "initialize",
      accounts: [
        {
          name: "payer",
          isMut: true,
          isSigner: false,
        },
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "counterPda",
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
          name: "counterPdaBump",
          type: "u8",
        },
      ],
    },
    {
      name: "processMemo",
      accounts: [
        {
          name: "counterPda",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "memo",
          type: "string",
        },
      ],
    },
    {
      name: "sendToGateway",
      accounts: [
        {
          name: "id",
          isMut: false,
          isSigner: false,
        },
        {
          name: "memoCounterPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "signingPda0",
          isMut: false,
          isSigner: false,
        },
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "gatewayProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "memo",
          type: "string",
        },
        {
          name: "destinationChain",
          type: "string",
        },
        {
          name: "destinationAddress",
          type: "string",
        },
      ],
    },
    {
      name: "sendToGatewayOffchainMemo",
      accounts: [
        {
          name: "id",
          isMut: false,
          isSigner: false,
        },
        {
          name: "memoCounterPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "signingPda0",
          isMut: false,
          isSigner: false,
        },
        {
          name: "gatewayRootPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "gatewayProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "memoHash",
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
          type: "string",
        },
      ],
    },
  ],
  accounts: [
    {
      name: "counter",
      type: {
        kind: "struct",
        fields: [
          {
            name: "counter",
            type: "u64",
          },
          {
            name: "bump",
            type: "u8",
          },
        ],
      },
    },
  ],
};
