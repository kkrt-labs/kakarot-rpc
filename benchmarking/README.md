# Kakarot Benchmarking

The directory contains scripts which we use to benchmark Kakarot, as of now we
are using aritillery to create virtual users and test the RPC.

NOTE: If you have a `.env` file at the root of this repository, then it is
suggested to rename it to something else when you run these benchmarks, the
reason being that the RPC will always override variables from `.env`, which can
lead to benchmarks not working.

Steps to benchmark:

- install all submodules in the project root:
  - git submodule update --init --recursive
- install Kakarot dependencies and compile:
  - from the project root:
    - cd lib/kakarot
    - poetry install { creating a venv is advised }
    - STARKNET_NETWORK=madara poetry run python ./scripts/compile_kakarot.py
- build Madara:
  - from the project root:
    - cd lib/madara
    - cargo build --release
- build RPC:
  - from the project root:
    - cargo build --release
- install depdendencies:
  - from the `benchmarking` directory:
    - `npm i`
- run the benchmark:
  - from the `benchmarking` directory:
    - `npm run benchmark:ci`
- a report file will be dumped in `reports` directory, you can check the
  benchmarking result there.
