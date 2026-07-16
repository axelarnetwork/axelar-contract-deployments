# Amplifier CosmWasm Admin Consolidation

|                | **Owner** |
| -------------- | --------- |
| **Created By** | rista404  |
| **Deployment** | rista404  |

| **Network** | **Status**            | **Date**   |
| ----------- | --------------------- | ---------- |
| **Mainnet** | Snapshot (planning)   | 2026-07-09 |
| **Testnet** | Snapshot (planning)   | 2026-07-09 |

## Background

This document is the basis for consolidating the privileged roles on every Axelar
Amplifier CosmWasm contract deployed on `mainnet` and `testnet`. It records the
current holders of those roles so they can be migrated to a single controller — the
intended target being Axelar governance, `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`.

The Owner (upgrade authority) is moved in two steps: first rotated to the operations
multisig `axelar14vps3ev03zyp2wmj89etx8rrxdxyltfy4rzl5m`, then handed from the multisig
to governance.

Two roles are in scope for the consolidation, recorded per contract below:

1. **Owner (upgrade authority)** — the wasmd contract-level `admin`, i.e. the address
   that can migrate (upgrade) the code. `— (immutable)` means no migrate admin is set
   and the code can no longer be upgraded.
2. **Internal Admin** — the `permission_control` **Admin** role stored in the
   contract's own state under the raw key `permission_control_contract_admin_addr`.
   `—` means the contract does not set this role.

**Governance** — the `permission_control` **Governance** role
(`permission_control_governance_addr`) — is shown for context but is not itself being
transferred: wherever it is set it is already the governance module `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` on **both**
networks (verified for all contracts), so it is not repeated per row.

### Method

- Owner: `axelard query wasm contract <addr>` → `.contract_info.admin`.
- Internal Admin / Governance: `axelard query wasm contract-state raw <addr> <hex-key>`
  for the `permission_control_*` storage `Item`s.
- RPCs (from `axelar-chains-config/info`): mainnet `https://axelar-rpc.qubelabs.io:443`
  (`axelar-dojo-1`), testnet `https://axelar-testnet-rpc.qubelabs.io:443`
  (`axelar-testnet-lisbon-3`).
- Addresses enumerated from `axelar-chains-config/info/{mainnet,testnet}.json`
  (`.axelar.contracts`).

**(gov)** marks a cell that is already the governance module (no transfer needed).

---

## Mainnet (`axelar-dojo-1`, height ~31,294,942)

### Singleton contracts

| Contract | Address | Owner (upgrade authority) | Internal Admin |
|---|---|---|---|
| `Coordinator` | `axelar1rwy79m8u76q2pm3lrxednlgtqjd8439l7hmctdxvjsv2shsu9meq8ntlvx` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | — |
| `Router` | `axelar1d9atnamjjhtc46zmzyc202llqs0rhtxnphs6mkjurekath3mkgtq7hsk93` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | `axelar1kctshqfxw74usjme9mqlvkf0rgdh96ty0mmm6p` |
| `Multisig` | `axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | `axelar1kctshqfxw74usjme9mqlvkf0rgdh96ty0mmm6p` |
| `ServiceRegistry` | `axelar1rpj2jjrv3vpugx9ake9kgk3s2kgwt0y60wtkmcgfml5m3et0mrls6nct9m` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | — |
| `Rewards` | `axelar1harq5xe68lzl2kx4e5ch4k8840cgqnry567g0fgw7vt2atcuugrqfa7j5z` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | — |
| `AxelarnetGateway` | `axelar18vsne7lns36uvm8gv2cv5jl2lghts0xm7dvzpqzn70dl56gk9hvsgu9sqg` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | — |
| `NexusGateway` | `axelar14x5fqsezmzame904gkydguycsuqy5f8lp5knkzanccy8g8nz3yus65wveg` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | — |
| `InterchainTokenService` | `axelar1aqcj54lzz0rk22gvqgcn8fr5tx4rzwdv5wv5j9dmnacgefvd7wzsy2j2mr` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | `axelar1kctshqfxw74usjme9mqlvkf0rgdh96ty0mmm6p` |
| `ItsAbiTranslator` | `axelar10u3epx046nep88escmgchjyhah9d50jjewa35m2vuw2s2j5tcn7qtw4zmh` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` **(gov)** | — |
| `ItsSolanaTranslator` | `axelar1l4w6pgl4h2gguu82ue9wgfzsj8xgqfugtvrhv0s6zzr0hjwc654quydjqx` | — *(immutable)* | — |
| `ChainCodecEvm` | `axelar1nxq3kk04glzaqx3dq79ry5eygdtxwdc0520fs5ycgtnnsw4c2s4qy8n723` | — *(immutable)* | — |
| `ChainCodecSolana` | `axelar1d2ecnmd77stp50wyqp5l827hjtld9utvweaw0zwyk57fysv3ez9stnxrk4` | — *(immutable)* | — |
| `ChainCodecStellar` | `axelar1f4zjxl4qngmujmvvgz9xwqh9mk2u8kna643sh74556nn49wwmy9qrgnwfw` | — *(immutable)* | — |
| `ChainCodecSui` | `axelar1t4u9xxerducdc9gwxtcgsy5vrg962jv4qtlwgf7lnyn3pzld2vfq5gfjvu` | — *(immutable)* | — |

