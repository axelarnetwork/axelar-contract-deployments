## v1.1 GMP Upgrade

```bash
node stellar/deploy-contract.js upgrade AxelarGateway --version 1.1.1 --migration-data '[["axelar","0xe75bfad0ac5c972ac4053d11bade19d182c4799f22872c3cca8090e07
a229a5f-250959"],["axelar","0x81d4274316380ec968c5cd249963b03588447104e369be005cbd60050c437715-272885"],["axelar","0x7ed28ebf275f430b64487ac74f944b151edf3b8392456d789f04f83bf75c079e-257089"],["axelar","0xfecb1bbe5e1eafa7fefb519884e3d58a05066c7ca07054372cab5a8105b31518-250301"],["axelar","0x6538b066d98bbd9d4e5338f19146d773747022fc4e698376671e4a1d228e69e3-252142"],["stellar","0x6a7348f84b5c0a42892656228834fcf3880a827cedeafcf6cfc171d825250395-2"],["axelar","0x93f6b146c47fe45c325b804e559fbb9036eba114ebb7a53ab12796aa5d5ba50a-256802"],["axelar","0x49ad7fd5f17a11694f1d0abdc1c498eed6f7128159685e3c066b6d1e4a3928fe-253098"],["axelar","0x85f5f5df8c774da7a50902f295fc9c9643187ab1bab4ae0d76dcfc11bd36bbc4-257244"],["axelar","0xd43f92c82e733db3d381addb7d8cff2f5d721e4e4f976f7811df1835104373b0-256938"],["axelar","0x621b48ce71ad7841772436ce1e5ed96238e4e537bbf37e55fdcc19e5ee3f6b4f-256521"],["stellar","0x965bd66495ad46390b97b6c03e4e52abe77b213cbaedfbabbd9e96b74648f847-2"],["axelar","0xb0f33127bb7049f967df74df92094ce8f9c32a21b33b048ba4bc34306ba08063-251212"],["axelar","0xe9a205b406e320b3124cb2e171567105fab78ac980d7f5dcc41a407dd955a308-251084"]]'
```

```text
Wallet address: GC2SJ4YXCMP2LYXMXBNJMK6SNK4XUR7TGJXY4GA3VACNMCZVCQ6VFGG3

Wallet balances: 409.0945038 XLM

Wallet sequence: 240976600500273171

Proceed with upgrade on Stellar? (y/n) y

Uploaded AxelarGateway wasm tx: 2ebcf4296ff931b38e085def744a379decf026c4e995de5b439be7ada2124aa5

New Wasm hash: d68610690fa381aace03f16ef591334d61e808bcba0ac9e3a15d76df492aff24

Upgraded contract tx: 7bef9f6d863da0794df4d2979b48bd6d9d0a3adb2b841fb5bed28024f4644b31

Contract upgraded successfully: {
  "contractName": "AxelarGateway",
  "newWasmHash": "d68610690fa381aace03f16ef591334d61e808bcba0ac9e3a15d76df492aff24"
}
```

```bash
node stellar/deploy-contract.js upgrade AxelarOperators --version 1.1.1 --migration-data '["GBAGPWP4GXOB4PD62KLUGOWKWVBYSUQOO37XHB7PNYWKVHSDAVO4HWHD","GDK4ZR7W
NQMQ43WZTZDB3YRSWIOEQGPD4CZBKQOKYNIUHLQ6PZNPMOJK"]'
```

```text
Wallet address: GC2SJ4YXCMP2LYXMXBNJMK6SNK4XUR7TGJXY4GA3VACNMCZVCQ6VFGG3

Wallet balances: 383.0461123 XLM

Wallet sequence: 240976600500273174

Proceed with upgrade on Stellar? (y/n) y

Uploaded AxelarOperators wasm tx: d688e4e1e07836238d5b730b9735d7866ffb706cd5b53cffadd67118dd72daa6

New Wasm hash: 8e0d3c6ace7b80c80d945eaca495ff2cea7de12e9cf736dcf1fb9aaee07b4dd2

Upgraded contract tx: fdea67e9601a0b054f9f47a6ca827ec6629b6c124886762b101921b9d2434368

Contract upgraded successfully: {
  "contractName": "AxelarOperators",
  "newWasmHash": "8e0d3c6ace7b80c80d945eaca495ff2cea7de12e9cf736dcf1fb9aaee07b4dd2"
}
```

