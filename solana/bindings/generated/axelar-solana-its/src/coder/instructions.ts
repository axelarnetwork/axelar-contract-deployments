// @ts-nocheck
import * as B from "@native-to-anchor/buffer-layout";
import { Idl, InstructionCoder } from "@coral-xyz/anchor";

export class AxelarSolanaItsInstructionCoder implements InstructionCoder {
  constructor(_idl: Idl) {}

  encode(ixName: string, ix: any): Buffer {
    switch (ixName) {
      case "initialize": {
        return encodeInitialize(ix);
      }
      case "setPauseStatus": {
        return encodeSetPauseStatus(ix);
      }
      case "setTrustedChain": {
        return encodeSetTrustedChain(ix);
      }
      case "removeTrustedChain": {
        return encodeRemoveTrustedChain(ix);
      }
      case "approveDeployRemoteInterchainToken": {
        return encodeApproveDeployRemoteInterchainToken(ix);
      }
      case "revokeDeployRemoteInterchainToken": {
        return encodeRevokeDeployRemoteInterchainToken(ix);
      }
      case "registerCanonicalInterchainToken": {
        return encodeRegisterCanonicalInterchainToken(ix);
      }
      case "deployRemoteCanonicalInterchainToken": {
        return encodeDeployRemoteCanonicalInterchainToken(ix);
      }
      case "interchainTransfer": {
        return encodeInterchainTransfer(ix);
      }
      case "deployInterchainToken": {
        return encodeDeployInterchainToken(ix);
      }
      case "deployRemoteInterchainToken": {
        return encodeDeployRemoteInterchainToken(ix);
      }
      case "deployRemoteInterchainTokenWithMinter": {
        return encodeDeployRemoteInterchainTokenWithMinter(ix);
      }
      case "registerTokenMetadata": {
        return encodeRegisterTokenMetadata(ix);
      }
      case "registerCustomToken": {
        return encodeRegisterCustomToken(ix);
      }
      case "linkToken": {
        return encodeLinkToken(ix);
      }
      case "callContractWithInterchainToken": {
        return encodeCallContractWithInterchainToken(ix);
      }
      case "callContractWithInterchainTokenOffchainData": {
        return encodeCallContractWithInterchainTokenOffchainData(ix);
      }
      case "setFlowLimit": {
        return encodeSetFlowLimit(ix);
      }
      case "operatorTransferOperatorship": {
        return encodeOperatorTransferOperatorship(ix);
      }
      case "operatorProposeOperatorship": {
        return encodeOperatorProposeOperatorship(ix);
      }
      case "operatorAcceptOperatorship": {
        return encodeOperatorAcceptOperatorship(ix);
      }
      case "tokenManagerAddFlowLimiter": {
        return encodeTokenManagerAddFlowLimiter(ix);
      }
      case "tokenManagerRemoveFlowLimiter": {
        return encodeTokenManagerRemoveFlowLimiter(ix);
      }
      case "tokenManagerSetFlowLimit": {
        return encodeTokenManagerSetFlowLimit(ix);
      }
      case "tokenManagerTransferOperatorship": {
        return encodeTokenManagerTransferOperatorship(ix);
      }
      case "tokenManagerProposeOperatorship": {
        return encodeTokenManagerProposeOperatorship(ix);
      }
      case "tokenManagerAcceptOperatorship": {
        return encodeTokenManagerAcceptOperatorship(ix);
      }
      case "tokenManagerHandOverMintAuthority": {
        return encodeTokenManagerHandOverMintAuthority(ix);
      }
      case "interchainTokenMint": {
        return encodeInterchainTokenMint(ix);
      }
      case "interchainTokenTransferMintership": {
        return encodeInterchainTokenTransferMintership(ix);
      }
      case "interchainTokenProposeMintership": {
        return encodeInterchainTokenProposeMintership(ix);
      }
      case "interchainTokenAcceptMintership": {
        return encodeInterchainTokenAcceptMintership(ix);
      }

      default: {
        throw new Error(`Invalid instruction: ${ixName}`);
      }
    }
  }

  encodeState(_ixName: string, _ix: any): Buffer {
    throw new Error("AxelarSolanaIts does not have state");
  }
}