### `Gateway` (per verifier chain)

| Chain | Address | Owner (upgrade authority) | Internal Admin |
|---|---|---|---|
| `berachain` | `axelar1yedkcrlaf479vcu3xk47emzryc89kwm2m06zmecrd7ruqu4wguuqk0z0vg` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | — |
| `flow` | `axelar1w8frw33jn0yx59845wdgk0yru6fxvgr6hlh4xfdtdf08y5jamcnsyu0z6u` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | — |
| `hedera` | `axelar16x686a2hqr7drs5y4kyzwn8aghf7ld4enfhqg295d5xdllgyhd9sqafr4z` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | — |
| `hyperliquid` | `axelar1qezshrvqspy84w3ermmrny5w6wp347c9nzu3ktq37mwv3hmn42pqdtgzty` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | — |
| `monad` | `axelar1pwjjvn9cqtk8f5furw26szn7pl2x547cyc8qrsjk542c9gpyfs0spuetwl` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | — |
| `plume` | `axelar163ee8p7xne8een7v3hwz6zxh339v6s8js3v5c9520mlnzd7nxneq5eq86v` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | — |
| `solana` | `axelar1f258jksatcwdkcp8awnvx7p78m7ycszcnpd3ufq4yps4f77dah6q7xskv6` | `axelar1krxdv5avagv6nd82fdvfs4w38t6meydrp8yu8m` | — |
| `stellar` | `axelar1t46envzujl4kmhst6myg364v00f8jcyjespw6tr00r3ycd0pehxqetqrpm` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | — |
| `sui` | `axelar1svl69e32m240xgjluezrvpudjn92usrn3dqzfm2tzn3zqkn76d6qfse593` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` **(gov)** | — |
| `xrpl-evm` | `axelar1vvdukrmxdylvnn8e59s3gnn49lutv3n9tg4vnsttn33a8zulgfssl62q69` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | — |

### `VotingVerifier` (per verifier chain)

| Chain | Address | Owner (upgrade authority) | Internal Admin |
|---|---|---|---|
| `berachain` | `axelar1xx6xdw6mwmfl6u2jygq0zfx2q6uyc8emtt29j9wg78l2l4p739nqmwsgal` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | — |
| `flow` | `axelar1kkqdsqvwq9a7p9fj0w89wpx2m2t0vrxl782aslhq0kdw2xxd2aesv3un04` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | — |
| `hedera` | `axelar1q8q8qq59yv9wmhcreu5ykt4azsk83ttve4e7jyavt32k6jq862xsqexnfh` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | — |
| `hyperliquid` | `axelar1n64vk7l3zagh2eadkuhl7602lxluu86dn9smfxyp7c2e4v8pqj5sv4ypjr` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | — |
| `monad` | `axelar1u080xgqqu9zhl4e9hf2ktdny4pq6kc2pmh6u6mlv8nw5zjvcetvqqjzeu0` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | — |
| `plume` | `axelar1nrdqke6tcxjuymg5gyd9x3yg35n3wrgarnj3sqskp98z2xnvlx9q82f63t` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | — |
| `solana` | `axelar19gut3kvqf57gnu5ylq474qfgk4gg5ly89cs5kk4mde688lc5adsq6qyz4h` | `axelar1krxdv5avagv6nd82fdvfs4w38t6meydrp8yu8m` | — |
| `stellar` | `axelar1dalnx2yvmu3g3aau8m7fj426fk9u8dnzlr5azvqmr4x82rtclats8lhjmu` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | — |
| `sui` | `axelar1sykyha8kzf35kc5hplqk76kdufntjn6w45ntwlevwxp74dqr3rvsq7fazh` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` **(gov)** | — |
| `xrpl-evm` | `axelar1q8kn9t39ddpce42atk0d6wpdudr6djqxmz689m3nxy92ck0nnftqxfsuyk` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | — |

### `MultisigProver` (per verifier chain)

| Chain | Address | Owner (upgrade authority) | Internal Admin |
|---|---|---|---|
| `berachain` | `axelar1k483q898t5w0acqzxhdjlsmnpgcxxa49ye8m46757n8mtk70ugtsu927xw` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | `axelar1w2ey0ek9e8q2dfmeznz6ah49zdywpdme0z0kly` |
| `flow` | `axelar1rsuejfntt4rs2y8dn4dd3acszs00zyg9wpnsc6fmhevcp6plu5qspzn7e0` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | `axelar1w2ey0ek9e8q2dfmeznz6ah49zdywpdme0z0kly` |
| `hedera` | `axelar1e7z2faehrvpwl3apq3srr8djp386urvm2fgw3yafmju6slphhe8skecrwk` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | `axelar1w2ey0ek9e8q2dfmeznz6ah49zdywpdme0z0kly` |
| `hyperliquid` | `axelar1fxd8rq5j6wluyc07vl9vqr4xmdxxm25l2gd6m2an20mn5fdnzy6qll2nxx` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | `axelar1w2ey0ek9e8q2dfmeznz6ah49zdywpdme0z0kly` |
| `monad` | `axelar1dt6apz0m2lkuls3ah2h7zw277r0v50668fxytqtdxv83yzs4n69qlutnpk` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | `axelar1w2ey0ek9e8q2dfmeznz6ah49zdywpdme0z0kly` |
| `plume` | `axelar1ll4yhqtldlgqwqthyffqln3cyr2f8ydzhv0djpjyp6sk4v5k4kqqrs60s7` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | `axelar1w2ey0ek9e8q2dfmeznz6ah49zdywpdme0z0kly` |
| `solana` | `axelar1xdtjwhenmy80wckuntd2npd3zeqayy0q0l5dfy48g6wmu3p48pgqknnc9g` | `axelar1krxdv5avagv6nd82fdvfs4w38t6meydrp8yu8m` | `axelar1w2ey0ek9e8q2dfmeznz6ah49zdywpdme0z0kly` |
| `stellar` | `axelar1wdgp5xyqjyv5zsq86n6pah2lsmd46mn0gt4055mvvk6mezn9skqs6p93dg` | `axelar1t46envzujl4kmhst6myg364v00f8jcyjespw6tr00r3ycd0pehxqetqrpm` | `axelar1w2ey0ek9e8q2dfmeznz6ah49zdywpdme0z0kly` |
| `sui` | `axelar1v8jrupu2rqpskwgtr69max0ajul92q8z5mdxd505m2hu3xc5jzcqm8zyc6` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` **(gov)** | `axelar1w2ey0ek9e8q2dfmeznz6ah49zdywpdme0z0kly` |
| `xrpl-evm` | `axelar198xehj5htckk75s8wcamxerxtdc45669zdqjmr69guveqntj9f6s5rqq55` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | `axelar1w2ey0ek9e8q2dfmeznz6ah49zdywpdme0z0kly` |

### XRPL contracts

| Contract | Address | Owner (upgrade authority) | Internal Admin |
|---|---|---|---|
| `XrplGateway` | `axelar15dsw0qqnvumnsukjtwt040wnelwrglgslqcqsa7d62f2neuv7slq7rq7zd` | `axelar14vps3ev03zyp2wmj89etx8rrxdxyltfy4rzl5m` | `axelar13m9ghqwyk50mqtm9cj884v42kw8kmjjtl82q78` |
| `XrplVotingVerifier` | `axelar14rd4uyrqyl0tw75gjn8zqfppmy08t3x3wrsujeqp37l0hghduanscfvkz6` | `axelar14vps3ev03zyp2wmj89etx8rrxdxyltfy4rzl5m` | `axelar1pczf792wf3p3xssk4dmwfxrh6hcqnrjp70danj` |
| `XrplMultisigProver` | `axelar15mhhuf887t6nfx2t0vuc6kx9w2uk65h939awmz6n7r6ggzyf659st25hff` | `axelar14vps3ev03zyp2wmj89etx8rrxdxyltfy4rzl5m` | `axelar13m9ghqwyk50mqtm9cj884v42kw8kmjjtl82q78` |

---

## Testnet (`axelar-testnet-lisbon-3`, height ~33,212,236)

### Singleton contracts

| Contract | Address | Owner (upgrade authority) | Internal Admin |
|---|---|---|---|
| `Coordinator` | `axelar1rwy79m8u76q2pm3lrxednlgtqjd8439l7hmctdxvjsv2shsu9meq8ntlvx` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `Router` | `axelar1d9atnamjjhtc46zmzyc202llqs0rhtxnphs6mkjurekath3mkgtq7hsk93` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` |
| `Multisig` | `axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` |
| `ServiceRegistry` | `axelar1rpj2jjrv3vpugx9ake9kgk3s2kgwt0y60wtkmcgfml5m3et0mrls6nct9m` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `Rewards` | `axelar1harq5xe68lzl2kx4e5ch4k8840cgqnry567g0fgw7vt2atcuugrqfa7j5z` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `AxelarnetGateway` | `axelar1kq687tszm67hr5ws5pqhtchc8uatxur8r4rm4xgclyghetthtlzs9pnqfl` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `InterchainTokenService` | `axelar1aqcj54lzz0rk22gvqgcn8fr5tx4rzwdv5wv5j9dmnacgefvd7wzsy2j2mr` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` |
| `ItsAbiTranslator` | `axelar1mla7j6dwvenszzajk64ee7hwlccl09p356mdc2akjwfus2fxdazs0wdmq8` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` **(gov)** | — |
| `ItsSolanaTranslator` | `axelar16gznt8y3rpt3vg2geng4s25c2evrd3syafsven5ha05uqm9xkyhq8a0jwy` | — *(immutable)* | — |
| `ChainCodecEvm` | `axelar1p6meh9pxkrf3qc8uu0kllcg7wl9dfukcrjklrf4e3w4jd947agsqxjpudh` | — *(immutable)* | — |
| `ChainCodecSolana` | `axelar1d2ecnmd77stp50wyqp5l827hjtld9utvweaw0zwyk57fysv3ez9stnxrk4` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` **(gov)** | — |
| `ChainCodecStellar` | `axelar12dxa5j4s0fcnxeqvnjrfl824y4dy77fz63vayqrpdw48r603cmwscyn9nu` | — *(immutable)* | — |
| `ChainCodecSui` | `axelar1vzarak8x6k9eyxgmq2hw8xjv80wwkdvchw980kte5w5ayweu7k4s9ddyaf` | — *(immutable)* | — |

### `Gateway` (per verifier chain)

| Chain | Address | Owner (upgrade authority) | Internal Admin |
|---|---|---|---|
| `arc-8` | `axelar15g2gf3n8exglrc2vmv8jm64ydetxtc754fhq7942qxgdww8wzyfqpk5628` | `axelar1wxej3l9aczsns3harrtdzk7rct29jl47tvu8mp` | — |
| `berachain` | `axelar1yedkcrlaf479vcu3xk47emzryc89kwm2m06zmecrd7ruqu4wguuqk0z0vg` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `celo-sepolia` | `axelar14yx3v0dfhkpkffk9q2yqhppena9w22ue76ylv6eneefmjpc6q05s5xy6td` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `flow` | `axelar1w8frw33jn0yx59845wdgk0yru6fxvgr6hlh4xfdtdf08y5jamcnsyu0z6u` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `hedera` | `axelar1xgr3j2wuc9ut7yjz4nr677scmzd4z6lh3srnpmdh0nyqzcfka3rqe2nsmq` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `hyperliquid` | `axelar1qezshrvqspy84w3ermmrny5w6wp347c9nzu3ktq37mwv3hmn42pqdtgzty` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `memento-demo` | `axelar126qyawsymtngnt4squm9l5j6myjy84ckyw93mgllughzckp22tcsc37cep` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `monad` | `axelar1pwjjvn9cqtk8f5furw26szn7pl2x547cyc8qrsjk542c9gpyfs0spuetwl` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `monad-3` | `axelar1nfe5lknq2z7f80dp3j60jjxa0k084hcgfl7hv9smx0ws5h7s4d0s2dkppr` | `axelar1wxej3l9aczsns3harrtdzk7rct29jl47tvu8mp` | — |
| `plume` | `axelar163ee8p7xne8een7v3hwz6zxh339v6s8js3v5c9520mlnzd7nxneq5eq86v` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `solana` | `axelar1f258jksatcwdkcp8awnvx7p78m7ycszcnpd3ufq4yps4f77dah6q7xskv6` | `axelar1wxej3l9aczsns3harrtdzk7rct29jl47tvu8mp` | — |
| `stellar-2026-q1-2` | `axelar19wr3vf0rk0jl0h8s2tcramn0wmnmv8gx7kp7j5rd5jjdypsjxkhqgufvzn` | `axelar1wxej3l9aczsns3harrtdzk7rct29jl47tvu8mp` | — |
| `sui` | `axelar1svl69e32m240xgjluezrvpudjn92usrn3dqzfm2tzn3zqkn76d6qfse593` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` **(gov)** | — |
| `test-avalanche` | `axelar16mek8sdcsq78jltfue35zhm5ds0cxpl0dfnrel8kck3jwtecdtnqcejdav` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `test-sepolia` | `axelar17hcsrrluv5zsrrt725wmscs3ma98hrg5f0t0ukmjxuyt2k7cwr3saxuwjc` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `xrpl-evm` | `axelar1vvdukrmxdylvnn8e59s3gnn49lutv3n9tg4vnsttn33a8zulgfssl62q69` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |

