<div align="center">
  <h1>Kakarot RPC</h1>
  <p align="center">
    <img src="docs/images/kakarot_github_banner.png" width="700">
  </p>
  <br />
  <a href="https://github.com/sayajin-labs/kakarot-rpc/issues/new?assignees=&labels=bug&template=01_BUG_REPORT.md&title=bug%3A+">
  Report a Bug
  </a>
  -
  <a href="https://github.com/sayajin-labs/kakarot-rpc/issues/new?assignees=&labels=enhancement&template=02_FEATURE_REQUEST.md&title=feat%3A+">
  Request a Feature
  </a>
  -
  <a href="https://github.com/sayajin-labs/kakarot-rpc/discussions">Ask a Question</a>
</div>

<div align="center">
<br />

![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/kkrt-labs/kakarot-rpc/push.yml?branch=main)
[![Project license](https://img.shields.io/github/license/sayajin-labs/kakarot-rpc.svg?style=flat-square)](LICENSE)
[![Pull Requests welcome](https://img.shields.io/badge/PRs-welcome-ff69b4.svg?style=flat-square)](https://github.com/sayajin-labs/kakarot-rpc/issues?q=is%3Aissue+is%3Aopen+label%3A%22help+wanted%22)

</div>

<details open="open">
<summary>Table of Contents</summary>

- [Report a Bug](https://github.com/sayajin-labs/kakarot-rpc/issues/new?assignees=&labels=bug&template=01_BUG_REPORT.md&title=bug%3A+")
- [Request a Feature](https://github.com/sayajin-labs/kakarot-rpc/issues/new?assignees=&labels=enhancement&template=02_FEATURE_REQUEST.md&title=feat%3A+≠≠≠≠≠≠≠)
- [About](#about)
- [Architecture](#architecture)
  - [High level](#high-level)
  - [Low level](#low-level)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
- [Installation](#installation)
  - [Setup the project](#setup-the-project)
  - [Build from source](#build-from-source)
  - [Environment variables](#environment-variables)
  - [Dev mode with Katana](#dev-mode-with-katana)
  - [Building a Docker Image](#building-a-docker-image)
  - [Sending transactions to RPC using forge script](#sending-transactions-to-rpc-using-forge-script)
  - [Configuration](#configuration)
- [Running a Node in Various Environments](#running-a-node-in-various-environments)
  - [Local Environment](#local-environment)
  - [Staging Environment](#staging-environment)
  - [Production Environment](#production-environment)
  - [Potential Pitfalls, Caveats, and Requirements](#potential-pitfalls-caveats-and-requirements)
    - [Requirements](#requirements)
    - [Potential Pitfalls](#potential-pitfalls)
    - [Caveats](#caveats)
  - [API](#api)
- [Testing](#testing)
  - [Rust tests](#rust-tests)
  - [Apibara indexer tests](#apibara-indexer-tests)
  - [Hive](#hive)
- [Project assistance](#project-assistance)
- [Contributing](#contributing)
- [Glossary](#glossary)
- [Authors \& contributors](#authors--contributors)
- [Security](#security)
- [License](#license)
- [Acknowledgements](#acknowledgements)
- [Benchmarks](#benchmarks)
- [Contributors ✨](#contributors-)

</details>

---

## About

Kakarot RPC fits in the three-part architecture of the Kakarot zkEVM rollup
([Kakarot EVM Cairo Programs](https://github.com/kkrt-labs/kakarot), Kakarot
RPC, [Kakarot Indexer](indexer/README.md)). It is the implementation of the
Ethereum JSON-RPC specification made to interact with Kakarot zkEVM in a fully
Ethereum-compatible way.

![Kakarot zkEVM architecture](./docs/images/Kakarot%20zkEVM.png)

The Kakarot RPC layer's goal is to receive and output EVM-compatible payloads &
calls while interacting with an underlying StarknetOS client. This enables
Kakarot zkEVM to interact with the usual Ethereum tooling: Metamask, Hardhat,
Foundry, etc.

Note that this is necessary because Kakarot zkEVM is implemented as a set of
Cairo Programs that run on an underlying CairoVM (so-called StarknetOS) chain.

This adapter layer is based on:

- [The Ethereum JSON-RPC spec](https://github.com/ethereum/execution-apis/tree/main/src/eth)
- [The Starknet JSON-RPC spec](https://github.com/starkware-libs/starknet-specs/blob/master/api/starknet_api_openrpc.json)
- [And their differences](https://github.com/starkware-libs/starknet-specs/blob/master/starknet_vs_ethereum_node_apis.md)

## Architecture

### High level

Here is a high level overview of the architecture of Kakarot RPC.

![Kakarot RPC Adapter flow](<./docs/images/Kakarot%20RPC%20(lower%20level).png>)

### Low level

Below is a lower level detailed overview of the internal architecture.
![Kakarot RPC Adapter flow](https://github.com/kkrt-labs/ef-tests/assets/82421016/4b34cbbb-df50-4ce3-9aaa-ed42b80ecd3b)

## Getting Started

TL;DR:

- Run `make setup` to build dependencies.
- Run `cargo build` to build Kakarot RPC.
- Test with `make test`.
- Run Kakarot RPC in dev mode:
  - Run dev RPC: `make run-dev` (you'll need a StarknetOS instance running in
    another process and Kakarot contracts deployed)
- Run with Docker Compose:
  - `make local-rpc-up`
  - To kill these processes, `make docker-down`
- Build the docker image for the RPC:
  - `make docker-build`

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install): The codebase is written in
  Rust to ensure high performance, maintainability, and a developer-friendly
  experience.
- [Docker](https://docs.docker.com/engine/install): Required for containerizing
  and running the various services and components in a consistent environment.
- [Python](https://www.python.org/): Used primarily for interacting with and
  building our Kakarot programs.
- [Poetry](https://python-poetry.org/docs/): A Python dependency management tool
  used for managing the dependencies of our Kakarot programs.
- [Deno](https://docs.deno.com/runtime/manual/): A JavaScript runtime used for
  our indexing service, based on the [Apibara](https://www.apibara.com/docs)
  third-party service.
- Make: Utilized to interact with the `Makefile` for running commands such as
  building the project or executing tests.

## Installation

### Setup the project

To set up the repository (pulling git submodule and building Cairo
dependencies), run:

```console
make setup
```

Caveats:

1. the `setup` make command uses linux (MacOs compatible) commands to allow
   running the `./scripts/extract_abi.sh`. This script is used to use strongly
   typed Rust bindings for Cairo programs. If you encounter problems when
   building the project, try running `./scripts/extract_abi.sh`.
2. the [kakarot](https://github.com/kkrt-labs/kakarot) submodule uses Python to
   build and deploy Kakarot contracts. If you don't have the right version
   available, we recommend to use [pyenv](https://github.com/pyenv/pyenv) to
   install it.
3. We use a pre-commit hook to ensure code quality and consistency. The hook are
   managed and automatically installed by trunk.

### Build from source

To build the project from source (in release mode):

```console
cargo build --release
```

Note that there are sometimes issues with some dependencies (notably scarb or
cairo related packages, there are sometimes needs to `cargo clean` and
`cargo build`)

### Environment variables

Copy the `.env.example` file to a `.env` file and populate each variable

```console
cp .env.example .env
```

Meanwhile you can just use unit tests to dev.

```console
make test
```

The binaries will be located in `target/release/`.

### Dev mode with [Katana](https://github.com/dojoengine/dojo/tree/main/crates/katana)

To run a local Starknet sequencer, you can use Katana. Katana, developed by the
Dojo team, is a sequencer designed to aid in local development. It allows you to
perform all Starknet-related activities in a local environment, making it an
efficient platform for development and testing. To run Katana and deploy the
Kakarot zkEVM (a set of Cairo smart contracts implementing the EVM):

```console
make run-katana
```

This command will install Katana and generate a genesis file at
`.katana/genesis.json`. Katana's genesis configuration feature is a way to
define the initial state and settings of the Kakarot blockchain network locally,
providing a customizable starting point for the chain. Among other things, it
allows you to:

- Specify the token used for network fees.
- Allocate initial token balances to accounts.
- Pre-declare classes at the start of the chain.
- Pre-deploy smart contracts at the start of the chain.

To deploy Kakarot Core EVM (set of Cairo Programs):

```console
 make deploy-kakarot
```

To run the Kakarot RPC pointing to this local devnet:

```console
STARKNET_NETWORK=katana make run-dev
```

Some notes on this local devnet:

- this will run a devnet by running katana, **with contracts automatically
  deployed**, so you don't have to do them manually (see in
  `./lib/kakarot/kakarot_scripts/deploy_kakarot.py` for the list of contracts).

- the deployments and declarations for the devnet will be written to the
  `deployments/katana` folder inside your project root after a successful run of
  the `make deploy-kakarot` command.

### Building a [Docker Image](https://docs.docker.com/reference/cli/docker/image/build/)

In order to build a Docker Image for the RPC, you can run the below command
which will setup the local environment and compile the binary:

```console
make docker-build
```

### Sending transactions to RPC using [forge script](https://book.getfoundry.sh/reference/forge/forge-script)

An example script to run which uses a pre-funded EOA account with private key
`EVM_PRIVATE_KEY`

```console
forge script scripts/PlainOpcodes.s.sol --broadcast --legacy --slow
```

### Configuration

Kakarot RPC is configurable through environment variables. Check out
`.env.example` file to see the environment variables.

## Running a Node in Various Environments

This section outlines how to run a complete node in different environments:
local, staging, and production. Running a node involves several critical
components to ensure the system operates effectively:

- **Starknet Engine**: Interacts with the Starknet ecosystem and processes
  transactions.
- **Kakarot Programs**: Implement the EVM logic using Cairo.
- **RPC Node**: Manages the Ethereum RPC logic, facilitating smooth interaction
  with the Kakarot chain.
- **Apibara Service**: Monitors the Kakarot chain and indexes its data.
- **MongoDB**: Serves as the database for storing transactions after indexing
  and acts as the core component for fetching information.

By correctly configuring these components, you can ensure that the node
functions as a robust part of the system.

In the following sections we have tried to provide the most important parameters
useful for understanding and configuring the node. However for the sake of
brevity, certain parameters deemed less important are omitted and can all be
found in the corresponding Docker compose files:

- Local: `docker-compose.yaml`
- Staging: `docker-compose.staging.yaml`
- Production: `docker-compose.prod.yaml`

### Local Environment

To start the entire infrastructure locally, use the following command:

```console
make local-rpc-up
```

This command will use the `docker-compose.yaml` file to set up the whole
infrastructure locally utilizing the following elements:

- **Katana (local sequencer)**:

  - Fees disabled (ETH and STRK gas price set to 0).
  - Maximum steps for account validation logic set to 16,777,216.
  - Maximum steps for account execution logic set to 16,777,216.
  - Chain ID set to KKRT (0x4b4b5254 in ASCII).

- **Kakarot EVM Programs**:

  - Prefunded Katana account with:
    - Account address:
      `0xb3ff441a68610b30fd5e2abbf3a1548eb6ba6f3559f2862bf2dc757e5828ca`.
    - Private key:
      `0x2bbf4f9fd0bbb2e60b0316c1fe0b76cf7a4d0198bd493ced9b8df2a3a24d68a`.
  - Anvil (local Ethereum node):
    - Private key:
      `0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80`.
  - Katana RPC URL: `http://starknet:5050`.
  - Network: `STARKNET_NETWORK=katana`.

- **Kakarot RPC Node** on port 3030:

  - MongoDB connection string:
    `MONGO_CONNECTION_STRING=mongodb://mongo:mongo@mongo:27017`.
  - Database name: `MONGO_DATABASE_NAME=kakarot-local`.
  - Max calldata felts: 30,000.
  - Pending transactions stored in MongoDB, with a retry service running every
    second.
  - Currently, Kakarot does not support pre-EIP-155 transactions, except for a
    whitelist of specific transaction hashes that can be found in the
    corresponding Docker compose file.

- **Apibara Indexer Service** on port 7171:

  - Uses the Starknet node URL for RPC.
  - Configured with MongoDB and Kakarot addresses.

- **MongoDB** with Mongo Express on port 27017 for data management.
- **Blockscout** on port 4000, provides a web interface for exploring and
  analyzing blockchain data.

### Staging Environment

To start the entire infrastructure in the staging environment, use the following
command:

```console
make staging-rpc-up
```

This command will use the `docker-compose.staging.yaml` file to set up the whole
infrastructure in the staging configuration utilizing the following elements:

- **Starknet Full-Node (Juno)** on port 6060:

  - Pending block is synced to the head of the chain every second.
  - Ethereum node websocket endpoint to be specified by env variable
    `ETH_NODE_WS` (for example
    `ETH_NODE_WS=wss://eth-sepolia.g.alchemy.com/v2/YOUR_API_KEY`).
  - Network configuration:
    - Network name: `KKRT_BETA`.
    - Network feeder URL:
      `https://gateway-beta.kakarot.sw-dev.io/feeder_gateway/`.
    - Network gateway URL: `https://gateway-beta.kakarot.sw-dev.io/gateway/`.
    - L1 chain ID: `11155111` (Ethereum Sepolia).
    - L2 chain ID: `kkrt`.
    - Core contract address: `0xc7c9ea7fD0921Cb6EDd9a3184F88cF1b821aA82B`.
    - Network range of blocks to skip hash verifications: `0` to `0`.

- **Starknet Explorer** on port 4000:

  - RPC API hosts.
  - Database connection details for Postgres.
  - Secret key base for security.
  - Listener enabled for synchronizing with the Starknet node.

- **Starknet Explorer Database (Postgres)** on port 5432.

- **Kakarot RPC Node** on port 3030:

  - Starknet network URL: `http://starknet:6060`.
  - MongoDB connection string: `mongodb://mongo:mongo@mongo:27017`.
  - Database name: `kakarot-local`.
  - Kakarot address:
    `0x2824d6ed6759ac4c4a54a39b78d04c0e48be8937237026bf8c3bf46a8bea722`.
  - Uninitialized account class hash:
    `0x600f6862938312a05a0cfecba0dcaf37693efc9e4075a6adfb62e196022678e`.
  - Max calldata felts: 30,000.
  - Pending transactions stored in MongoDB, with a retry service running every
    10 second.
  - Whitelisted pre-EIP-155 transaction hashes (see the corresponding Docker
    compose file).

- **Apibara DNA Indexer Service** on port 7171:

  - Uses the Starknet node URL for RPC.
  - Configured with MongoDB and Kakarot addresses.

- **MongoDB** with Mongo Express on port 27017 for data management.
- **Blockscout** on port 4001, provides a web interface for exploring and
  analyzing blockchain data.

### Production Environment

To start the entire infrastructure in the production environment, use the
following command:

```console
make testnet-rpc-up
```

This command will use the `docker-compose.prod.yaml` file to set up the whole
infrastructure in the production configuration utilizing the following elements:

- **Starknet Full-Node (Juno)** on port 6060:

  - Synchronizes pending blocks to the head of the chain every second.
  - Ethereum node websocket endpoint specified by `ETH_NODE_WS` (for example
    `ETH_NODE_WS=wss://eth-sepolia.g.alchemy.com/v2/YOUR_API_KEY`).
  - Network configuration:
    - Network name: `kakarot-sepolia`.
    - Network feeder URL: `https://gateway.kakarot.sw-dev.io/feeder_gateway/`.
    - Network gateway URL: `https://gateway.kakarot.sw-dev.io/gateway/`.
    - L1 chain ID: `11155111` (Ethereum Sepolia).
    - L2 chain ID: `kkrt`.
    - Core contract address: `0x74Ca1aC5BD4c3c97006d2B7b9375Dd3B6C17ACcD`.
    - Network range of blocks to skip hash verifications: `0` to `1000000`.

- **Starknet Explorer** on port 4000:

  - RPC API hosts.
  - Database connection details for Postgres.
  - Secret key base for security.
  - Listener enabled for synchronizing with the Starknet node.

- **Starknet Explorer Database (Postgres)** on port 5432.

- **Kakarot RPC Node** on port 3030:

  - Starknet network URL: `http://starknet:6060`.
  - MongoDB connection string: `mongodb://mongo:mongo@mongo:27017`.
  - Database name: `kakarot-local`.
  - Kakarot address:
    `0x11c5faab8a76b3caff6e243b8d13059a7fb723a0ca12bbaadde95fb9e501bda`.
  - Uninitialized account class hash:
    `0x600f6862938312a05a0cfecba0dcaf37693efc9e4075a6adfb62e196022678e`.
  - Account contract class hash:
    `0x1276d0b017701646f8646b69de6c3b3584edce71879678a679f28c07a9971cf`.
  - Max calldata felts: 30,000.
  - Pending transactions stored in MongoDB, with a retry service running every
    10 seconds.
  - Whitelisted pre-EIP-155 transaction hashes (see local environment).

- **Apibara DNA Indexer Service** on port 7171:

  - Uses the Starknet node URL for RPC.
  - Configured with MongoDB and Kakarot addresses.

- **MongoDB** with Mongo Express on port 27017 for data management.
- **Blockscout** on port 4001, provides a web interface for exploring and
  analyzing blockchain data.

### Potential Pitfalls, Caveats, and Requirements

When setting up the Kakarot node in any environment, it's important to be aware
of the following:

#### Requirements

- **Hardware**: Ensure your system meets the necessary hardware requirements for
  running Docker containers efficiently. A modern multi-core CPU, at least 16GB
  of RAM, and ample storage space are recommended.
- **Software**: Install the latest versions of Docker and Docker Compose to
  ensure compatibility with the provided configuration.
- **Network**: Stable internet connection for downloading images and
  communicating with remote services if needed. We have noticed difficulties on
  networks with low bandwidth.

#### Potential Pitfalls

- **Resource Limits**: Docker containers might consume significant system
  resources. Monitor system performance and consider adjusting container
  resource limits if necessary.
- **Network Configuration**: Ensure no port conflicts on your local machine,
  especially with ports 3030, 5050, 6060, 7171, 27017... used by the services.
- **Volume Persistence**: Docker volumes are used for data persistence. Ensure
  they are properly managed and backed up to prevent data loss.

#### Caveats

- **Pre-EIP-155 Transactions**: Kakarot does not natively support pre-EIP-155
  transactions, except for those whitelisted. Be cautious about transaction
  compatibility.
- **Environment Configuration**: Double-check environment variables and their
  values, particularly those related to security, such as private keys and
  database credentials.
- **Service Dependencies**: The order of service initialization is crucial.
  Dependencies between services must be respected to avoid runtime errors.

### API

You can take a look at `rpc-call-examples` directory. Please note the following:

- `sendRawTransaction.hurl`: the raw transaction provided allows to call the
  `inc()` function for the Counter contract. However, given that this
  transaction is signed for the EOA's nonce at the current devnet state (0x2),
  the call will only work once. If you want to keep incrementing (or
  decrementing) the counter, you need to regenerate the payload for the call
  with an updated nonce using the
  [provided python script](https://github.com/kkrt-labs/kakarot/blob/main/kakarot_scripts/utils/kakarot.py).

## Testing

### Rust tests

In order to execute the Rust tests, follow the below instructions:

- Run `make setup` in order to setup the project.
- Run `make test` which will create a Genesis test file for Kakarot and launch
  tests.
- If you which to only run a specific test, be sure to first at least run
  `make katana-genesis` once, then run
  `make test-target TARGET=test_you_want_to_run`.

### Apibara indexer tests

In order to run the Typescript unit tests, you will need to have
[Deno](https://docs.deno.com/runtime/manual/) installed. Then you can run
`KAKAROT_ADDRESS=ADDRESS_YOU_WANT_TO_USE_FOR_KAKAROT deno test --allow-env`.

### Hive

The [Hive](https://github.com/ethereum/hive/tree/master) end-to-end test suite
is set up in the Github Continuous Integration (CI) flow of the repository. This
ensures a safe guard when modifying the current RPC implementation and/or the
[execution layer](https://github.com/kkrt-labs/kakarot).

Due to the current existing differences between the Kakarot EVM implementation
which aims to be a type 2 ZK-EVM (see the blog post from
[Vitalik](https://vitalik.eth.limo/general/2022/08/04/zkevm.html) for more
details), some of the Hive tests need to be skipped or slightly modified in
order to pass.

For the
[hive rpc tests](https://github.com/kkrt-labs/hive/tree/master/simulators/ethereum/rpc),
all the websockets related tests are skipped as websockets aren't currently
supported by the Kakarot RPC.

For the
[hive rpc compatibility tests](https://github.com/kkrt-labs/hive/tree/master/simulators/ethereum/rpc-compat),
the following tests are skipped:

- debug_getRawBlock/get-block-n: the Kakarot implementation currently doesn't
  compute the block hash following EVM standards.
- debug_getRawBlock/get-genesis: see `debug_getRawBlock/get-block-n`.
- debug_getRawHeader/get-block-n: debug API is currently not supported by the
  Kakarot RPC.
- debug_getRawHeader/get-genesis: debug API is currently not supported by the
  Kakarot RPC.
- debug_getRawHeader/get-invalid-number: debug API is currently not supported by
  the Kakarot RPC.
- debug_getRawTransaction/get-invalid-hash: the Kakarot implementation of the
  debug_getRawTransaction endpoint uses `alloy_primitives::B256` type when
  deserializing the hash. This test is expected to fail as the provided hash in
  the query doesn't start with `0x`. As this test doesn't bring much, we decide
  to skip it.
- eth_createAccessList/create-al-multiple-reads: the createAccessList endpoint
  is currently not supported by the Kakarot RPC.
- eth_createAccessList/create-al-simple-contract: the createAccessList endpoint
  is currently not supported by the Kakarot RPC.
- eth_createAccessList/create-al-simple-transfer: the createAccessList endpoint
  is currently not supported by the Kakarot RPC.
- eth_feeHistory/fee-history: the Kakarot implementation doesn't currently set
  the block gas limit dynamically, which causes some disparity in the returned
  data. Additionally, the rewards of the blocks aren't available.
- eth_getBalance/get-balance-blockhash: see `debug_getRawBlock/get-block-n`.
- eth_getBlockByHash/get-block-by-hash: see `debug_getRawBlock/get-block-n`.
- eth_getBlockReceipts/get-block-receipts-by-hash: see
  `debug_getRawBlock/get-block-n`.
- eth_getBlockTransactionCountByHash/get-block-n: see
  `debug_getRawBlock/get-block-n`.
- eth_getBlockTransactionCountByHash/get-genesis: see
  `debug_getRawBlock/get-block-n`.
- eth_getProof/get-account-proof-blockhash: the getProof endpoint is currently
  not supported by the Kakarot RPC.
- eth_getProof/get-account-proof-with-storage: the getProof endpoint is
  currently not supported by the Kakarot RPC.
- eth_getProof/get-account-proof: the getProof endpoint is currently not
  supported by the Kakarot RPC.
- eth_getStorage/get-storage-invalid-key-too-large: the Kakarot implementation
  of the eth_getStorage endpoint uses `alloy_primitives::U256` type when
  deserializing the number. This test is expected to fail as the provided block
  number in the query doesn't start with exceeds 32 bytes. As this test doesn't
  bring much, we decide to skip it.
- eth_getStorage/get-storage-invalid-key: the Kakarot implementation uses the
  `jsonrpsee` crate's macro `rpc` in order to generate the server implementation
  of the ETH API. This test passes an invalid block hash `0xasdf` and expects
  the server to return with an error code `-32000` which corresponds to an
  invalid input error. The code derived from the `rpc` macro returns an error
  code of `-32602` which corresponds to an invalid parameters error, whenever it
  encounters issues when deserializing the input. We decide to ignore this test
  as the only issue is the error code returned.
- eth_getTransactionByBlockHashAndIndex/get-block-n: see
  `debug_getRawBlock/get-block-n`.

In addition to the tests we skip, some of the objects fields need to be ignored
in the passing tests:

- For blocks: the hash, parent hash, timestamp, base fee per gas, difficulty,
  gas limit, miner, size, state root, total difficulty and withdrawals are all
  skipped. Due to the difference between a type 1 and a type 2 ZK-EVM, these
  fields are currently not computed according to the EVM specifications and need
  to be skipped.
- For receipts, transactions and logs: the block hash is skipped.

If you which to run our hive test suite locally, the following steps should be
taken:

- Set up the repo: `make setup`.
- Build a local docker image of the RPC. Check the hive
  [Dockerfile](docker/hive/Dockerfile) for the values for `xxx` and `yyy`:

  ```shell
  docker build --build-arg APIBARA_STARKNET_BIN_DIR=xxx --build-arg APIBARA_SINK_BIN_DIR=yyy  -t hive . -f docker/hive/Dockerfile
  ```

- Checkout the Kakarot fork of hive:
  `git clone https://github.com/kkrt-labs/hive`
- Build the hive binary: `go build hive.go`
- Run the full rpc test suite against Kakarot:
  `./hive --sim "ethereum/rpc" --client kakarot`
- Additional filtering can be provided using `--sim.limit` if you which to run a
  certain limited set of tests.

## Project assistance

If you want to say **thank you** or/and support active development of Kakarot
RPC:

- Add a [GitHub Star](https://github.com/kkrt-labs/kakarot-rpc) to the project.
- Tweet about the Kakarot RPC: <https://x.com/KakarotZkEvm>.

## Contributing

First off, thanks for taking the time to contribute! Contributions are what make
the open-source community such an amazing place to learn, inspire, and create.
Any contributions you make will benefit everybody else and are **greatly
appreciated**.

Please read [our contribution guidelines](docs/CONTRIBUTING.md), and thank you
for being involved!

## Glossary

- StarknetOS chain: also called CairoVM chain, or Starknet appchain, it is a
  full-node (or sequencer) that is powered by the Cairo VM (Cairo smart
  contracts can be deployed to it). It a chain that behaves in most ways
  similarly to Starknet L2.
- Kakarot Core EVM: The set of Cairo Programs that implement the Ethereum
  Virtual Machine instruction set.
- Katana: A StarknetOS sequencer developed by the Dojo team. Serves as the
  underlying StarknetOS client for Kakarot zkEVM locally. It is built with speed
  and minimalism in mind.
- Madara: A StarknetOS sequencer and full-node developed by the Madara (e.g.
  Pragma Oracle, Deoxys, etc.) and Starkware exploration teams. Based on the
  Substrate framework, it is built with decentralization and robustness in mind.
- Kakarot zkEVM: the entire system that forms the Kakarot zkRollup: the core EVM
  Cairo Programs and the StarknetOS chain they are deployed to, the RPC layer
  (this repository), and the Kakarot Indexer (the backend service that ingests
  Starknet data types and formats them in EVM format for RPC read requests).

## Authors & contributors

For a full list of all authors and contributors, see
[the contributors page](https://github.com/sayajin-labs/kakarot-rpc/contributors).

## Security

Kakarot RPC follows good practices of security, but 100% security cannot be
assured. Kakarot RPC is provided **"as is"** without any **warranty**. Use at
your own risk.

_For more information and to report security issues, please refer to our
[security documentation](docs/SECURITY.md)._

## License

This project is licensed under the **MIT license**.

See [LICENSE](LICENSE) for more information.

## Acknowledgements

We warmly thank all the people who made this project possible.

- [Reth](https://github.com/paradigmxyz/reth) (Rust Ethereum), Thank you for
  providing open source libraries for us to reuse.
- [jsonrpsee](https://github.com/paritytech/jsonrpsee)
- Starkware and its exploration team, thank you for helping and providing a
  great test environment with Madara.
- [Lambdaclass](https://github.com/lambdaclass)
- [Dojo](https://github.com/dojoengine/dojo), thank you for providing great test
  utils.
- [starknet-rs](https://github.com/xJonathanLEI/starknet-rs), thank you for a
  great SDK.
- All our contributors. This journey wouldn't be possible without you.

## Benchmarks

For now, Kakarot RPC provides a minimal benchmarking methodology. You'll need
[Bun](https://bun.sh/) installed locally.

- Run a Starknet node locally (Katana or Madara), e.g.
  `katana --block-time 6000 --disable-fee` if you have the dojo binary locally,
  or `make madara-rpc-up` for Madara.
- Deploy the Kakarot smart contract (`make deploy-kakarot`)
- Run the Kakarot RPC binary (`make run-dev`)
- Run `make benchmark-katana` or `make benchmark-madara`

## Contributors ✨

Thanks goes to these wonderful people
([emoji key](https://allcontributors.org/docs/en/emoji-key)):

<!-- ALL-CONTRIBUTORS-LIST:START - Do not remove or modify this section -->
<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->
<table>
  <tbody>
    <tr>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/AbdelStark"><img src="https://avatars.githubusercontent.com/u/45264458?v=4?s=100" width="100px;" alt="Abdel @ StarkWare "/><br /><sub><b>Abdel @ StarkWare </b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=AbdelStark" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://www.silika.studio/"><img src="https://avatars.githubusercontent.com/u/112415316?v=4?s=100" width="100px;" alt="etash"/><br /><sub><b>etash</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=etashhh" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/0xMentorNotAPseudo"><img src="https://avatars.githubusercontent.com/u/4404287?v=4?s=100" width="100px;" alt="Mentor Reka"/><br /><sub><b>Mentor Reka</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=0xMentorNotAPseudo" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://bezier.fi/"><img src="https://avatars.githubusercontent.com/u/66029824?v=4?s=100" width="100px;" alt="Flydexo"/><br /><sub><b>Flydexo</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=Flydexo" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/Eikix"><img src="https://avatars.githubusercontent.com/u/66871571?v=4?s=100" width="100px;" alt="Elias Tazartes"/><br /><sub><b>Elias Tazartes</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=Eikix" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/greged93"><img src="https://avatars.githubusercontent.com/u/82421016?v=4?s=100" width="100px;" alt="greged93"/><br /><sub><b>greged93</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=greged93" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/bajpai244"><img src="https://avatars.githubusercontent.com/u/41180869?v=4?s=100" width="100px;" alt="Harsh Bajpai"/><br /><sub><b>Harsh Bajpai</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=bajpai244" title="Code">💻</a></td>
    </tr>
    <tr>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/ftupas"><img src="https://avatars.githubusercontent.com/u/35031356?v=4?s=100" width="100px;" alt="ftupas"/><br /><sub><b>ftupas</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=ftupas" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://www.linkedin.com/in/clementwalter/"><img src="https://avatars.githubusercontent.com/u/18620296?v=4?s=100" width="100px;" alt="Clément Walter"/><br /><sub><b>Clément Walter</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=ClementWalter" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/jobez"><img src="https://avatars.githubusercontent.com/u/615197?v=4?s=100" width="100px;" alt="johann bestowrous"/><br /><sub><b>johann bestowrous</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=jobez" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/danilowhk"><img src="https://avatars.githubusercontent.com/u/12735159?v=4?s=100" width="100px;" alt="danilowhk"/><br /><sub><b>danilowhk</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=danilowhk" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/irisdv"><img src="https://avatars.githubusercontent.com/u/8224462?v=4?s=100" width="100px;" alt="Iris"/><br /><sub><b>Iris</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=irisdv" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://aniketpr01.github.io/"><img src="https://avatars.githubusercontent.com/u/46114123?v=4?s=100" width="100px;" alt="Aniket Prajapati"/><br /><sub><b>Aniket Prajapati</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=aniketpr01" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/tekkac"><img src="https://avatars.githubusercontent.com/u/98529704?v=4?s=100" width="100px;" alt="Trunks @ Carbonable"/><br /><sub><b>Trunks @ Carbonable</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=tekkac" title="Code">💻</a></td>
    </tr>
    <tr>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/alex-sumner"><img src="https://avatars.githubusercontent.com/u/46249612?v=4?s=100" width="100px;" alt="Alex Sumner"/><br /><sub><b>Alex Sumner</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=alex-sumner" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/raphdeknop"><img src="https://avatars.githubusercontent.com/u/49572419?v=4?s=100" width="100px;" alt="Raphaël Deknop"/><br /><sub><b>Raphaël Deknop</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=raphdeknop" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/bhavyagosai"><img src="https://avatars.githubusercontent.com/u/64588227?v=4?s=100" width="100px;" alt="Bhavya Gosai"/><br /><sub><b>Bhavya Gosai</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=bhavyagosai" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/apoorvsadana"><img src="https://avatars.githubusercontent.com/u/95699312?v=4?s=100" width="100px;" alt="apoorvsadana"/><br /><sub><b>apoorvsadana</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=apoorvsadana" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://www.linkedin.com/in/paul-henrykajfasz/"><img src="https://avatars.githubusercontent.com/u/42912740?v=4?s=100" width="100px;" alt="Paul-Henry Kajfasz"/><br /><sub><b>Paul-Henry Kajfasz</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=phklive" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/khaeljy"><img src="https://avatars.githubusercontent.com/u/1810456?v=4?s=100" width="100px;" alt="Khaeljy"/><br /><sub><b>Khaeljy</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=khaeljy" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://nodeguardians.io/character/98995858fd55"><img src="https://avatars.githubusercontent.com/u/122918260?v=4?s=100" width="100px;" alt="Tristan"/><br /><sub><b>Tristan</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=TAdev0" title="Code">💻</a></td>
    </tr>
    <tr>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/glihm"><img src="https://avatars.githubusercontent.com/u/7962849?v=4?s=100" width="100px;" alt="glihm"/><br /><sub><b>glihm</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=glihm" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/omahs"><img src="https://avatars.githubusercontent.com/u/73983677?v=4?s=100" width="100px;" alt="omahs"/><br /><sub><b>omahs</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=omahs" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/MartianGreed"><img src="https://avatars.githubusercontent.com/u/11038484?v=4?s=100" width="100px;" alt="valdo.carbonaboyz.stark"/><br /><sub><b>valdo.carbonaboyz.stark</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=MartianGreed" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/dpinones"><img src="https://avatars.githubusercontent.com/u/30808181?v=4?s=100" width="100px;" alt="Damián Piñones"/><br /><sub><b>Damián Piñones</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=dpinones" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/zarboq"><img src="https://avatars.githubusercontent.com/u/37303126?v=4?s=100" width="100px;" alt="zarboq"/><br /><sub><b>zarboq</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=zarboq" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/dubzn"><img src="https://avatars.githubusercontent.com/u/58611754?v=4?s=100" width="100px;" alt="Santiago Galván (Dub)"/><br /><sub><b>Santiago Galván (Dub)</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=dubzn" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://droak.sh/"><img src="https://avatars.githubusercontent.com/u/5263301?v=4?s=100" width="100px;" alt="Oak"/><br /><sub><b>Oak</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=d-roak" title="Code">💻</a></td>
    </tr>
  </tbody>
  <tfoot>
    <tr>
      <td align="center" size="13px" colspan="7">
        <img src="https://raw.githubusercontent.com/all-contributors/all-contributors-cli/1b8533af435da9854653492b1327a23a4dbd0a10/assets/logo-small.svg">
          <a href="https://all-contributors.js.org/docs/en/bot/usage">Add your contributions</a>
        </img>
      </td>
    </tr>
  </tfoot>
</table>

<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->

<!-- ALL-CONTRIBUTORS-LIST:END -->

This project follows the
[all-contributors](https://github.com/all-contributors/all-contributors)
specification. Contributions of any kind welcome!