function encodeInitialize({ chainName, itsHubAddress }: any): Buffer {
  return encodeData(
    { initialize: { chainName, itsHubAddress } },
    1 + 4 + chainName.length + 4 + itsHubAddress.length
  );
}

function encodeSetPauseStatus({ paused }: any): Buffer {
  return encodeData({ setPauseStatus: { paused } }, 1 + 1);
}

function encodeSetTrustedChain({ chainName }: any): Buffer {
  return encodeData(
    { setTrustedChain: { chainName } },
    1 + 4 + chainName.length
  );
}

function encodeRemoveTrustedChain({ chainName }: any): Buffer {
  return encodeData(
    { removeTrustedChain: { chainName } },
    1 + 4 + chainName.length
  );
}

function encodeApproveDeployRemoteInterchainToken({
  deployer,
  salt,
  destinationChain,
  destinationMinter,
}: any): Buffer {
  return encodeData(
    {
      approveDeployRemoteInterchainToken: {
        deployer,
        salt,
        destinationChain,
        destinationMinter,
      },
    },
    1 + 32 + 1 * 32 + 4 + destinationChain.length + 4 + destinationMinter.length
  );
}

function encodeRevokeDeployRemoteInterchainToken({
  deployer,
  salt,
  destinationChain,
}: any): Buffer {
  return encodeData(
    { revokeDeployRemoteInterchainToken: { deployer, salt, destinationChain } },
    1 + 32 + 1 * 32 + 4 + destinationChain.length
  );
}

function encodeRegisterCanonicalInterchainToken({}: any): Buffer {
  return encodeData({ registerCanonicalInterchainToken: {} }, 1);
}

function encodeDeployRemoteCanonicalInterchainToken({
  destinationChain,
  gasValue,
  signingPdaBump,
}: any): Buffer {
  return encodeData(
    {
      deployRemoteCanonicalInterchainToken: {
        destinationChain,
        gasValue,
        signingPdaBump,
      },
    },
    1 + 4 + destinationChain.length + 8 + 1
  );
}

function encodeInterchainTransfer({
  tokenId,
  destinationChain,
  destinationAddress,
  amount,
  gasValue,
  signingPdaBump,
}: any): Buffer {
  return encodeData(
    {
      interchainTransfer: {
        tokenId,
        destinationChain,
        destinationAddress,
        amount,
        gasValue,
        signingPdaBump,
      },
    },
    1 +
      1 * 32 +
      4 +
      destinationChain.length +
      4 +
      destinationAddress.length +
      8 +
      8 +
      1
  );
}

function encodeDeployInterchainToken({
  salt,
  name,
  symbol,
  decimals,
}: any): Buffer {
  return encodeData(
    { deployInterchainToken: { salt, name, symbol, decimals } },
    1 + 1 * 32 + 4 + name.length + 4 + symbol.length + 1
  );
}

function encodeDeployRemoteInterchainToken({
  salt,
  destinationChain,
  gasValue,
  signingPdaBump,
}: any): Buffer {
  return encodeData(
    {
      deployRemoteInterchainToken: {
        salt,
        destinationChain,
        gasValue,
        signingPdaBump,
      },
    },
    1 + 1 * 32 + 4 + destinationChain.length + 8 + 1
  );
}

function encodeDeployRemoteInterchainTokenWithMinter({
  salt,
  destinationChain,
  destinationMinter,
  gasValue,
  signingPdaBump,
}: any): Buffer {
  return encodeData(
    {
      deployRemoteInterchainTokenWithMinter: {
        salt,
        destinationChain,
        destinationMinter,
        gasValue,
        signingPdaBump,
      },
    },
    1 +
      1 * 32 +
      4 +
      destinationChain.length +
      4 +
      destinationMinter.length +
      8 +
      1
  );
}

function encodeRegisterTokenMetadata({
  gasValue,
  signingPdaBump,
}: any): Buffer {
  return encodeData(
    { registerTokenMetadata: { gasValue, signingPdaBump } },
    1 + 8 + 1
  );
}