```bash
node stellar/deploy-contract.js upgrade AxelarGasService --version 1.1.1
```

```text
Wallet address: GC2SJ4YXCMP2LYXMXBNJMK6SNK4XUR7TGJXY4GA3VACNMCZVCQ6VFGG3

Wallet balances: 378.9354348 XLM

Wallet sequence: 240976600500273176

Proceed with upgrade on Stellar? (y/n) y

Uploaded AxelarGasService wasm tx: 939a075399181f79e08e224e4b732f47a3ae0210a67098abe807613bf3230d00

New Wasm hash: 5f85b5ca8888347990b7d6384a3c73dac1fc652f93086224d78dbadfc934d729

Upgraded contract tx: 6f03ad36b5b35a1d6f519a3fa9e3d3f74bfd9522e717ed1031a7738cf8b181fa

Contract upgraded successfully: {
  "contractName": "AxelarGasService",
  "newWasmHash": "5f85b5ca8888347990b7d6384a3c73dac1fc652f93086224d78dbadfc934d729"
}
```

```bash
node stellar/deploy-contract.js deploy AxelarExample --version 1.0.3
```

```text
Uploaded AxelarExample wasm tx: 00f391de2a2d3a02be09422c8f5297fee2e75e81dcaf104b6d96a714d26a9756

Initializing contract with args: {
  "gatewayAddress": "CD6VSKXB4HY2DWU7EP2PUIYTBJBJ36LDJXEZN4NSXFYF5YP37DDFX6NF",
  "gasServiceAddress": "CDZNIEA5FLJY2L4BWFW3P6WPFYWQNZTNP6ED2K5UHD5PNYTIMNFZDD3W",
  "itsAddress": "CBDBMIOFHGWUFRYH3D3STI2DHBOWGDDBCRKQEUB4RGQEBVG74SEED6C6"
}

Initialized contract tx: bedfc3a6d5e50e14886811317047e012634ccb9f021584f32045731945658ef2

Contract initialized at address: CCHEWZGXJSJL6Y4XONWGCWWQPWXEVPEE7GSF76PICHJSSQCJEHEL62F6

Contract deployed successfully: {
  "address": "CCHEWZGXJSJL6Y4XONWGCWWQPWXEVPEE7GSF76PICHJSSQCJEHEL62F6",
  "deployer": "GC2SJ4YXCMP2LYXMXBNJMK6SNK4XUR7TGJXY4GA3VACNMCZVCQ6VFGG3",
  "wasmHash": "cb96e568d52b5933111d3d97c7a3c23330df1db086aad6001f67e2daaa62d73b",
  "version": "1.0.3",
  "initializeArgs": {
    "gatewayAddress": "CD6VSKXB4HY2DWU7EP2PUIYTBJBJ36LDJXEZN4NSXFYF5YP37DDFX6NF",
    "gasServiceAddress": "CDZNIEA5FLJY2L4BWFW3P6WPFYWQNZTNP6ED2K5UHD5PNYTIMNFZDD3W",
    "itsAddress": "CBDBMIOFHGWUFRYH3D3STI2DHBOWGDDBCRKQEUB4RGQEBVG74SEED6C6"
  }
}
```

```bash
node stellar/deploy-contract.js deploy Multicall --version 1.0.1
```

