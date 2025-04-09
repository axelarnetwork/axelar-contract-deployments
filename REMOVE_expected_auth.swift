[
    (
        Contract(Owner),
        AuthorizedInvocation {
            function: Contract((
                Contract(ITS),
                Symbol(migrate_token),
                Vec(
                    Ok(Bytes(obj#1331)),
                    Ok(Address(obj#1335)),
                    Ok(String(obj#1339))
                )
            )),
            sub_invocations: []
        }
    ),
    (
        Contract(    // 1. Wallet authorization for ITS.migrate_token
),
        AuthorizedInvocation {
            function: Contract((
                Contract(TokenManager),
                Symbol(upgrade),
                Vec(Ok(Bytes(obj#1351)))
            )),
            sub_invocations: []
        }
    ),
    (
        Contract(    // 1. Wallet authorization for ITS.migrate_token
),
        AuthorizedInvocation {
            function: Contract((
                Contract(TokenManager),
                Symbol(migrate),
                Vec(Ok(Void))
            )),
            sub_invocations: []
        }
    ),
    (
        Contract(    // 1. Wallet authorization for ITS.migrate_token
),
        AuthorizedInvocation {
            function: Contract((
                Contract(InterchainToken),
                Symbol(upgrade),
                Vec(Ok(Bytes(obj#1369)))
            )),
            sub_invocations: []
        }
    ),
    (
        Contract(    // 1. Wallet authorization for ITS.migrate_token
),
        AuthorizedInvocation {
            function: Contract((
                Contract(InterchainToken),
                Symbol(migrate),
                Vec(Ok(Void))
            )),
            sub_invocations: []
        }
    )
]