function encodeRegisterCustomToken({
  salt,
  tokenManagerType,
  operator,
}: any): Buffer {
  return encodeData(
    { registerCustomToken: { salt, tokenManagerType, operator } },
    1 +
      1 * 32 +
      (() => {
        switch (Object.keys(tokenManagerType)[0]) {
          case "nativeInterchainToken":
            return 1;
          case "mintBurnFrom":
            return 1;
          case "lockUnlock":
            return 1;
          case "lockUnlockFee":
            return 1;
          case "mintBurn":
            return 1;
        }
      })() +
      1 +
      (operator === null ? 0 : 32)
  );
}

function encodeLinkToken({
  salt,
  destinationChain,
  destinationTokenAddress,
  tokenManagerType,
  linkParams,
  gasValue,
  signingPdaBump,
}: any): Buffer {
  return encodeData(
    {
      linkToken: {
        salt,
        destinationChain,
        destinationTokenAddress,
        tokenManagerType,
        linkParams,
        gasValue,
        signingPdaBump,
      },
    },
    1 +
      1 * 32 +
      4 +
      destinationChain.length +
      4 +
      destinationTokenAddress.length +
      (() => {
        switch (Object.keys(tokenManagerType)[0]) {
          case "nativeInterchainToken":
            return 1;
          case "mintBurnFrom":
            return 1;
          case "lockUnlock":
            return 1;
          case "lockUnlockFee":
            return 1;
          case "mintBurn":
            return 1;
        }
      })() +
      4 +
      linkParams.length +
      8 +
      1
  );
}

function encodeCallContractWithInterchainToken({
  tokenId,
  destinationChain,
  destinationAddress,
  amount,
  data,
  gasValue,
  signingPdaBump,
}: any): Buffer {
  return encodeData(
    {
      callContractWithInterchainToken: {
        tokenId,
        destinationChain,
        destinationAddress,
        amount,
        data,
        gasValue,
        signingPdaBump,
      },
    },
    1 +
      1 * 32 +
      4 +
      destinationChain.length +
      4 +
      destinationAddress.length +
      8 +
      4 +
      data.length +
      8 +
      1
  );
}

function encodeCallContractWithInterchainTokenOffchainData({
  tokenId,
  destinationChain,
  destinationAddress,
  amount,
  payloadHash,
  gasValue,
  signingPdaBump,
}: any): Buffer {
  return encodeData(
    {
      callContractWithInterchainTokenOffchainData: {
        tokenId,
        destinationChain,
        destinationAddress,
        amount,
        payloadHash,
        gasValue,
        signingPdaBump,
      },
    },
    1 +
      1 * 32 +
      4 +
      destinationChain.length +
      4 +
      destinationAddress.length +
      8 +
      1 * 32 +
      8 +
      1
  );
}

function encodeSetFlowLimit({ flowLimit }: any): Buffer {
  return encodeData({ setFlowLimit: { flowLimit } }, 1 + 8);
}

function encodeOperatorTransferOperatorship({ inputs }: any): Buffer {
  return encodeData(
    { operatorTransferOperatorship: { inputs } },
    1 +
      (() => {
        switch (Object.keys(inputs.roles)[0]) {
          case "minter":
            return 1;
          case "operator":
            return 1;
          case "flowLimiter":
            return 1;
        }
      })() +
      1 +
      1 +
      (inputs.proposalPdaBump === null ? 0 : 1)
  );
}

function encodeOperatorProposeOperatorship({ inputs }: any): Buffer {
  return encodeData(
    { operatorProposeOperatorship: { inputs } },
    1 +
      (() => {
        switch (Object.keys(inputs.roles)[0]) {
          case "minter":
            return 1;
          case "operator":
            return 1;
          case "flowLimiter":
            return 1;
        }
      })() +
      1 +
      1 +
      (inputs.proposalPdaBump === null ? 0 : 1)
  );
}

function encodeOperatorAcceptOperatorship({ inputs }: any): Buffer {
  return encodeData(
    { operatorAcceptOperatorship: { inputs } },
    1 +
      (() => {
        switch (Object.keys(inputs.roles)[0]) {
          case "minter":
            return 1;
          case "operator":
            return 1;
          case "flowLimiter":
            return 1;
        }
      })() +
      1 +
      1 +
      (inputs.proposalPdaBump === null ? 0 : 1)
  );
}