```text
Uploaded Multicall wasm tx: e8369bb9e8a8ac43d5466611772e629d7df37c7884eab6b656feff836373173b

Initializing contract with args: {}

Initialized contract tx: 28e149bc734f355ebd8e323542918f8d1de1cc3884fab02629718ebb956d7dd3

Contract initialized at address: CC5LVKQA73ZVVUBAOCV5INV4TXPMELBFJ6XTQUBJTP4O2LSUKAA7VHLZ

Contract deployed successfully: {
  "address": "CC5LVKQA73ZVVUBAOCV5INV4TXPMELBFJ6XTQUBJTP4O2LSUKAA7VHLZ",
  "deployer": "GC2SJ4YXCMP2LYXMXBNJMK6SNK4XUR7TGJXY4GA3VACNMCZVCQ6VFGG3",
  "wasmHash": "0c491cc15edf95dbc131cbac07dc3035f05a9e6fd180d2733b9315685323df26",
  "version": "1.0.1",
  "initializeArgs": {}
}
```

## ITS Upgrade

```bash
node stellar/deploy-contract.js upload TokenManager --version 1.1.1
```

```text
Uploaded TokenManager wasm tx: 81276211df5b55f95b082053bd787970e738e4929e9f6d17011a56899cdf14e4

Contract uploaded successfully: {
  "contractName": "TokenManager",
  "wasmHash": "9883cdb6740d685007a88dbd5a1269ffc1550a81a182fed5de3748356d0e342c"
}
```

```bash
node stellar/deploy-contract.js upload InterchainToken --version 1.1.1
```

```text
Uploaded InterchainToken wasm tx: 2d50e74dd41b44d4f137ce2abe174d2d096f5a962cf42e4218272ebdf1758ad2

Contract uploaded successfully: {
  "contractName": "InterchainToken",
  "wasmHash": "71648e1b8ee1231ea03c096257ed3c6d55478effbfab77b8e587a5139ab84cbe"
}
```

```bash
node stellar/deploy-contract.js upgrade InterchainTokenService --version 1.1.2 --migration-data '{"newTokenManagerWasmHash":"9883cdb6740d685007a88dbd5a1269
ffc1550a81a182fed5de3748356d0e342c","newInterchainTokenWasmHash":"71648e1b8ee1231ea03c096257ed3c6d55478effbfab77b8e587a5139ab84cbe"}'
```

```text
Uploaded InterchainTokenService wasm tx: f82cb4164a3098c376996d38b4f33d002be832169917e0ce2cd0830d9444b073

New Wasm hash: ed1762aa118cc09a2c035213f92b8cc0fa32d74302ec9636d4f925a3eba49dc3

Upgraded contract tx: bda516a240c6633ad205d993d0eed56ebcc739deabc32881f3b5e70b9fb9c384

Contract upgraded successfully: {
  "contractName": "InterchainTokenService",
  "newWasmHash": "ed1762aa118cc09a2c035213f92b8cc0fa32d74302ec9636d4f925a3eba49dc3"
}
```

```bash
node stellar/its.js migrate-tokens "A/cM/Lqj2/Fx22VBAjjgW1jDrnywIZy3Bo1Z1Wr5b7Q=" "dGQ0vIU2L8yyO8EEthUPrvCy0v+WpQ8tElUwREprmQ8=" "XqRV/aAChDa0fS7nCVsHUDo9e
mkk9vDjOrzuIaPtKv0=" "zlmGy2Rk0M/FhXFVKbDXxzQxjq33qxrvWtExvamdenI=" "pGiAIMevFALTHyQpjk+P1+e9nlgSqIEjHi9nOq0UxcE=" "sF8KrUqC9SrWClGQyl3oDK2QfbV4LI6jqVpBZhCYRPA=" "+qa+qidI5P8g0RG6rRHsU27kxgjD5Lc+bD/XkGWShKo=" "xHlsRTIOfLZxlLAiX4ruZM+diCMXxWEGmNfBfespLw0=" "0ZLTZvcC/ljjXUjx2SZHEchrQYgiApj2qZ/XfNnACXM=" "JuOWPNpbr6ugOgKTMnzWAl+tyBbSth9pzsPD43NT0j4=" --version 1.1.1
```