### `VotingVerifier` (per verifier chain)

| Chain | Address | Owner (upgrade authority) | Internal Admin |
|---|---|---|---|
| `arc-8` | `axelar1qxyyfs4jynsmtu2ajjh4lmz2a9drzr6e7gzf2wcz4nc8qt96mlyqk9f6s4` | `axelar1wxej3l9aczsns3harrtdzk7rct29jl47tvu8mp` | — |
| `berachain` | `axelar1xx6xdw6mwmfl6u2jygq0zfx2q6uyc8emtt29j9wg78l2l4p739nqmwsgal` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `celo-sepolia` | `axelar1ccyfamfvzvheec5c4knvq0l5g42knemfrnaq6t6znuwskt963k5smr9aam` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `flow` | `axelar1kkqdsqvwq9a7p9fj0w89wpx2m2t0vrxl782aslhq0kdw2xxd2aesv3un04` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `hedera` | `axelar1ce9rcvw8htpwukc048z9kqmyk5zz52d5a7zqn9xlq2pg0mxul9mqxlx2cq` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `hyperliquid` | `axelar1n64vk7l3zagh2eadkuhl7602lxluu86dn9smfxyp7c2e4v8pqj5sv4ypjr` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `memento-demo` | `axelar1uekdelqqxxuq5e6jxttlaxrhq3aq2ksn45h9lvtljc6hayeqe95qss5s6v` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `monad-3` | `axelar12ruanxwnzfymj52gr93lg70g2pe3yxvzy7g0yrhj66vtdrg7snsqhzwq0r` | `axelar1wxej3l9aczsns3harrtdzk7rct29jl47tvu8mp` | — |
| `plume` | `axelar1nrdqke6tcxjuymg5gyd9x3yg35n3wrgarnj3sqskp98z2xnvlx9q82f63t` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `solana` | `axelar19gut3kvqf57gnu5ylq474qfgk4gg5ly89cs5kk4mde688lc5adsq6qyz4h` | `axelar1wxej3l9aczsns3harrtdzk7rct29jl47tvu8mp` | — |
| `stellar-2025-q3` | `axelar18y8p7dxesmxttvdzp5sqjksqrnh9xg32gtfqnkkucvv9de38f69qfn6ph3` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `stellar-2026-q1-2` | `axelar1wmwnuj0609q5sd6xfl9xvptcprsas6vr95z2kxjsnanc7suchntqgk3flu` | `axelar1wxej3l9aczsns3harrtdzk7rct29jl47tvu8mp` | — |
| `sui` | `axelar1sykyha8kzf35kc5hplqk76kdufntjn6w45ntwlevwxp74dqr3rvsq7fazh` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` **(gov)** | — |
| `test-avalanche` | `axelar1hupk5du59cgu4ps5s637rhakwsq0060ycdp57j2ccevna7wqqzrqnfrr0p` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `test-sepolia` | `axelar1r4rmvn83vrfj4evy5l8cv2nat2v0whm36ds3crn2mhlq8ufmhvts9467zz` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |
| `xrpl-evm` | `axelar1q8kn9t39ddpce42atk0d6wpdudr6djqxmz689m3nxy92ck0nnftqxfsuyk` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | — |

### `MultisigProver` (per verifier chain)

| Chain | Address | Owner (upgrade authority) | Internal Admin |
|---|---|---|---|
| `arc-8` | `axelar1nsjl6hldse4agnjf9pm29hjetyvaxgwm2638z3qd27d32hsqygyqtsluqj` | `axelar1wxej3l9aczsns3harrtdzk7rct29jl47tvu8mp` | `axelar1w7y7v26rtnrj4vrx6q3qq4hfsmc68hhsxnadlf` |
| `berachain` | `axelar1k483q898t5w0acqzxhdjlsmnpgcxxa49ye8m46757n8mtk70ugtsu927xw` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` |
| `celo-sepolia` | `axelar1huplwem2amlects7n06llvl46m5tfm33vtty6p82d7lmka5kmmhsrdywf4` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` |
| `flow` | `axelar1rsuejfntt4rs2y8dn4dd3acszs00zyg9wpnsc6fmhevcp6plu5qspzn7e0` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` |
| `hedera` | `axelar1kleasry5ed73a8u4q6tdeu80hquy4nplfnrntx3n6agm2tcx40fssjk7gj` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` |
| `hyperliquid` | `axelar1fxd8rq5j6wluyc07vl9vqr4xmdxxm25l2gd6m2an20mn5fdnzy6qll2nxx` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` |
| `memento-demo` | `axelar13s3xyvcjpetwdfyc9q2hh9nc3hdvf7cvtkh33qk0g8adjjjgrk6qeacv67` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` |
| `monad-3` | `axelar162vcvdwa4ga6aq90juclh8ssaus5dq4kw6jz89wugr0gkczr6wussc47th` | `axelar1wxej3l9aczsns3harrtdzk7rct29jl47tvu8mp` | `axelar1w7y7v26rtnrj4vrx6q3qq4hfsmc68hhsxnadlf` |
| `plume` | `axelar1ll4yhqtldlgqwqthyffqln3cyr2f8ydzhv0djpjyp6sk4v5k4kqqrs60s7` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` |
| `solana` | `axelar1xdtjwhenmy80wckuntd2npd3zeqayy0q0l5dfy48g6wmu3p48pgqknnc9g` | `axelar1wxej3l9aczsns3harrtdzk7rct29jl47tvu8mp` | `axelar1w7y7v26rtnrj4vrx6q3qq4hfsmc68hhsxnadlf` |
| `stellar-2025-q3` | `axelar1aux2l6er84m6gtayqdwqhz4rl0txqdlj3v7szr72j7etve3jmpks7x4euy` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` |
| `stellar-2026-q1-2` | `axelar14xy6t4gnl26jxe3z2lqk24nnu3q0f7khlrpns9d00q9dvaqjzqmqp93ahp` | `axelar1wxej3l9aczsns3harrtdzk7rct29jl47tvu8mp` | `axelar1w7y7v26rtnrj4vrx6q3qq4hfsmc68hhsxnadlf` |
| `sui` | `axelar1v8jrupu2rqpskwgtr69max0ajul92q8z5mdxd505m2hu3xc5jzcqm8zyc6` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` **(gov)** | `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` |
| `test-avalanche` | `axelar1l5k8wlzmkmtnvjvs9x77wdcfweucwgums9e8fh0d8cy76ymqc5aqzguqnn` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` |
| `test-sepolia` | `axelar1u7qt6kz34ljjx6c94444e2v57uzt2tdlgxc6qjkvu4c5phncca8qakejex` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` |
| `xrpl-evm` | `axelar198xehj5htckk75s8wcamxerxtdc45669zdqjmr69guveqntj9f6s5rqq55` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` |

### XRPL contracts

| Contract | Address | Owner (upgrade authority) | Internal Admin |
|---|---|---|---|
| `XrplGateway` | `axelar18qltw4382s5qz0rgzfxz67mr83smk580hewlkfd33l5tmcdp8unqw35glh` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar1h2la8ghy8skc8w3dd0k28nkp57lql69fq9ydvl` |
| `XrplVotingVerifier` | `axelar1pnynr6wnmchutkv6490mdqqxkz54fnrtmq8krqhvglhsqhmu7wzsnc86sy` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar1dtfpfmvpq03l8qcxvrweahcakzgh52xavpumqv` |
| `XrplMultisigProver` | `axelar1k82qfzu3l6rvc7twlp9lpwsnav507czl6xyrk0xv287t4439ymvsl6n470` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar1h2la8ghy8skc8w3dd0k28nkp57lql69fq9ydvl` |