function encodeTokenManagerAddFlowLimiter({ inputs }: any): Buffer {
  return encodeData(
    { tokenManagerAddFlowLimiter: { inputs } },
    1 +
      (() => {
        switch (Object.keys(inputs.roles)[0]) {
          case "minter":
            return 1;
          case "operator":
            return 1;
          case "flowLimiter":
            return 1;
        }
      })() +
      1 +
      1 +
      (inputs.proposalPdaBump === null ? 0 : 1)
  );
}

function encodeTokenManagerRemoveFlowLimiter({ inputs }: any): Buffer {
  return encodeData(
    { tokenManagerRemoveFlowLimiter: { inputs } },
    1 +
      (() => {
        switch (Object.keys(inputs.roles)[0]) {
          case "minter":
            return 1;
          case "operator":
            return 1;
          case "flowLimiter":
            return 1;
        }
      })() +
      1 +
      1 +
      (inputs.proposalPdaBump === null ? 0 : 1)
  );
}

function encodeTokenManagerSetFlowLimit({ flowLimit }: any): Buffer {
  return encodeData({ tokenManagerSetFlowLimit: { flowLimit } }, 1 + 8);
}

function encodeTokenManagerTransferOperatorship({ inputs }: any): Buffer {
  return encodeData(
    { tokenManagerTransferOperatorship: { inputs } },
    1 +
      (() => {
        switch (Object.keys(inputs.roles)[0]) {
          case "minter":
            return 1;
          case "operator":
            return 1;
          case "flowLimiter":
            return 1;
        }
      })() +
      1 +
      1 +
      (inputs.proposalPdaBump === null ? 0 : 1)
  );
}

function encodeTokenManagerProposeOperatorship({ inputs }: any): Buffer {
  return encodeData(
    { tokenManagerProposeOperatorship: { inputs } },
    1 +
      (() => {
        switch (Object.keys(inputs.roles)[0]) {
          case "minter":
            return 1;
          case "operator":
            return 1;
          case "flowLimiter":
            return 1;
        }
      })() +
      1 +
      1 +
      (inputs.proposalPdaBump === null ? 0 : 1)
  );
}

function encodeTokenManagerAcceptOperatorship({ inputs }: any): Buffer {
  return encodeData(
    { tokenManagerAcceptOperatorship: { inputs } },
    1 +
      (() => {
        switch (Object.keys(inputs.roles)[0]) {
          case "minter":
            return 1;
          case "operator":
            return 1;
          case "flowLimiter":
            return 1;
        }
      })() +
      1 +
      1 +
      (inputs.proposalPdaBump === null ? 0 : 1)
  );
}

function encodeTokenManagerHandOverMintAuthority({ tokenId }: any): Buffer {
  return encodeData(
    { tokenManagerHandOverMintAuthority: { tokenId } },
    1 + 1 * 32
  );
}

function encodeInterchainTokenMint({ amount }: any): Buffer {
  return encodeData({ interchainTokenMint: { amount } }, 1 + 8);
}

function encodeInterchainTokenTransferMintership({ inputs }: any): Buffer {
  return encodeData(
    { interchainTokenTransferMintership: { inputs } },
    1 +
      (() => {
        switch (Object.keys(inputs.roles)[0]) {
          case "minter":
            return 1;
          case "operator":
            return 1;
          case "flowLimiter":
            return 1;
        }
      })() +
      1 +
      1 +
      (inputs.proposalPdaBump === null ? 0 : 1)
  );
}

function encodeInterchainTokenProposeMintership({ inputs }: any): Buffer {
  return encodeData(
    { interchainTokenProposeMintership: { inputs } },
    1 +
      (() => {
        switch (Object.keys(inputs.roles)[0]) {
          case "minter":
            return 1;
          case "operator":
            return 1;
          case "flowLimiter":
            return 1;
        }
      })() +
      1 +
      1 +
      (inputs.proposalPdaBump === null ? 0 : 1)
  );
}

