
## Token Manager Deployment

The flow of deploying a token manager is as follows (Solidity version ):

```solidity
ITS _deployTokenManager(bytes32 tokenId, TokenManagerType tokenManagerType, bytes memory params)
    tokenManagerDeployer.delegatecall(abi.encodeWithSelector(ITokenManagerDeployer.deployTokenManager.selector, tokenId, tokenManagerType, params));
```

```solidity
TokenManagerDeployer.deployTokenManager(bytes32 tokenId, TokenManagerType tokenManagerType, bytes memory params)
    bytes memory args = abi.encode(address(this), implementationType, tokenId, params);  bytes memory
    bytecode = abi.encodePacked(type(TokenManagerProxy).creationCode, args);
    tokenManager = _create3(bytecode, tokenId);
```


```solidity
TokenManagerProxy.constructor(address interchainTokenService_, uint256 implementationType_, bytes32 tokenId, bytes memory params)
    tokenManager.delegatecall(abi.encodeWithSelector(IProxy.setup.selector, params))
```

```solidity

TokenManager.setup(bytes memory params)
    bytes memory operatorBytes = abi.decode(params_, (bytes));
    _addAccountRoles(operator, (1 << uint8(Roles.FLOW_LIMITER)) | (1 <<uint8(Roles.OPERATOR)));
_addAccountRoles(interchainTokenService, (1 << uint8(Roles.FLOW_LIMITER)) | (1 << uint8(Roles.OPERATOR)));
```

What we can derive from this:
1. we can set 1 operator (optional)
   1. retrieved from bytes params, if none set to address(0)
   2. we instantiate a new operator group where per TOKEN MANAGER
2. the ITS becomes a flow limiter
   1. we instantiate a new flow limiter group where the ITS is the only member PER TOKEN MANAGER
