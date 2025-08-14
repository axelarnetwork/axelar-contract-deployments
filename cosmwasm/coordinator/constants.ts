export const DEFAULTS = {
    serviceName: 'amplifier',
    votingThreshold: ['51', '100'] as [string, string],
    signingThreshold: ['51', '100'] as [string, string],
    blockExpiry: '10',
    confirmationHeight: 1000000,
    msgIdFormat: 'hex_tx_hash_and_event_index',
    addressFormat: 'eip55',
    verifierSetDiffThreshold: 1,
    encoder: 'abi',
    keyType: 'ecdsa',
    domainSeparator: '0x79191ee0824b0f995492dc4ac6e737040f4d9fd4501f6078e56671da70968259',
    proposalDeposit: '1000000000',
    minAddressLength: 39,
    maxAddressLength: 100,
    hexStringLength: 64,
    defaultSaltLength: 32,
};

export const CONTRACTS_TO_HANDLE = ['VotingVerifier', 'MultisigProver', 'Gateway'];