function encodeInterchainTokenAcceptMintership({ inputs }: any): Buffer {
  return encodeData(
    { interchainTokenAcceptMintership: { inputs } },
    1 +
      (() => {
        switch (Object.keys(inputs.roles)[0]) {
          case "minter":
            return 1;
          case "operator":
            return 1;
          case "flowLimiter":
            return 1;
        }
      })() +
      1 +
      1 +
      (inputs.proposalPdaBump === null ? 0 : 1)
  );
}

const LAYOUT = B.union(B.u8("instruction"));
LAYOUT.addVariant(
  0,
  B.struct([B.utf8Str("chainName"), B.utf8Str("itsHubAddress")]),
  "initialize"
);
LAYOUT.addVariant(1, B.struct([B.bool("paused")]), "setPauseStatus");
LAYOUT.addVariant(2, B.struct([B.utf8Str("chainName")]), "setTrustedChain");
LAYOUT.addVariant(3, B.struct([B.utf8Str("chainName")]), "removeTrustedChain");
LAYOUT.addVariant(
  4,
  B.struct([
    B.publicKey("deployer"),
    B.seq(B.u8(), 32, "salt"),
    B.utf8Str("destinationChain"),
    B.bytes("destinationMinter"),
  ]),
  "approveDeployRemoteInterchainToken"
);
LAYOUT.addVariant(
  5,
  B.struct([
    B.publicKey("deployer"),
    B.seq(B.u8(), 32, "salt"),
    B.utf8Str("destinationChain"),
  ]),
  "revokeDeployRemoteInterchainToken"
);
LAYOUT.addVariant(6, B.struct([]), "registerCanonicalInterchainToken");
LAYOUT.addVariant(
  7,
  B.struct([
    B.utf8Str("destinationChain"),
    B.u64("gasValue"),
    B.u8("signingPdaBump"),
  ]),
  "deployRemoteCanonicalInterchainToken"
);
LAYOUT.addVariant(
  8,
  B.struct([
    B.seq(B.u8(), 32, "tokenId"),
    B.utf8Str("destinationChain"),
    B.bytes("destinationAddress"),
    B.u64("amount"),
    B.u64("gasValue"),
    B.u8("signingPdaBump"),
  ]),
  "interchainTransfer"
);
LAYOUT.addVariant(
  9,
  B.struct([
    B.seq(B.u8(), 32, "salt"),
    B.utf8Str("name"),
    B.utf8Str("symbol"),
    B.u8("decimals"),
  ]),
  "deployInterchainToken"
);
LAYOUT.addVariant(
  10,
  B.struct([
    B.seq(B.u8(), 32, "salt"),
    B.utf8Str("destinationChain"),
    B.u64("gasValue"),
    B.u8("signingPdaBump"),
  ]),
  "deployRemoteInterchainToken"
);
LAYOUT.addVariant(
  11,
  B.struct([
    B.seq(B.u8(), 32, "salt"),
    B.utf8Str("destinationChain"),
    B.bytes("destinationMinter"),
    B.u64("gasValue"),
    B.u8("signingPdaBump"),
  ]),
  "deployRemoteInterchainTokenWithMinter"
);
LAYOUT.addVariant(
  12,
  B.struct([B.u64("gasValue"), B.u8("signingPdaBump")]),
  "registerTokenMetadata"
);
LAYOUT.addVariant(
  13,
  B.struct([
    B.seq(B.u8(), 32, "salt"),
    ((p: string) => {
      const U = B.union(B.u8("discriminator"), null, p);
      U.addVariant(0, B.struct([]), "nativeInterchainToken");
      U.addVariant(1, B.struct([]), "mintBurnFrom");
      U.addVariant(2, B.struct([]), "lockUnlock");
      U.addVariant(3, B.struct([]), "lockUnlockFee");
      U.addVariant(4, B.struct([]), "mintBurn");
      return U;
    })("tokenManagerType"),
    B.option(B.publicKey(), "operator"),
  ]),
  "registerCustomToken"
);
LAYOUT.addVariant(
  14,
  B.struct([
    B.seq(B.u8(), 32, "salt"),
    B.utf8Str("destinationChain"),
    B.bytes("destinationTokenAddress"),
    ((p: string) => {
      const U = B.union(B.u8("discriminator"), null, p);
      U.addVariant(0, B.struct([]), "nativeInterchainToken");
      U.addVariant(1, B.struct([]), "mintBurnFrom");
      U.addVariant(2, B.struct([]), "lockUnlock");
      U.addVariant(3, B.struct([]), "lockUnlockFee");
      U.addVariant(4, B.struct([]), "mintBurn");
      return U;
    })("tokenManagerType"),
    B.bytes("linkParams"),
    B.u64("gasValue"),
    B.u8("signingPdaBump"),
  ]),
  "linkToken"
);
LAYOUT.addVariant(
  15,
  B.struct([
    B.seq(B.u8(), 32, "tokenId"),
    B.utf8Str("destinationChain"),
    B.bytes("destinationAddress"),
    B.u64("amount"),
    B.bytes("data"),
    B.u64("gasValue"),
    B.u8("signingPdaBump"),
  ]),
  "callContractWithInterchainToken"
);
LAYOUT.addVariant(
  16,
  B.struct([
    B.seq(B.u8(), 32, "tokenId"),
    B.utf8Str("destinationChain"),
    B.bytes("destinationAddress"),
    B.u64("amount"),
    B.seq(B.u8(), 32, "payloadHash"),
    B.u64("gasValue"),
    B.u8("signingPdaBump"),
  ]),
  "callContractWithInterchainTokenOffchainData"
);
LAYOUT.addVariant(17, B.struct([B.u64("flowLimit")]), "setFlowLimit");
LAYOUT.addVariant(
  18,
  B.struct([
    B.struct(
      [
        ((p: string) => {
          const U = B.union(B.u8("discriminator"), null, p);
          U.addVariant(1, B.struct([]), "minter");
          U.addVariant(2, B.struct([]), "operator");
          U.addVariant(4, B.struct([]), "flowLimiter");
          return U;
        })("roles"),
        B.u8("destinationRolesPdaBump"),
        B.option(B.u8(), "proposalPdaBump"),
      ],
      "inputs"
    ),
  ]),
  "operatorTransferOperatorship"
);
LAYOUT.addVariant(
  19,
  B.struct([
    B.struct(
      [
        ((p: string) => {
          const U = B.union(B.u8("discriminator"), null, p);
          U.addVariant(1, B.struct([]), "minter");
          U.addVariant(2, B.struct([]), "operator");
          U.addVariant(4, B.struct([]), "flowLimiter");
          return U;
        })("roles"),
        B.u8("destinationRolesPdaBump"),
        B.option(B.u8(), "proposalPdaBump"),
      ],
      "inputs"
    ),
  ]),
  "operatorProposeOperatorship"
);
LAYOUT.addVariant(
  20,
  B.struct([
    B.struct(
      [
        ((p: string) => {
          const U = B.union(B.u8("discriminator"), null, p);
          U.addVariant(1, B.struct([]), "minter");
          U.addVariant(2, B.struct([]), "operator");
          U.addVariant(4, B.struct([]), "flowLimiter");
          return U;
        })("roles"),
        B.u8("destinationRolesPdaBump"),
        B.option(B.u8(), "proposalPdaBump"),
      ],
      "inputs"
    ),
  ]),
  "operatorAcceptOperatorship"
);
LAYOUT.addVariant(
  21,
  B.struct([
    B.struct(
      [
        ((p: string) => {
          const U = B.union(B.u8("discriminator"), null, p);
          U.addVariant(1, B.struct([]), "minter");
          U.addVariant(2, B.struct([]), "operator");
          U.addVariant(4, B.struct([]), "flowLimiter");
          return U;
        })("roles"),
        B.u8("destinationRolesPdaBump"),
        B.option(B.u8(), "proposalPdaBump"),
      ],
      "inputs"
    ),
  ]),
  "tokenManagerAddFlowLimiter"
);
LAYOUT.addVariant(
  22,
  B.struct([
    B.struct(
      [
        ((p: string) => {
          const U = B.union(B.u8("discriminator"), null, p);
          U.addVariant(1, B.struct([]), "minter");
          U.addVariant(2, B.struct([]), "operator");
          U.addVariant(4, B.struct([]), "flowLimiter");
          return U;
        })("roles"),
        B.u8("destinationRolesPdaBump"),
        B.option(B.u8(), "proposalPdaBump"),
      ],
      "inputs"
    ),
  ]),
  "tokenManagerRemoveFlowLimiter"
);
LAYOUT.addVariant(
  23,
  B.struct([B.u64("flowLimit")]),
  "tokenManagerSetFlowLimit"
);
LAYOUT.addVariant(
  24,
  B.struct([
    B.struct(
      [
        ((p: string) => {
          const U = B.union(B.u8("discriminator"), null, p);
          U.addVariant(1, B.struct([]), "minter");
          U.addVariant(2, B.struct([]), "operator");
          U.addVariant(4, B.struct([]), "flowLimiter");
          return U;
        })("roles"),
        B.u8("destinationRolesPdaBump"),
        B.option(B.u8(), "proposalPdaBump"),
      ],
      "inputs"
    ),
  ]),
  "tokenManagerTransferOperatorship"
);
LAYOUT.addVariant(
  25,
  B.struct([
    B.struct(
      [
        ((p: string) => {
          const U = B.union(B.u8("discriminator"), null, p);
          U.addVariant(1, B.struct([]), "minter");
          U.addVariant(2, B.struct([]), "operator");
          U.addVariant(4, B.struct([]), "flowLimiter");
          return U;
        })("roles"),
        B.u8("destinationRolesPdaBump"),
        B.option(B.u8(), "proposalPdaBump"),
      ],
      "inputs"
    ),
  ]),
  "tokenManagerProposeOperatorship"
);
LAYOUT.addVariant(
  26,
  B.struct([
    B.struct(
      [
        ((p: string) => {
          const U = B.union(B.u8("discriminator"), null, p);
          U.addVariant(1, B.struct([]), "minter");
          U.addVariant(2, B.struct([]), "operator");
          U.addVariant(4, B.struct([]), "flowLimiter");
          return U;
        })("roles"),
        B.u8("destinationRolesPdaBump"),
        B.option(B.u8(), "proposalPdaBump"),
      ],
      "inputs"
    ),
  ]),
  "tokenManagerAcceptOperatorship"
);
LAYOUT.addVariant(
  27,
  B.struct([B.seq(B.u8(), 32, "tokenId")]),
  "tokenManagerHandOverMintAuthority"
);
LAYOUT.addVariant(28, B.struct([B.u64("amount")]), "interchainTokenMint");
LAYOUT.addVariant(
  29,
  B.struct([
    B.struct(
      [
        ((p: string) => {
          const U = B.union(B.u8("discriminator"), null, p);
          U.addVariant(1, B.struct([]), "minter");
          U.addVariant(2, B.struct([]), "operator");
          U.addVariant(4, B.struct([]), "flowLimiter");
          return U;
        })("roles"),
        B.u8("destinationRolesPdaBump"),
        B.option(B.u8(), "proposalPdaBump"),
      ],
      "inputs"
    ),
  ]),
  "interchainTokenTransferMintership"
);
LAYOUT.addVariant(
  30,
  B.struct([
    B.struct(
      [
        ((p: string) => {
          const U = B.union(B.u8("discriminator"), null, p);
          U.addVariant(1, B.struct([]), "minter");
          U.addVariant(2, B.struct([]), "operator");
          U.addVariant(4, B.struct([]), "flowLimiter");
          return U;
        })("roles"),
        B.u8("destinationRolesPdaBump"),
        B.option(B.u8(), "proposalPdaBump"),
      ],
      "inputs"
    ),
  ]),
  "interchainTokenProposeMintership"
);
LAYOUT.addVariant(
  31,
  B.struct([
    B.struct(
      [
        ((p: string) => {
          const U = B.union(B.u8("discriminator"), null, p);
          U.addVariant(1, B.struct([]), "minter");
          U.addVariant(2, B.struct([]), "operator");
          U.addVariant(4, B.struct([]), "flowLimiter");
          return U;
        })("roles"),
        B.u8("destinationRolesPdaBump"),
        B.option(B.u8(), "proposalPdaBump"),
      ],
      "inputs"
    ),
  ]),
  "interchainTokenAcceptMintership"
);

function encodeData(ix: any, span: number): Buffer {
  const b = Buffer.alloc(span);
  LAYOUT.encode(ix, b);
  return b;
}