```text
Migrating token: 0x03f70cfcbaa3dbf171db65410238e05b58c3ae7cb0219cb7068d59d56af96fb4

Upgrader address: CCIAE4D3SBAKHYGWN6FMY74ILYKNQXE7FQNYZESFUFAPS2JYZRNYED4F

Retrieved TokenManager address tx: afda741ce6684dc3eb75eb1aa54921fd25fbace272fecbd03d7ea203a272fbf6

TokenManager address: CBLLBQFUH2DQH2RVNFXTQGDTUJ4KFNIPK3R2IN5AYSSF2KEE2IODBTWB

Retrieved InterchainToken address tx: 9f93cf2d776b5fdfd3e3e4eab330a0c7be22c8f7d6ead40c80c5cfdf77747ee0

InterchainToken address: CB7BTETLXJHEBMMR4A736GR5KKSA3SRWMP7DYRPTQAEHYJWAHBYMB2EV

Migrated token tx: 8b6920bfc076563acaec972f4c45bf3e38c0c12776eae9c9b08005211a627e72

Migrating token: 0x746434bc85362fccb23bc104b6150faef0b2d2ff96a50f2d125530444a6b990f

Upgrader address: CCIAE4D3SBAKHYGWN6FMY74ILYKNQXE7FQNYZESFUFAPS2JYZRNYED4F

Retrieved TokenManager address tx: dac225d0cb17cb573d2d2013992fd96133691b93d6780d640d7485050edd37cf

TokenManager address: CDNPSZPQZJTCLN2CQPOS6HABME2KDT6WFVCIJXRIV6ZKOPJHO5LG3JTM

Retrieved InterchainToken address tx: e0ccf284271166ea47887da9403ec2392319e497cf4ca7a58b809397975d6277

InterchainToken address: CB6Y3ILCSW2XZJIJHZ3XSZBZVYNHGA4VXNKMLHDAHLVMNR2DNL43ESIG

Migrated token tx: cb8124258e9215b0518740d38b0cd76ae3f14830e27e20d148a8c033dffa6289

Migrating token: 0x5ea455fda0028436b47d2ee7095b07503a3d7a6924f6f0e33abcee21a3ed2afd

Upgrader address: CCIAE4D3SBAKHYGWN6FMY74ILYKNQXE7FQNYZESFUFAPS2JYZRNYED4F

Retrieved TokenManager address tx: aaf61067455b1902b8b12b443be64c2f41ab0c76712a264aaf18aef380c352e2

TokenManager address: CCBRTUZTUIUZ2DXIC2MIDCNCL2PSFAXJPIIDK6XCGIU3ZVDQ4C63SB6X

Retrieved InterchainToken address tx: 5a3034ab1544e9a02a8a65255259cd4c783817bc3e4d244552d44b89c4775773

InterchainToken address: CCHVF2CIBH6RP3FGXMFQ4ZGHYUGJDECRYCUZW6X7Q5A2ZBWUUZWTY4QT

Migrated token tx: f34410c9682a840625b64c692a0f0acb8125a3994faba0475358961348ec1f48

Sending transaction failed
            throw Error(`Transaction failed: ${getResponse.resultXdr}`);
```

```bash
node stellar/its.js migrate-tokens "zlmGy2Rk0M/FhXFVKbDXxzQxjq33qxrvWtExvamdenI=" "pGiAIMevFALTHyQpjk+P1+e9nlgSqIEjHi9nOq0UxcE=" "sF8KrUqC9SrWClGQyl3oDK2Qf
bV4LI6jqVpBZhCYRPA=" "+qa+qidI5P8g0RG6rRHsU27kxgjD5Lc+bD/XkGWShKo=" "xHlsRTIOfLZxlLAiX4ruZM+diCMXxWEGmNfBfespLw0=" "0ZLTZvcC/ljjXUjx2SZHEchrQYgiApj2qZ/XfNnACXM=" "JuOWPNpbr6ugOgKTMnzWAl+tyBbSth9pzsPD43NT0j4=" --version 1.1.1
```

