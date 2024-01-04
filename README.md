<div align="center">
  <h1>Kakarot RPC</h1>
  <img src="docs/images/logo.png" height="200">
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

[![GitHub Workflow Status](https://github.com/sayajin-labs/kakarot-rpc/actions/workflows/test.yml/badge.svg)](https://github.com/sayajin-labs/kakarot-rpc/actions/workflows/test.yml)
[![Project license](https://img.shields.io/github/license/sayajin-labs/kakarot-rpc.svg?style=flat-square)](LICENSE)
[![Pull Requests welcome](https://img.shields.io/badge/PRs-welcome-ff69b4.svg?style=flat-square)](https://github.com/sayajin-labs/kakarot-rpc/issues?q=is%3Aissue+is%3Aopen+label%3A%22help+wanted%22)

</div>

<details open="open">
<summary>Table of Contents</summary>

- [Report a Bug](https://github.com/sayajin-labs/kakarot-rpc/issues/new?assignees=&labels=bug&template=01_BUG_REPORT.md&title=bug%3A+")
- [Request a Feature](https://github.com/sayajin-labs/kakarot-rpc/issues/new?assignees=&labels=enhancement&template=02_FEATURE_REQUEST.md&title=feat%3A+≠≠≠≠≠≠≠)
- [About](#about)
- [Architecture](#architecture)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Installation](#installation)
    - [Build from source](#build-from-source)
    - [Environment variables](#environment-variables)
  - [Configuration](#configuration)
- [Installation](#installation)
  - [API](#api)
- [Roadmap](#roadmap)
- [Support](#support)
- [Project assistance](#project-assistance)
- [Contributing](#contributing)
- [Authors \& contributors](#authors--contributors)
- [Security](#security)
- [License](#license)
- [Acknowledgements](#acknowledgements)

</details>

---

## About

> Kakarot RPC is the JSON-RPC server adapter to interact with Kakarot ZK-EVM in
> a fully EVM-compatible way.

This adapter layer is based on:

- [The Ethereum JSON-RPC spec](https://github.com/ethereum/execution-apis/tree/main/src/eth)
- [The Starknet JSON-RPC spec](https://github.com/starkware-libs/starknet-specs/blob/master/api/starknet_api_openrpc.json)
- [And their differences](https://github.com/starkware-libs/starknet-specs/blob/master/starknet_vs_ethereum_node_apis.md)

The Kakarot RPC layer's goal is to receive and output EVM-compatible JSON-RPC
payloads & calls while interacting with the Starknet Blockchain. This enables
Kakarot zkEVM to interact with the usual Ethereum tooling: Metamask, Hardhat,
Foundry, etc.

## Architecture

Here is a high level overview of the architecture of Kakarot RPC.

![Kakarot RPC Adapter flow](https://user-images.githubusercontent.com/66871571/215438348-26ac2aee-bf30-4429-bbca-a7b901ac0594.png)

## Getting Started

TL;DR:

- Run `make build` to build Kakarot RPC.
- Test with `make test`.
- Run Kakarot RPC in dev mode:
  - Run devnet: `make devnet` ( or feel free to run your own )
  - Run dev RPC: `make run`
  - Run production RPC `make run-release`

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [Docker](https://docs.docker.com/engine/install)
- Make

## Installation

### Build from source

To build the project from source do `make build` (this requires
[nightly rustup](https://rust-lang.github.io/rustup/concepts/channels.html)):

```console
make build
```

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

Specify the environment variables and run the binary.

```console
make run-release
```

### Dev mode with [katana](https://github.com/dojoengine/dojo/tree/main/crates/katana)

run devnet

```console
make devnet
```

run

```console
make run
```

Some notes on `make devnet`:

- you can run a devnet, by running `make devnet` at the project root.

- this will run a devnet by running katana, **with contracts automatically
  deployed**, so you don't have to do them manually (see below for list of
  contracts and addresses).

- it will use the values from `.env.example` file for deployment by default, but
  you can override any variable that you want by passing it to docker { changing
  `.env.example` won't work as it was copied during build phase of the image },
  you can see the `devnet` target in the `Makefile` of the project, and see how
  we are overriding STARKNET_NETWORK environment variable, in similar fashion,
  you can override any other environment variable.

- the deployments and declarations for the devnet will be written to the
  `deployments/katana` folder inside your project root after a successful run of
  the `make devnet` command.

- feel free to run your own devnet if you are playing around with some custom
  changes to Kakarot.

### Running with [Docker Compose](https://docs.docker.com/compose/)

To orchestrate running a Katana/Madara devnet instance, deploy Kakarot contracts
and initialize the RPC, you may use the following commands:

**Note: Ensure that you have the `.env` file**

For Katana

```console
make katana-rpc-up
```

For Madara

```console
make madara-rpc-up
```

### Sending transactions to RPC using [forge script](https://book.getfoundry.sh/reference/forge/forge-script)

An example script to run which uses a pre-funded EOA account with private key
`EVM_PRIVATE_KEY`

```console
forge script scripts/PlainOpcodes.s.sol --broadcast --legacy --slow
```

### Configuration

Kakarot RPC is configurable through environment variables.

Here is the list of all the available environment variables:

<!-- markdownlint-disable MD013 -->

| Name                          | Default value             | Description                  |
| ----------------------------- | ------------------------- | ---------------------------- |
| TARGET_RPC_URL                | <http://0.0.0.0:5050/rpc> | Target Starknet RPC URL      |
| RUST_LOG                      | Debug                     | Log level                    |
| KAKAROT_HTTP_RPC_ADDRESS      | 0.0.0.0:3030              | Kakarot RPC URL              |
| KAKAROT_ADDRESS               | see below                 | Kakarot address              |
| PROXY_ACCOUNT_CLASS_HASH      | see below                 | Proxy account class hash     |
| DEPLOYER_ACCOUNT_ADDRESS      | N/A                       | Deployer Account Address     |
| DEPLOYER_ACCOUNT_PRIVATE_KEY  | see below                 | Deployer Account Private Key |

<!-- markdownlint-enable MD013 -->

### Devnet deployed/declared contracts

Deployed:

| Contract | Address                                                           |
| -------- | ----------------------------------------------------------------- |
| Kakarot  | 0x7a88f6f9d63ccaa5855babb32cbb0230b8588aaaa6bc4ce2d173fa528ce7567 |
| EOA      | 0x54b288676b749DEF5Fc10Eb17244fe2C87375de1                        |
| Counter  | 0x2e11Ed82f5eC165AB8Ce3cC094f025Fe7527F4D1                        |

Declared:

<!-- markdownlint-disable MD013 -->

| Contract                 | Class hash                                                        |
| ------------------------ | ----------------------------------------------------------------- |
| Proxy account class hash | 0xba8f3f34eb92f56498fdf14ecac1f19d507dcc6859fa6d85eb8545370654bd  |

<!-- markdownlint-enable MD013 -->

The Counter contract implementation can be found
[here](https://github.com/sayajin-labs/kakarot/blob/main/tests/integration/solidity_contracts/PlainOpcodes/Counter.sol)

### Deployer Account
The Kakarot RPC requires a funded deployer account to deploy ethereum EOAs whose on-chain smart contract don't exist, the role of
the deployer is to deploy these accounts for a smoother UX { the deployer recovers the amount spent of this deployments }

The kakarot [deploy scripts](https://github.com/kkrt-labs/kakarot/blob/9773e4d10a3c3a32fb8aa3cfbf6fdbff35d6985e/scripts/deploy_kakarot.py#L67) deploy and fund an account with the private key "0x0288a51c164874bb6a1ca7bd1cb71823c234a86d0f7b150d70fa8f06de645396" for [Katana](https://github.com/dojoengine/dojo/tree/main/crates/katana) and [Madara](https://github.com/keep-starknet-strange/madara), the address of this account can be found in the file `deployments/{network}/deployments.json` with the key `deployer_account` after running this script on [Kakarot](https://github.com/kkrt-labs/kakarot).

You can configure Kakarot RPC to run with a particular Deployer Account via the following environment variables:
- `DEPLOYER_ACCOUNT_ADDRESS`
- `DEPLOYER_ACCOUNT_PRIVATE_KEY`

When running in production on testnet and mainnet it is advised to have a separate pre-funded account for this.

### API

You can take a look at `rpc-call-examples` directory. Please note the following:

- `sendRawTransaction.hurl`: the raw transaction provided allows to call the
  `inc()` function for the Counter contract. However, given that this
  transaction is signed for the EOA's nonce at the current devnet state (0x2),
  the call will only work once. If you want to keep incrementing (or
  decrementing) the counter, you need to regenerate the payload for the call
  with an updated nonce using the
  [provided python script](https://github.com/sayajin-labs/kakarot/blob/main/scripts/utils/kakarot.py#L273).

## Roadmap

See the [open issues](https://github.com/sayajin-labs/kakarot-rpc/issues) for a
list of proposed features (and known issues).

- [Top Feature Requests](https://github.com/sayajin-labs/kakarot-rpc/issues?q=label%3Aenhancement+is%3Aopen+sort%3Areactions-%2B1-desc)
  (Add your votes using the 👍 reaction)
- [Top Bugs](https://github.com/sayajin-labs/kakarot-rpc/issues?q=is%3Aissue+is%3Aopen+label%3Abug+sort%3Areactions-%2B1-desc)
  (Add your votes using the 👍 reaction)
- [Newest Bugs](https://github.com/sayajin-labs/kakarot-rpc/issues?q=is%3Aopen+is%3Aissue+label%3Abug)

## Support

Reach out to the maintainer at one of the following places:

- [GitHub Discussions](https://github.com/sayajin-labs/kakarot-rpc/discussions)
- Contact options listed on
  [this GitHub profile](https://github.com/starknet-exploration)

## Project assistance

If you want to say **thank you** or/and support active development of Kakarot
RPC:

- Add a [GitHub Star](https://github.com/sayajin-labs/kakarot-rpc) to the
  project.
- Tweet about the Kakarot RPC.
- Write interesting articles about the project on [Dev.to](https://dev.to/),
  [Medium](https://medium.com/) or your personal blog.

Together, we can make Kakarot RPC **better**!

## Contributing

First off, thanks for taking the time to contribute! Contributions are what make
the open-source community such an amazing place to learn, inspire, and create.
Any contributions you make will benefit everybody else and are **greatly
appreciated**.

Please read [our contribution guidelines](docs/CONTRIBUTING.md), and thank you
for being involved!

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

## Contributors ✨

Thanks goes to these wonderful people
([emoji key](https://allcontributors.org/docs/en/emoji-key)):

<!-- ALL-CONTRIBUTORS-LIST:START - Do not remove or modify this section -->
<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->
<table>
  <tbody>
    <tr>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/abdelhamidbakhta"><img src="https://avatars.githubusercontent.com/u/45264458?v=4?s=100" width="100px;" alt="Abdel @ StarkWare "/><br /><sub><b>Abdel @ StarkWare </b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=abdelhamidbakhta" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://www.silika.studio/"><img src="https://avatars.githubusercontent.com/u/112415316?v=4?s=100" width="100px;" alt="etash"/><br /><sub><b>etash</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=etashhh" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/0xMentorNotAPseudo"><img src="https://avatars.githubusercontent.com/u/4404287?v=4?s=100" width="100px;" alt="Mentor Reka"/><br /><sub><b>Mentor Reka</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=0xMentorNotAPseudo" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://bezier.fi/"><img src="https://avatars.githubusercontent.com/u/66029824?v=4?s=100" width="100px;" alt="Flydexo"/><br /><sub><b>Flydexo</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=Flydexo" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/Eikix"><img src="https://avatars.githubusercontent.com/u/66871571?v=4?s=100" width="100px;" alt="Eikix - Elias Tazartes"/><br /><sub><b>Elias Tazartes</b></sub></a><br /><a href="https://github.com/sayajin-labs/kakarot-rpc/commits?author=Eikix" title="Code">💻</a></td>
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
