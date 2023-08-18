# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- ci: add `CHANGELOG.md` and enforce it is edited for each PR on `main`
- test: update integration tests to use prepopulated Katana dumped state
- ci: cross-compile binaries to improve build time
- dev: always pull image for latest tags when doing `docker-compose`
- ci: add timeout to all workflows
- feat: add Starknet transaction receipt wrapper for idiomatic conversion to Eth
  transaction receipt.
- fix: remove Madara as dependency.
- feat: add deployer account and deploy fee to Madara genesis
- chore: bump madara image
- dev: rename crate conformance-test-utils to hive-utils
- dev: update code and integration tests to return correct datatype for get_logs
- fix: update jsonrpsee error codes to EIP 1474 codes.
- ci: use madara binary for benchmark CI
