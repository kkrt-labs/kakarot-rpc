const { providers, Wallet } = require('ethers');
const { writeFileSync } = require('fs');
const { sleep } = require('./utils');

const main = async () => {
  const provider = new providers.JsonRpcProvider('http://127.0.0.1:3030');
  const private_key =
    '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80';

  const eoa = new Wallet(private_key, provider);

  // wait 30 seconds for transaction pool to be filled with transactions
  await sleep(30 * 1000);

  const prevNonce = await eoa.getTransactionCount();
  const prevBlock = await provider.getBlockNumber();

  console.log('prev nonce', prevNonce);
  console.log('prev block', prevBlock);

  // wait 60 seconds for transactions to be be processed
  await sleep(60 * 1000);

  const blockTime = 6;
  const currNonce = await eoa.getTransactionCount();
  const currBlock = await provider.getBlockNumber();
  const blockDiff = currBlock - prevBlock;
  const nonceDiff = currNonce - prevNonce;
  const tpsPerBlock = nonceDiff / blockDiff;
  const tpsPerSec = tpsPerBlock / blockTime;

  console.log('current nonce', currNonce);
  console.log('current block', currBlock);

  console.log('total blocks processed', blockDiff);
  console.log('total transactions done', nonceDiff);
  console.log('transactions per starknet block', tpsPerBlock);
  console.log('transactions per second', tpsPerSec);

  console.log('saving data to report.json');
  writeFileSync(
    './reports/metrics.json',
    JSON.stringify([
      {
        name: 'transactions per block',
        value: tpsPerBlock,
        value: 'transactions/block',
      },
      {
        name: 'transactions per second',
        value: tpsPerSec,
        unit: 'transactions/second',
      },
    ])
  );
};

main();