---

## Address legend

### Mainnet

| Address | Type | Roles held | Count |
|---|---|---|---|
| `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | BaseAccount | owner×31 | 31 |
| `axelar1w2ey0ek9e8q2dfmeznz6ah49zdywpdme0z0kly` | BaseAccount | admin×10 | 10 |
| `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | ModuleAccount (governance) | owner×4 | 4 |
| `axelar1krxdv5avagv6nd82fdvfs4w38t6meydrp8yu8m` | BaseAccount | owner×3 | 3 |
| `axelar1kctshqfxw74usjme9mqlvkf0rgdh96ty0mmm6p` | BaseAccount | admin×3 | 3 |
| `axelar14vps3ev03zyp2wmj89etx8rrxdxyltfy4rzl5m` | BaseAccount | owner×3 | 3 |
| `axelar13m9ghqwyk50mqtm9cj884v42kw8kmjjtl82q78` | BaseAccount | admin×2 | 2 |
| `axelar1t46envzujl4kmhst6myg364v00f8jcyjespw6tr00r3ycd0pehxqetqrpm` | wasm contract (Stellar `Gateway`) | owner×1 | 1 |
| `axelar1pczf792wf3p3xssk4dmwfxrh6hcqnrjp70danj` | BaseAccount | admin×1 | 1 |

