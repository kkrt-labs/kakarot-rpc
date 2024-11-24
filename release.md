# Introduction

This file outlines the current procedure for the release of the Kakarot-RPC. This procedure is subject to updates 
and should be maintained in order to reflect the latest necessary steps to release.

# Procedure

## Staging

### Deploy updated contracts

- Run the `e2e pipeline` CI with the `deploy` argument, targeting staging. This will update the contracts on the 
  staging environment and finish with a pull request on the `Kakarot` repository with the updated deployment values. 
- Merge the PR on the `Kakarot` repository.

### Prepare release

- Open a PR on the RPC repository titled "Prepare RPC alpha release". This PR should: update Karnot's 
  deployment-config.yaml file and update the `Kakarot` submodule dependency to point to the commit which contains the 
  updated deployments file.
- Prepare the release of the RPC by making an alpha release on the RPC repository. This will trigger the CI which 
  will build the RPC and indexer images.

### Deploy the release and test

- Run the `karnot-deployment` CI, targeting staging. This will update the staging environment with the correct 
  images and env.
- Run the `e2e pipeline` CI with the `test` argument, targeting staging.
- If all the above works, you can move on to the next step, which repeats the previous actions but on the production 
  environment.

## Production

### Deploy updated contracts

- Run the `e2e pipeline` CI with the `deploy` argument, targeting production. This will update the contracts on the
  staging environment and finish with a pull request on the `Kakarot` repository with the updated deployment values.
- Merge the PR on the `Kakarot` repository.

### Release

- Open a PR on the RPC repository titled "Prepare RPC release". This PR should: update Karnot's
  deployment-config.yaml file, update the `Kakarot` submodule dependency to point to the commit which contains the
  updated deployments file, update the various docker files to points to the future release version, update the 
  manifest's (Cargo.toml) version.
- Release on the RPC repository. This will trigger the CI which will build the RPC and indexer images.

### Deploy the release

- Run the `karnot-deployment` CI, targeting production. This will update the production environment with the correct
  images and env.
- OPTIONAL: run the `e2e pipeline` CI with the `test` argument, targeting production.