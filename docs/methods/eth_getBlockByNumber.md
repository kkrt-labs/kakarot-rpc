# eth_getBlockByNumber

## Metadata

- name: eth_getBlockByNumber
- prefix: eth
- state: ⚠️
- [specification](https://github.com/ethereum/execution-apis/blob/main/src/eth/block.yaml#L17)
- [issue](https://github.com/sayajin-labs/kakarot-rpc/issues/16)

## Specification Description

Returns information about a block by number.

### Parameters

- [BlockNumberOrTag](https://github.com/ethereum/execution-apis/blob/main/src/schemas/block.yaml#L102)
- boolean - Hydrated transactions (required)

### Returns

- [Block](https://github.com/ethereum/execution-apis/blob/main/src/schemas/block.yaml#L1)

## Kakarot Logic

This method does not interact with the Kakarot contract or any other Starknet
contract. It calls a Starknet JSON-RPC client and fetches information about a
block by block number.

### Kakarot methods

### Starknet methods

- [starknet_getBlockWithTxHashes](https://github.com/starkware-libs/starknet-specs/blob/master/api/starknet_api_openrpc.json#L11)
  if Hydrated transactions == false
- [starknet_getBlockWithTxs](https://github.com/starkware-libs/starknet-specs/blob/master/api/starknet_api_openrpc.json#L44)
  if Hydrated transactions == true

### Example

Example call:

```json
{
  "jsonrpc": "2.0",
  "method": "eth_getBlockByNumber",
  "params": ["latest", false],
  "id": 0
}
```

Example responses:

- Hydrated transactions == false

```json
{
  "jsonrpc": "2.0",
  "result": {
    "baseFeePerGas": "0x33cddcdc3",
    "difficulty": "0x0",
    "extraData": "0x506f776572656420627920626c6f58726f757465",
    "gasLimit": "0x1c9c380",
    "gasUsed": "0x11ebff6",
    "hash": "0xe8602d8054019b9e47596ec1862022c09d129dc8f9ad84b208f0bc07ddf7dee7",
    "logsBloom": "0xf9224dd5cdd6d819fa1fb05ee1254a632a5044c9890bc85665c9a2ca8f1e7bc7f01e5b01d49a2a1db4705dc34f5c05a7272faa1e49c6bfe51b68567335386400cc428c16518a49ace99b5b2fe172aafd9fb68c20cfe01b6c06129ec1ebf5b8acf220c35416b38712fdfeadab42f06aed03afb250dc7c96bf737951f27d5bd11e6302a35a3f4e53159af93da18b7079e55944a5cbcd2315fdd277f0527d9070a4aebac57f958ceefbf231f4e58e5a64289f716ecd2b33eef2c2ff23eaeeadf43e733de0da03898d7a0a41198874273773c3cf13e07f9d809ed84529afba31aebfe3bfb8993d708c24c90f30da3e995d809051737be62ec5ce8d0f833243999ad5",
    "miner": "0x51a1449b3b6d635eddec781cd47a99221712de97",
    "mixHash": "0xcd63bd0e77f80e9d9faef8401339d918bc99e86a523042c9f189657173a736fe",
    "nonce": "0x0000000000000000",
    "number": "0xfa6cfe",
    "parentHash": "0xa64175c62be5903e84420c5339d25c64b44bd3d882ff3ee5e01ba415159228b6",
    "receiptsRoot": "0x546df020dea7f034c3bfc942bbad8ace76c1402198f28febc6a027494898c2e8",
    "sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
    "size": "0x16c9a",
    "stateRoot": "0xc973c6d4e85a820c19e6478d6767b26445e233f69f3e92264c92a17da1c554d6",
    "timestamp": "0x63c3e2d3",
    "totalDifficulty": "0xc70d815d562d3cfa955",
    "transactions": [
      "0x56a8fdb331c294f002dce8ad6f35ab5d47c644e8cc09798fd812c257600a03b9",
      "0x90ceed6fd8b186ad644413eb8b4266c296e7958d73e4c540e18fd03a3c24b236",
      "0xf11f8f23b28347c0f88dbbda923c423f78bbf48742cb7e4d3d9f3b956156a18d",
      "0x772049ae99ef19882a8ccd5c7f8b8464f802b13c7e6e2b7477a772c7b95d1661",
      "0x810293b9122ead61f766ba13daf8d93afb6bdaa43fc4554ff539f17753b3e80d",
      "0xa6d3af6f2354fdf4c11278dcdd17e76247a1e441693d6b4c57f65c2f07790cfe",
      "0x762b4676abfae3aca71c65bec7cb095c4319a34a8cae8e7da1b8bae73f2a8e5a",
      "0x3479a5d63f9ed8e48a752efa919e4d862576b2a7d7573012d25b9c9bde95fad1",
      "0x5b84192ffdb8c4e877b318d2781dd36ee4197b36d2d5f0363dec79caec2017ce",
      "0xb7c4abdb7126bc3580736fceb860abb1e081f6b07e049e79d31f451423a83002",
      "0xbe7cf74cabed781d4042fbadaf195024b2ed49803292be7509197fbc03db8d57",
      "0xbfc7421f049fb01f48cdfcfff5b4f1ce65e03f69e1ccf1ed838817702ca6a60f",
      "0x6521a23e5b1f1a310bea2f4c49b99ad5e6585b0d58f88690ed4bde7fa9a3d250",
      "0x9188e5b8fa1ef5a250dabfddcaeb58a14845236cb1ed032e72f808a524cdd073",
      "0x81402e1086321a7e5d8180caf5637e33b7c58652a549252e592137bcff401b6d",
      "0x74b353b4e880d4637a576ea9e39dd1b7b6a3b76bc98ae2df368a3b566433b329",
      "0x59019f2303c2249d0d47798a19922ab4f0754a3da765005d267bac93cb93212c",
      "0x3efdc1f8524919739a8c3d430bc75b0eca83b3b3a932d9ee93bcb50fc595f9b0",
      "0x3a0ed883adb0cc66075c2e0870f306fb228b7ee4c6896963411774dd13cf7d6a",
      "0xf1ed8090ff2bfd00bea0841aa8f9e537b46d45e5b3d48eb36015f0669ee083e8",
      "0xed81c97f537dd7680f4364b6aa3a598b34e41981e04b0a0b5dc7c05dec94b567",
      "0x78d369a69c1564d81a434b25599668dbb6deb37fc8b427c34daf3093c078b43f",
      "0xa6861f88ba3cef523e2a67d0fe2851721bf40b80807ea637c4009b1d84c0aea4",
      "0x2f2878cf1ba81fb633f65952ff062a2500aaafd735157ea4738e97433df4ea78",
      "0x508760a4a7497bd6fa2d7bced31febf23595324495c1c5bceb837df5912a435a",
      "0x7538dbdccab842cfb9cda020708e653073604a713bbb48fbc9181d36c28babe3",
      "0xd4ecbaab65ee20420df35aeac37d0b2e348f9db7366818094e2913ca8c1952d1",
      "0xc8560c92486fac9b1c5929d56b99de9d390ca9241a3b31b4095132c5cd75d190",
      "0xee957e7b8a8f3bdf443e4519dec2b7a1c89c215c4406329526ee8eecb068d5ee",
      "0xcbb3f2d674ca162fa3aa83e0ae5598b033169c3583a741f1a1c2b0213cba3968",
      "0xa54a050bcb362956ed444af51416de177f82b6c556809b58e957ced5bfd49522",
      "0x905ff9c768b5d069a9a534f210e426b03ef4e026b0a3fe6bcaf3a74bd1099a73",
      "0xd8c93abffc6c671c3960e6a6a810da37fa15b13535761d1d3b1c740a60d7d57d",
      "0xd9b933624e37be890eb29e689d5cde77fc4a9e2649edd35e06e5d8fdd88f0bc2",
      "0x2a19dc9f611aea88c8c870bb345ba245e256f734d34903dbf0f76139c989068a",
      "0xf22e237f1c9ec03bb9c7cedd60d00dab3dbf0cac4901129026f66378034ede7f",
      "0x2c7576729d3458c6b73f87a4649a1f14c31cbf4a7efaaeec03e8fe4757354b17",
      "0x9e5019d3c4721a0e88a05d45f3eb9f9e58dbcf12686bfb23b4a5a34b0396665b",
      "0x05d6fa8e12d95c6e5f7c680b7caff23835e9130031d015c6663753bd7e7199fa",
      "0x8239e320e3daccb0337aa926f161efea32914b9ce5411ff887fb068c3373c526"
    ],
    "transactionsRoot": "0xcea8f46d74dc7f3358b495b861b44ddea6f3a10cb54b2d6e2ab654b559f87fb5",
    "uncles": []
  },
  "id": 0
}
```

- Hydrated transactions == true

```json
{
  "jsonrpc": "2.0",
  "result": {
    "number": "0xfa6cfe",
    "hash": "0xccc182975a06d2a08b562af18d44a59f86d50b5380f3480f286ac9136fad19a9",
    "transactions": [
      {
        "blockHash": "0xccc182975a06d2a08b562af18d44a59f86d50b5380f3480f286ac9136fad19a9",
        "blockNumber": "0xfa6cfe",
        "hash": "0xb355db968d1ef974a683734d14e324f167c6cadd169a865a0731dc396ced8c3e",
        "chainId": "0x1",
        "from": "0x686434694edd29ca7da32846ce01141b3e611f76",
        "gas": "0x5208",
        "gasPrice": "0xa7a358200",
        "input": "0x",
        "nonce": "0x0",
        "r": "0x7b44416ad2d48fbd10b9ceb32d474872c03bb159f91a28f83dbfe9fb5d29992e",
        "s": "0x5c4d415a5b40aa6add902cb8313b86847d17f421a0b1fa63daf79f23bca6ce20",
        "to": "0xf4f304f964143363e2ccb4662f70cb5ffd839ef4",
        "transactionIndex": "0x0",
        "type": "0x0",
        "v": "0x26",
        "value": "0xce72bcb3e5e000"
      },
      {
        "blockHash": "0xccc182975a06d2a08b562af18d44a59f86d50b5380f3480f286ac9136fad19a9",
        "blockNumber": "0xfa6cfe",
        "hash": "0x08c6ca2b975e129db15039daadd25bfa71273d1068315130aca6e983b76f53a1",
        "chainId": "0x1",
        "from": "0xec1622a5761ae181c2a3115b268c66fb70229c2f",
        "gas": "0x5208",
        "gasPrice": "0xa7a358200",
        "input": "0x",
        "nonce": "0x0",
        "r": "0xa34646d589e101d3efc51324a8ce3b3117e69d58691d6de418b3689d9a61ad96",
        "s": "0x4514b79dcd3886eff15dd0cb73019be0bf4fc2b58ae42d6374778db5d199a75c",
        "to": "0xe795aeb69c509360e6a18bb87877959bcb36f360",
        "transactionIndex": "0x1",
        "type": "0x0",
        "v": "0x26",
        "value": "0xce72bcb3e5e000"
      },
      {
        "blockHash": "0xccc182975a06d2a08b562af18d44a59f86d50b5380f3480f286ac9136fad19a9",
        "blockNumber": "0xfa6cfe",
        "hash": "0x357aa393894b5cf00cd84357b2f66f6cba2f520fd73b7601c2ce538e9d57552d",
        "chainId": "0x1",
        "from": "0xa1cfba2be3381065102435b1b699a9db334aefdc",
        "gas": "0x5208",
        "gasPrice": "0xa7a358200",
        "input": "0x",
        "nonce": "0x154",
        "r": "0x3360578b58f4787e57a25f9115c188c25b96c170cece8714a5267a1913a8084a",
        "s": "0x6f22a9cd5283cd2e50937f3c27a4728e99d12ab675e38c9acf894b7b63b23451",
        "to": "0x93527e24d8722d27f68f892b1bc4635fe0999068",
        "transactionIndex": "0x2",
        "type": "0x0",
        "v": "0x26",
        "value": "0xd1ce35a935f000"
      },
      {
        "blockHash": "0xccc182975a06d2a08b562af18d44a59f86d50b5380f3480f286ac9136fad19a9",
        "blockNumber": "0xfa6cfe",
        "hash": "0x0a6d2bc6d1f291cda545b4fe2eb2d84b12c3ae719c1b56a66dcfdcb0554eabc4",
        "accessList": [],
        "chainId": "0x1",
        "from": "0x267be1c1d684f78cb4f6a176c4911b741e4ffdc0",
        "gas": "0x5208",
        "gasPrice": "0x3b859434b",
        "input": "0x",
        "maxFeePerGas": "0xe2f783975",
        "maxPriorityFeePerGas": "0x77359400",
        "nonce": "0x2c995c",
        "r": "0xaf4036e3b3b3e3179ad251c30b74fd3a64659143dc0bae2cd8fa94793099ba67",
        "s": "0x70022f5928f77fbec33479c401960da9fff81e7c40b664f4bff6b4b42df9e95c",
        "to": "0xfb39a44778feb935a6fa0cd9069e643e650c4eb6",
        "transactionIndex": "0x25",
        "type": "0x2",
        "v": "0x0",
        "value": "0x156d63d1cd34000"
      },
      {
        "blockHash": "0xccc182975a06d2a08b562af18d44a59f86d50b5380f3480f286ac9136fad19a9",
        "blockNumber": "0xfa6cfe",
        "hash": "0x2b4ff5196e42fc7c1d38080067376a1b3c09626c16db1745c629bdbd0566c5e6",
        "accessList": [],
        "chainId": "0x1",
        "from": "0x65768f7e9448595181271a0af9a2351d383ace1a",
        "gas": "0x5208",
        "gasPrice": "0x3b859434b",
        "input": "0x",
        "maxFeePerGas": "0x6898973e0",
        "maxPriorityFeePerGas": "0x77359400",
        "nonce": "0x1dfc",
        "r": "0xd925c402caabfa9193f2fe63ddb552f6583ddd43aa79f4358f992c483317d0a8",
        "s": "0x47af42535f46a4e41631eb59f3174e3b0661bdddd99a4405a2b2d31fa59989d3",
        "to": "0x4b92df8a350df7a791b1c8e090aae3a84df79282",
        "transactionIndex": "0x26",
        "type": "0x2",
        "v": "0x0",
        "value": "0x3323c9376c816000"
      },
      {
        "blockHash": "0xccc182975a06d2a08b562af18d44a59f86d50b5380f3480f286ac9136fad19a9",
        "blockNumber": "0xfa6cfe",
        "hash": "0x05c467b7f679df097f77918ed44340c0c5285620c3aaa02d15d6e5b4481eebb4",
        "accessList": [],
        "chainId": "0x1",
        "from": "0x3018018c44338b9728d02be12d632c6691e020d1",
        "gas": "0x186a0",
        "gasPrice": "0x3b859434b",
        "input": "0x23b872dd000000000000000000000000a1fba3f99daee41660f2eca885dd3fb5667ad6790000000000000000000000003018018c44338b9728d02be12d632c6691e020d100000000000000000000000000000000000000000000000000000003d3aef340",
        "maxFeePerGas": "0x2a600b9c00",
        "maxPriorityFeePerGas": "0x77359400",
        "nonce": "0x39f2d",
        "r": "0x86074e17994ef497784b1241606a19a1ebf8490e50304a76b1c1bcc3d035bb9d",
        "s": "0x52f0793bff6dc0e8e86f999ad6ebb906b8f21067699b226241d330e07b7f246c",
        "to": "0xdac17f958d2ee523a2206206994597c13d831ec7",
        "transactionIndex": "0x27",
        "type": "0x2",
        "v": "0x1",
        "value": "0x0"
      },
      {
        "blockHash": "0xccc182975a06d2a08b562af18d44a59f86d50b5380f3480f286ac9136fad19a9",
        "blockNumber": "0xfa6cfe",
        "hash": "0x986966662320a31d86d07d72d433a24f25477d2ddaef5f3fbde0626880179738",
        "accessList": [],
        "chainId": "0x1",
        "from": "0xdfd5293d8e347dfe59e90efd55b2956a1343963d",
        "gas": "0x32918",
        "gasPrice": "0x3b859434b",
        "input": "0xa9059cbb00000000000000000000000093b6e24976a2a3bbed11feb840b47727be914d0d000000000000000000000000000000000000000000009ed194db19b238c00000",
        "maxFeePerGas": "0x17bfac7c00",
        "maxPriorityFeePerGas": "0x77359400",
        "nonce": "0x4df269",
        "r": "0x76ee4d05a6d8e4fcdf403ff02b091926954e61f9727aea1c444bf565e2b09242",
        "s": "0x7081f8ce1da46205b1cd573c74b1d755cc177ec2a15888d3dd91b567a755c6f9",
        "to": "0x579cea1889991f68acc35ff5c3dd0621ff29b0c9",
        "transactionIndex": "0x28",
        "type": "0x2",
        "v": "0x0",
        "value": "0x0"
      },
      {
        "blockHash": "0xccc182975a06d2a08b562af18d44a59f86d50b5380f3480f286ac9136fad19a9",
        "blockNumber": "0xfa6cfe",
        "hash": "0x3c9b83fbc79b3320931bfb05149a2c9a5ff6888d19bb09b2d1477db4cd73fe6f",
        "accessList": [],
        "chainId": "0x1",
        "from": "0x28c6c06298d514db089934071355e5743bf21d60",
        "gas": "0x32918",
        "gasPrice": "0x3b859434b",
        "input": "0x",
        "maxFeePerGas": "0x17bfac7c00",
        "maxPriorityFeePerGas": "0x77359400",
        "nonce": "0x55dc36",
        "r": "0xe10c6e26cd76cd39dca5a19ced2d7385b6c002b8c0ff4d6a10557ff43b0d9a7b",
        "s": "0x1cdce499efbc788c9ecc753235e1243ca517c787b820bc87a2dd8e193c9a7fb7",
        "to": "0x6169eac0b250bbfe075f95540445080a93d0b180",
        "transactionIndex": "0x29",
        "type": "0x2",
        "v": "0x1",
        "value": "0x9bf9ccee8bd41c00"
      },
      {
        "blockHash": "0xccc182975a06d2a08b562af18d44a59f86d50b5380f3480f286ac9136fad19a9",
        "blockNumber": "0xfa6cfe",
        "hash": "0xcf52556018d02689de81d85b8a35c6c7306915e07c6c9f9e70b3d91d87205d71",
        "accessList": [],
        "chainId": "0x1",
        "from": "0x56eddb7aa87536c09ccc2793473599fd21a8b17f",
        "gas": "0x32918",
        "gasPrice": "0x3b859434b",
        "input": "0x",
        "maxFeePerGas": "0x17bfac7c00",
        "maxPriorityFeePerGas": "0x77359400",
        "nonce": "0x3f17c5",
        "r": "0x249da2abe3d1345deccea75c144298def0bb5e0a7fa72102db34c9add0099ada",
        "s": "0x60c0343699d6a8995fec3e915853792ae175f93816aecf1ef8e56ab7f9fe115c",
        "to": "0xb93618838ddc2b34e2f6dcf7731be682f77fe796",
        "transactionIndex": "0x2a",
        "type": "0x2",
        "v": "0x1",
        "value": "0x1179e34f2b96f8000"
      },
      {
        "blockHash": "0xccc182975a06d2a08b562af18d44a59f86d50b5380f3480f286ac9136fad19a9",
        "blockNumber": "0xfa6cfe",
        "hash": "0x7bd0f1b802c4995dbd1ddf3c84880e33f589227e525c7dd133b2f9d348421e32",
        "accessList": [],
        "chainId": "0x1",
        "from": "0x21a31ee1afc51d94c2efccaa2092ad1028285549",
        "gas": "0x32918",
        "gasPrice": "0x3b859434b",
        "input": "0x",
        "maxFeePerGas": "0x17bfac7c00",
        "maxPriorityFeePerGas": "0x77359400",
        "nonce": "0x51be6a",
        "r": "0x16198b7ddb270dd07477cce03fde1e100ebbcea7ae6410fbc8bc8876af02359",
        "s": "0x4b53c73f7a79af2650f191568fabb1ab08f8d6f507bca1090ecadaf58f60fde0",
        "to": "0xda25824ee511fd8cbfce7dc8b316820418a34a10",
        "transactionIndex": "0x2b",
        "type": "0x2",
        "v": "0x1",
        "value": "0xa9a2f36fc4770000"
      },
      {
        "blockHash": "0xccc182975a06d2a08b562af18d44a59f86d50b5380f3480f286ac9136fad19a9",
        "blockNumber": "0xfa6cfe",
        "hash": "0xfdc34ec89a99fcb6e7bbfc452e812c7c978b7e7201d52b321efbfe3e0d947d68",
        "chainId": "0x1",
        "from": "0x0b4ab48cbdf3f508defa785321be034f942e8993",
        "gas": "0x5208",
        "gasPrice": "0x342770c00",
        "input": "0x",
        "nonce": "0xa",
        "r": "0x9a1600b6fe134bf7121320b06bee37fa538cf7f04ef3cb0c3c86c06ecc2dbb0",
        "s": "0x4873f78fae494881a696f752066dddb66f26e393c41f44d722087a67508c40aa",
        "to": "0x47e407827f461888c07c437db3017d88a848847d",
        "transactionIndex": "0x8c",
        "type": "0x0",
        "v": "0x26",
        "value": "0x832afb1f56400"
      },
      {
        "blockHash": "0xccc182975a06d2a08b562af18d44a59f86d50b5380f3480f286ac9136fad19a9",
        "blockNumber": "0xfa6cfe",
        "hash": "0xa843a463e45949de80dee304d7fd57fdfca7393d9ef04b04a8b76962d781af38",
        "accessList": [
          {
            "address": "0x795065dcc9f64b5614c407a6efdc400da6221fb0",
            "storageKeys": [
              "0x000000000000000000000000000000000000000000000000000000000000000c",
              "0x0000000000000000000000000000000000000000000000000000000000000008",
              "0x0000000000000000000000000000000000000000000000000000000000000006",
              "0x0000000000000000000000000000000000000000000000000000000000000007",
              "0x0000000000000000000000000000000000000000000000000000000000000009",
              "0x000000000000000000000000000000000000000000000000000000000000000a"
            ]
          },
          {
            "address": "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
            "storageKeys": [
              "0xe74eef0a77feef61a512352c6d67d19c6777a09854d041c6c84bf82fe1b30d93",
              "0x20db7a6b0cae05dc43c7588d71a84c7675222d2a6ca56a16f22ea9ae044b0146",
              "0xfd2563dd5f0a7456bfcf35694e0e1df69d74318cc435a2783a2d120a6e36c885"
            ]
          },
          {
            "address": "0x36e2fcccc59e5747ff63a03ea2e5c0c2c14911e7",
            "storageKeys": [
              "0x000000000000000000000000000000000000000000000000000000000000000a",
              "0x000000000000000000000000000000000000000000000000000000000000000c",
              "0x0000000000000000000000000000000000000000000000000000000000000008",
              "0x0000000000000000000000000000000000000000000000000000000000000006",
              "0x0000000000000000000000000000000000000000000000000000000000000007",
              "0x0000000000000000000000000000000000000000000000000000000000000009"
            ]
          },
          {
            "address": "0x8798249c2e607446efb7ad49ec89dd1865ff4272",
            "storageKeys": [
              "0x83cef273285dc689a828b24143ff70fff6f9e4f54e620c063786797ee641a577",
              "0x2c742da69f54b072feca7a535a769e253f9323cc755cd3eb7a108ae1f89e0679",
              "0x0000000000000000000000000000000000000000000000000000000000000002",
              "0x0000000000000000000000000000000000000000000000000000000000000005"
            ]
          },
          {
            "address": "0x6b3595068778dd592e39a122f4f5a5cf09c90fe2",
            "storageKeys": [
              "0x7680d8abe5dc20991fcb0c7b95636582d139e7626e26f5e78ea0d53ad541434d",
              "0x2c742da69f54b072feca7a535a769e253f9323cc755cd3eb7a108ae1f89e0679",
              "0xa60c07f2aed92cf0e2ca94448542cb8f5cc91bf932d411877ec1850bf66a155f"
            ]
          }
        ],
        "chainId": "0x1",
        "from": "0xa06c3c08a19e51b33309eddfb356c33ead8517a3",
        "gas": "0x200b24",
        "gasPrice": "0x34274aba3",
        "input": "0xa100fa6cfe00c02aaa39b223fe8d0a0e5c4f27ead9083c756cc20044a9059cbb00000000000000000000000036e2fcccc59e5747ff63a03ea2e5c0c2c14911e70000000000000000000000000000000000000000000000000de0b6b3a76400000036e2fcccc59e5747ff63a03ea2e5c0c2c14911e700c4022c0d9f000000000000000000000000000000000000000000000031f1a784bafdc1320d0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000e4000004000bd8006e00720000d27d1fa000d43e000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008798249c2e607446efb7ad49ec89dd1865ff4272002467dfd4c9000000000000000000000000000000000000000000000031f1a784bafdc1320d006b3595068778dd592e39a122f4f5a5cf09c90fe20044a9059cbb000000000000000000000000795065dcc9f64b5614c407a6efdc400da6221fb00000000000000000000000000000000000000000000000437f58b50cec13134d00795065dcc9f64b5614c407a6efdc400da6221fb000c4022c0d9f00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000de9be9f8ab78547000000000000000000000000e4000004000bd8006e00720000d27d1fa000d43e000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "maxFeePerGas": "0x34274aba3",
        "maxPriorityFeePerGas": "0x150fc58",
        "nonce": "0x5bb3",
        "r": "0xc7d17dfb78812b17a3ccd9790748ce1a7def93b10940d745ecf372878233d8f6",
        "s": "0x59b1802ef319d825d63b72d03a49460d93f88727da20b335f525b7cfcab306bd",
        "to": "0xe4000004000bd8006e00720000d27d1fa000d43e",
        "transactionIndex": "0x8d",
        "type": "0x2",
        "v": "0x0",
        "value": "0x0"
      },
      {
        "blockHash": "0xccc182975a06d2a08b562af18d44a59f86d50b5380f3480f286ac9136fad19a9",
        "blockNumber": "0xfa6cfe",
        "hash": "0x460a2da11381302998e52e0fe8852e585912209e506fc0ed25858c9e16aea952",
        "accessList": [],
        "chainId": "0x1",
        "from": "0xdafea492d9c6733ae3d56b7ed1adb60692c98bc5",
        "gas": "0x5208",
        "gasPrice": "0x34123af4b",
        "input": "0x",
        "maxFeePerGas": "0x34123af4b",
        "maxPriorityFeePerGas": "0x0",
        "nonce": "0x2c08e",
        "r": "0x78f8646f1d3076f1b1e09555892fd16587be205cf925db665b50a6031c7f7506",
        "s": "0x5e865384cca7ab76ba6e5ba23decc7fb6246400b2ce4ec46d0655b2f51ed0a1f",
        "to": "0x4675c7e5baafbffbca748158becba61ef3b0a263",
        "transactionIndex": "0x8e",
        "type": "0x2",
        "v": "0x1",
        "value": "0x679d8fad824b9a"
      }
    ],
    "difficulty": "0x0",
    "extraData": "0x496c6c756d696e61746520446d6f63726174697a6520447374726962757465",
    "gasLimit": "0x1c9c380",
    "gasUsed": "0xbf99ce",
    "logsBloom": "0x3fead0d3e1844c26f3099280cb1053a9046260033ae805c4a0cf4700544237d54887f3954841242106345f88e01983698a6700481fe3b9572bcad8ab10bfbc214e472811cd1099886a33080911b32ced878038a185701080d024cd1b9a0762515a5680f4833654248002f904106bbc91f282a054135d370e1e0844f0519c408d090d875c272658107f58234581a249c63d01dcc5ed73847967e225eb40385860bac703785a8366c70abcccc475200e322bf640051820e00e477b0a86044a8517f1103056e40839479dd9020f404e5ec519e13840b03a485d49a1590f0191b0c4d879a9492886861017371ea21d8448c6af1d7e0048081146265f89c2e423b323",
    "miner": "0xdafea492d9c6733ae3d56b7ed1adb60692c98bc5",
    "mixHash": "0x1d68a2eac8af20656a10fd1e99eacd6ce414abf1a133d82e806353e84c2a49f4",
    "nonce": "0x0000000000000000",
    "parentHash": "0x122d4d5dff33383d2fa964d405e5d6f91af5927bff27c53070cfe32aeca450b1",
    "receiptsRoot": "0x2366e1233bd84bc653ea606d49e40847363bbfbc8552c4b99d5a126cea70b81b",
    "sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
    "size": "0xf619",
    "stateRoot": "0xf4b5543a220a54bb899c2127fb69faba3bb166e87ad2c9f3e18dac098c43621a",
    "timestamp": "0x63c3e333",
    "totalDifficulty": "0xc70d815d562d3cfa955",
    "transactionsRoot": "0xc7771c9e952a2b31fd83859a3e8a3df48c8a75c8c0568f615ebfacbc505881f7",
    "uncles": [],
    "baseFeePerGas": "0x34123af4b"
  },
  "id": 0
}
```
