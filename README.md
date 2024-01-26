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

Kakarot RPC fits in the three-part architecture of the Kakarot zkEVM rollup ([Kakarot EVM Cairo Programs](https://github.com/kkrt-labs/kakarot), Kakarot RPC, [Kakarot Indexer](https://github.com/kkrt-labs/kakarot-indexer)). It is the implementation of the Ethereum JSON-RPC specification made to interact with Kakarot zkEVM in a fully Ethereum-compatible way.

![Kakarot zkEVM architecture](./docs/images/Kakarot%20zkEVM.png)

The Kakarot RPC layer's goal is to receive and output EVM-compatible
payloads & calls while interacting with an underlying StarknetOS client. This enables
Kakarot zkEVM to interact with the usual Ethereum tooling: Metamask, Hardhat,
Foundry, etc.

Note that this is necessary because Kakarot zkEVM is implemented as a set of Cairo Programs that run on an underlying CairoVM (so-called StarknetOS) chain.

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
  - Run dev RPC: `make run-dev` (you'll need a StarknetOS instance running in another process and Kakarot contracts deployed)
- Run with Docker Compose:
  - `make katana-rpc-up`
  - To kill these processes, `make docker-down`

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [Docker](https://docs.docker.com/engine/install)
- Make

## Installation

### Setup the project

To set up the repository (pulling git submodule and building Cairo dependencies), run:

```console
make setup
```

Caveats: the `setup` make command uses linux (MacOs compatible)
commands to allow running the `./scripts/extract_abi.sh`.
This script is used to use strongly typed Rust bindings for Cairo programs.
If you encounter problems when building the project, try running `./scripts/extract_abi.sh`

### Build from source

To build the project from source (in release mode):

```console
cargo build --release
```

Note that there are sometimes issues with some dependencies (notably scarb or cairo related packages, there are sometimes needs to `cargo clean` and `cargo build`)

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

To run a local StarknetOS client (Katana) and
deploy Kakarot zkEVM on it, i.e. the set of Cairo smart contracts implementing the EVM:

```console
make run-katana
```

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
  deployed**, so you don't have to do them manually (see in `./lib/kakarot/scripts/deploy_kakarot.py` for the list of contracts).

- the deployments and declarations for the devnet will be written to the
  `deployments/katana` folder inside your project root after a successful run of
  the `make deploy-kakarot` command.

### Running with [Docker Compose](https://docs.docker.com/compose/)

To orchestrate running a Katana/Madara devnet instance, deploy Kakarot contracts
and initialize the RPC, you may use the following commands:

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
Check out `.env.example` file to see the environment variables.

### API

You can take a look at `rpc-call-examples` directory. Please note the following:

- `sendRawTransaction.hurl`: the raw transaction provided allows to call the
  `inc()` function for the Counter contract. However, given that this
  transaction is signed for the EOA's nonce at the current devnet state (0x2),
  the call will only work once. If you want to keep incrementing (or
  decrementing) the counter, you need to regenerate the payload for the call
  with an updated nonce using the
  [provided python script](https://github.com/sayajin-labs/kakarot/blob/main/scripts/utils/kakarot.py#L273).

## Project assistance

If you want to say **thank you** or/and support active development of Kakarot
RPC:

- Add a [GitHub Star](https://github.com/kkrt-labs/kakarot-rpc) to the
  project.
- Tweet about the Kakarot RPC: https://twitter.com/KakarotZkEvm.

## Contributing

First off, thanks for taking the time to contribute! Contributions are what make
the open-source community such an amazing place to learn, inspire, and create.
Any contributions you make will benefit everybody else and are **greatly
appreciated**.

Please read [our contribution guidelines](docs/CONTRIBUTING.md), and thank you
for being involved!

## Glossary

- StarknetOS chain: also called CairoVM chain, or Starknet appchain, it is a full-node (or sequencer) that is powered by the Cairo VM (Cairo smart contracts can be deployed to it). It a chain that behaves in most ways similarly to Starknet L2.
- Kakarot Core EVM: The set of Cairo Programs that implement the Ethereum Virtual Machine instruction set.
- Katana: A StarknetOS sequencer developed by the Dojo team. Serves as the underlying StarknetOS client for Kakarot zkEVM locally. It is built with speed and minimalism in mind.
- Madara: A StarknetOS sequencer and full-node developed by the Madara (e.g. Pragma Oracle, Deoxys, etc.) and Starkware exploration teams. Based on the Substrate framework, it is built with decentralization and robustness in mind.
- Kakarot zkEVM: the entire system that forms the Kakarot zkRollup: the core EVM Cairo Programs and the StarknetOS chain they are deployed to, the RPC layer (this repository), and the Kakarot Indexer (the backend service that ingests Starknet data types and formats them in EVM format for RPC read requests).

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

- [Reth](https://github.com/paradigmxyz/reth) (Rust Ethereum),
  Thank you for providing open source libraries for us to reuse.
- [jsonrpsee](https://github.com/paritytech/jsonrpsee)
- Starkware and its exploration team,
  thank you for helping and providing a great test environment with Madara.
- [Lambdaclass](https://github.com/lambdaclass)
- [Dojo](https://github.com/dojoengine/dojo),
  thank you for providing great test utils.
- [starknet-rs](https://github.com/xJonathanLEI/starknet-rs),
  thank you for a great SDK.
- All our contributors. This journey wouldn't be possible without you.

## Benchmarks

For now, Kakarot RPC provides a minimal benchmarking methodology.
You'll need [Bun](https://bun.sh/) installed locally.

- Run a Starknet node locally (Katana or Madara),
  e.g. `katana --block-time 6000 --disable-fee` if you have the dojo binary locally,
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
