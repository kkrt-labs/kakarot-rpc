# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- feat: add automatic deployment of eoa from RPC if an account doesn't exist on chain
- ci: decrease the rate which we send transactions in benchmark CI
- fix: update the hive genesis utils for missing Kakarot requirements.
- ci: use madara binary for benchmark CI
- fix: update jsonrpsee error codes to EIP 1474 codes.
- dev: update code and integration tests to return correct datatype for get_logs
- dev: rename crate conformance-test-utils to hive-utils
- chore: bump madara image
- feat: add deployer account and deploy fee to Madara genesis
- fix: remove Madara as dependency.
- feat: add Starknet transaction receipt wrapper for idiomatic conversion to Eth
  transaction receipt.
- ci: add timeout to all workflows
- dev: always pull image for latest tags when doing `docker-compose`
- ci: cross-compile binaries to improve build time
- test: update integration tests to use prepopulated Katana dumped state
- ci: add `CHANGELOG.md` and enforce it is edited for each PR on `main`