```text
Migrating token: 0xce5986cb6464d0cfc585715529b0d7c734318eadf7ab1aef5ad131bda99d7a72

Upgrader address: CCIAE4D3SBAKHYGWN6FMY74ILYKNQXE7FQNYZESFUFAPS2JYZRNYED4F

Retrieved TokenManager address tx: cd68dfee686d2286b889a1cb18b15fc069ea5a57b0b56c5a9019d3d43854a7d6

TokenManager address: CDK33BKPUIE2SKIUVG4ANKUJVGR5CQLL7WVORIAT4R24KVXJUMGZZZDQ

Retrieved InterchainToken address tx: 68fec3bdf4747441df20874b5e76d2d1903a3fd4dae530de7cfb0fccd83d0654

InterchainToken address: CDMWNS76IQXK6J2JC2CK27DT6WQ2ZA4J5SMFLN6VPC3OIWFCK2Q473G6

Migrated token tx: 8410c5f873e27e867430d6421e7e1270015df57907c0c963bfd22d0002a280e6

Migrating token: 0xa4688020c7af1402d31f24298e4f8fd7e7bd9e5812a881231e2f673aad14c5c1

Upgrader address: CCIAE4D3SBAKHYGWN6FMY74ILYKNQXE7FQNYZESFUFAPS2JYZRNYED4F

Retrieved TokenManager address tx: 870ce7c8b4360ed4ed9093ae75e8a8aa487d386a6ee579af0c2cd806d5dd5f8d

TokenManager address: CD2X2676D5ARZ7LRGDO7MJTTGHRVZX5UQNN4FXMNSTZD4KOYX6CYLCSJ

Retrieved InterchainToken address tx: 45b34901b64413109cd7963a6a858953a9f05f9a0f4a806e9de89fc2051c37e2

InterchainToken address: CCN57TFTO3X7NZEAUIYQGDYC76LJBYPND57IYCVFTJFXKOS2IXHXQSDY

Migrated token tx: 51cf258179b19fdc6772f6bb4c31b3e6dc4b1e0840639d2bf53a4eaefe01aa66

Migrating token: 0xb05f0aad4a82f52ad60a5190ca5de80cad907db5782c8ea3a95a4166109844f0

Upgrader address: CCIAE4D3SBAKHYGWN6FMY74ILYKNQXE7FQNYZESFUFAPS2JYZRNYED4F

Retrieved TokenManager address tx: cc4bcf9eb77c44384e8f88cc912d4fd265585efe8ab2eabf66b6b8bd8c5fb703

TokenManager address: CAYFH4LTVI3JBIDGP5UBM3HU724IJIDTTBKSYJ5TEBOG7RU2B5QJ72BU

Retrieved InterchainToken address tx: b047bbc933139041ada347a3c858240ad13f0721a61fa2a9798f0b66bcabdf50

InterchainToken address: CB7PIYPBZLDZ5A2GL4OJQWQT22NCYSWEZAV2XOYHROUG2SJZCFE45VDJ

Migrated token tx: 2b964a9999e26097d12f81fc7ed1f204cc7e656a0229bd85dee4f618ea142890

Migrating token: 0xfaa6beaa2748e4ff20d111baad11ec536ee4c608c3e4b73e6c3fd790659284aa

Upgrader address: CCIAE4D3SBAKHYGWN6FMY74ILYKNQXE7FQNYZESFUFAPS2JYZRNYED4F

Retrieved TokenManager address tx: e73ab90ff0bc459e99664026a2a6ac5013eb9bd9e1d388f36ac007d176f36318

TokenManager address: CABC6SN2RGEES25ZVBVOPYTOOUKOKCNWLSRTKMCWLCRTJEXSJ6PRNBBN

Retrieved InterchainToken address tx: e283b1a429209392c3708e84eb110924f94dd927b16c782f815c6f0b1d37dc23

InterchainToken address: CBC3S3RVAPTNQNK2OVVTFRQ6T6JYDSOFPPFO7W3SOR4WWHHSJHDRWAGN

Migrated token tx: d40e4afdce5925044096681e25fa27cf0bbf393d524cf96466fe039a533a3a1a

Migrating token: 0xc4796c45320e7cb67194b0225f8aee64cf9d882317c5610698d7c17deb292f0d

Upgrader address: CCIAE4D3SBAKHYGWN6FMY74ILYKNQXE7FQNYZESFUFAPS2JYZRNYED4F

Retrieved TokenManager address tx: b48b34649644dd5632b72100459c2c36e9020cff5dc8d6767bff016b0b389630

TokenManager address: CABXIFFE5YMFV4TKZTUCHHUUKPMGPLG7D4PI5EYHNSDQ33NYGFLT2WKZ

Retrieved InterchainToken address tx: 2a566ef0becc56b12ef6561cd489b64d342a1882dd26b629f15f2738e45f143b

InterchainToken address: CCX3G6763NVWPRVKTCAE3XY3SPY32TCPPON72NTDONV23UQRRJE6M6T5

Migrated token tx: f59302710461d350811fc4a00874398dd206cf1ab34238998bded3975db043be

Migrating token: 0xd192d366f702fe58e35d48f1d9264711c86b4188220298f6a99fd77cd9c00973

Upgrader address: CCIAE4D3SBAKHYGWN6FMY74ILYKNQXE7FQNYZESFUFAPS2JYZRNYED4F

Retrieved TokenManager address tx: aebd4d638f3ba8c220c454806e61893e9d7310fedc172059892b702586266d51

TokenManager address: CBONAZIKLQYHPMDNPJBVW6ELCGG4EY5WQSF5ZPO6UMXIDTPUPCFVRPXF

Retrieved InterchainToken address tx: 7cb08acde59843ef5d367c684b7b092317dc1be6c83e2622c4dabc84944c2b53

InterchainToken address: CAQR2ADAMQ3MMRQ4WR2SNWVW3S2KGH4K762ZPUWSTGR5LVGLJRAHEKW6

Migrated token tx: 8990f84c5486d26d7e7df08bde6476116306e850545fbf908cabd06e3c4a9ed1

Migrating token: 0x26e3963cda5bafaba03a0293327cd6025fadc816d2b61f69cec3c3e37353d23e

Upgrader address: CCIAE4D3SBAKHYGWN6FMY74ILYKNQXE7FQNYZESFUFAPS2JYZRNYED4F

Retrieved TokenManager address tx: ed122a435d11387355f3da544e1bd5251845e3f7260ee23625b742ecbef64103

TokenManager address: CAS2JG4RTZOHQ67X46JAJ4W73JICNTGVFAGVCDPLUIVHC72CLN2LZI7T

Retrieved InterchainToken address tx: afdfe074f4ac1f4e5483081b92f9ff604533ac2aa9cdd26423c2ba9e39989822

InterchainToken address: CCYZG2QLIHGFDQASP2GUXSVFNK6GFDA4MOU5A2IXHZASGT6A5O5ZAPP2

Migrated token tx: 026f52a7c564a2ec223bb5d23cdcd1ff85472fa33b4c763092216fce672dcce1
```

```bash
node stellar/deploy-contract.js deploy Upgrader --version 1.1.1
```

```text
Uploaded Upgrader wasm tx: 5db67d88ccfb0d621dc6b33e64b9c7afb812f1cd3d2ccdd2ff159334b92efb69

Initializing contract with args: {}

Initialized contract tx: ae413a06c625ca747ce5ece547a5146e40c3fdabaf3234c94abcdedaabfc13b1

Contract initialized at address: CDXI3F2R6Q3W5AZRP3VQIOXM3YEIBFPEUAGZDOOSZ7DNQXHOKXU4FPVB

Contract deployed successfully: {
  "address": "CDXI3F2R6Q3W5AZRP3VQIOXM3YEIBFPEUAGZDOOSZ7DNQXHOKXU4FPVB",
  "deployer": "GC2SJ4YXCMP2LYXMXBNJMK6SNK4XUR7TGJXY4GA3VACNMCZVCQ6VFGG3",
  "wasmHash": "8393a1d52cc40fc3fd37d93da56a3322109159d794ab1d0fbee120dcb3d8cbcc",
  "version": "1.1.1",
  "initializeArgs": {}
}
```

