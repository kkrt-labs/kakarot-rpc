<div align="center">
  <h1>Kakarot RPC</h1>
  <img src="docs/images/logo.png" height="200">
  <br />
  <a href="https://github.com/sayajin-labs/kakarot-rpc/issues/new?assignees=&labels=bug&template=01_BUG_REPORT.md&title=bug%3A+">Report a Bug</a>
  -
  <a href="https://github.com/sayajin-labs/kakarot-rpc/issues/new?assignees=&labels=enhancement&template=02_FEATURE_REQUEST.md&title=feat%3A+">Request a Feature</a>
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

- [Report a Bug](#report-a-bug)
- [Request a Feature](#request-a-feature)
- [About](#about)
- [Architecture](#architecture)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Installation](#installation)
    - [kakarotup](#kakarotup)
    - [Environment variables](#environment-variables)
  - [Configuration](#configuration)
- [Usage](#usage)
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

> Kakarot RPC is a the Ethereum RPC adapter to interact with Kakarot ZK-EVM.

## Architecture

Here is a high level overview of the architecture of Kakarot RPC.

> TODO: Add architecture diagram

## Getting Started

TL;DR:
- Run `kakarotup` to install Kakarot RPC.
- Test with `cargo test --all`.
- Run Kakarot RPC in dev mode: `RUST_LOG=debug cargo run -p kakarot_rpc`

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)

### Installation

#### kakarotup

To install with `kakarotup` run (kakarotup requires nightly rustup):

```bash
curl -sL https://raw.githubusercontent.com/sayajin-labs/kakarot-rpc/main/kakarotup | sh
```

#### Environment variables

Copy the `.env.example` file to a `.env` file and populate each variable.

```bash
cp examples/.env.example .env
```

Meanwhile you can just use unit tests to dev.

```bash
cargo test --all
```

Build from source:

```bash
cargo build --all --release
```

The binaries will be located in `target/release/`. 

Specify the environment variables and run the binary.

```bash
source .env && cargo run -p kakarot_rpc
```
### Configuration

Kakarot RPC is configurable through environment variables.

Here is the list of all the available environment variables:

| Name             | Default value | Description      |
| ---------------- | ------------- | ---------------- |
| STARKNET_RPC_URL | No            | StarkNet RPC URL |

## Usage

### API

> **[TODO]**

## Roadmap

See the [open issues](https://github.com/sayajin-labs/kakarot-rpc/issues) for
a list of proposed features (and known issues).

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

If you want to say **thank you** or/and support active development of Kakarot RPC:

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

Kakarot RPC follows good practices of security, but 100% security cannot be assured.
Kakarot RPC is provided **"as is"** without any **warranty**. Use at your own risk.

_For more information and to report security issues, please refer to our
[security documentation](docs/SECURITY.md)._

## License

This project is licensed under the **MIT license**.

See [LICENSE](LICENSE) for more information.

## Acknowledgements
## Contributors ✨

Thanks goes to these wonderful people ([emoji key](https://allcontributors.org/docs/en/emoji-key)):

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

This project follows the [all-contributors](https://github.com/all-contributors/all-contributors) specification. Contributions of any kind welcome!