- `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` — Axelar governance module account. Internal **Governance** role on every contract that sets one, and upgrade owner of a handful of contracts. Likely transfer target for consolidation.
- `axelar1w2ey0ek9e8q2dfmeznz6ah49zdywpdme0z0kly` — Current internal **admin** of every mainnet `MultisigProver` (set via the [MultisigProver admin transfer](./2026-06-MultisigProver-Admin-Transfer.md)).
- `axelar1t46envzujl4kmhst6myg364v00f8jcyjespw6tr00r3ycd0pehxqetqrpm` — The Stellar `Gateway` **contract** (not an account).
- `axelar1pczf792wf3p3xssk4dmwfxrh6hcqnrjp70danj` — Former `MultisigProver` admin (replaced in the [MultisigProver admin transfer](./2026-06-MultisigProver-Admin-Transfer.md)). Still the mainnet **XrplVotingVerifier** admin.

### Testnet

| Address | Type | Roles held | Count |
|---|---|---|---|
| `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | BaseAccount | owner×43, admin×3 | 46 |
| `axelar1wxej3l9aczsns3harrtdzk7rct29jl47tvu8mp` | BaseAccount | owner×12 | 12 |
| `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` | BaseAccount | admin×12 | 12 |
| `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | ModuleAccount (governance) | owner×5 | 5 |
| `axelar1w7y7v26rtnrj4vrx6q3qq4hfsmc68hhsxnadlf` | BaseAccount | admin×4 | 4 |
| `axelar1h2la8ghy8skc8w3dd0k28nkp57lql69fq9ydvl` | BaseAccount | admin×2 | 2 |
| `axelar1dtfpfmvpq03l8qcxvrweahcakzgh52xavpumqv` | BaseAccount | admin×1 | 1 |

- `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` — Axelar governance module account. Internal **Governance** role on every contract that sets one, and upgrade owner of a handful of contracts. Likely transfer target for consolidation.

---

## Observations

- **Mainnet MultisigProver admins are already rotated.** All mainnet `MultisigProver`
  instances have internal admin `axelar1w2ey0ek9e8q2dfmeznz6ah49zdywpdme0z0kly`
  (set via the [MultisigProver admin transfer](./2026-06-MultisigProver-Admin-Transfer.md)).
- **Immutable contracts:** the `ChainCodec*` contracts and `ItsSolanaTranslator` have
  no wasmd upgrade admin (`— (immutable)`), so their code cannot be migrated.
