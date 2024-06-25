# benchmarks

To install dependencies:

```bash
bun install
```

To run:

```bash
# to benchmark Kakarot using Madara, run:
bun run benchmark-madara

# to benchmark Kakarot using Katana, run:
bun run benchmark-katana
```

Note:
The benchmarks rely on a INTER_TRANSACTION_MS_DELAY environment variable.
It is aimed at spacing transactions between one another.
This achieves a two-fold goal:

- Refrain from filling the mempool too fast,
  i.e. reach maximum capacity of the mempool/backlog before the end of the benchmark.
- allow clients to order the transactions with regards to their nonce,
  as for now, only one wallet fires transactions, and nonces must be sequential.

This implies that one must calibrate a good value for INTER_TRANSACTION_MS_DELAY.
