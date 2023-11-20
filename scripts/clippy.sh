#!/bin/bash

cargo clippy "$@" --workspace --all-features --all-targets \
    -- -D warnings -D future-incompatible -D nonstandard-style \
    -D rust-2018-idioms -D rust-2021-compatibility -D unused