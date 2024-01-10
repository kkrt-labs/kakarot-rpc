import { parseEther } from "ethers";
import { JsonRpcProvider, Wallet } from "ethers";

const rpcUrl = process.env.KAKAROT_RPC_URL;
const privateKey = process.env.EVM_PRIVATE_KEY;

// The delay between sending transactions in milliseconds.
// This is used to control the rate at which transactions are sent because
// underlying Starknet Clients cannot order nonces if transactions come in from the same sender too quickly.
// TODO Fix: send transactions from many different senders.
const interTransactionDelay = Number(process.env.INTER_TRANSACTION_MS_DELAY);

if (rpcUrl === undefined) {
  throw new Error(
    "KAKAROT_HTTP_RPC_URL is not defined in the environment variables",
  );
}

if (privateKey === undefined) {
  throw new Error(
    "EVM_PRIVATE_KEY is not defined in the environment variables",
  );
}

if (interTransactionDelay === undefined) {
  throw new Error(
    "INTER_TRANSACTION_MS_DELAY is not defined in the environment variables",
  );
}

const provider = new JsonRpcProvider(`http://${rpcUrl}`);
const wallet = new Wallet(privateKey, provider);
const recipient = Wallet.createRandom().address;

// 1 Gwei
const SEND_AMOUNT = parseEther("0.0000000001");

const originNonce = await wallet.getNonce();
let nonce = originNonce;

let isRunningFlag = true;
let startNonce: number | undefined = undefined;
let startBlockNumber: number | undefined = undefined;

let endNonce: number | undefined = undefined;
let endBlockNumber: number | undefined = undefined;

const startDelay = 10 * 1000;
const endDelay = 60 * 1000;

setTimeout(async () => {
  startNonce = await wallet.getNonce();
  startBlockNumber = await provider.getBlockNumber();
}, startDelay);

setTimeout(async () => {
  endNonce = await wallet.getNonce();
  endBlockNumber = await provider.getBlockNumber();
  isRunningFlag = false;
}, endDelay);

// Send enough transactions to reasonably fill the mempool/fifo queue.
while (isRunningFlag) {
  try {
    await wallet.sendTransaction({
      to: recipient,
      nonce,
      value: SEND_AMOUNT,
    });
  } catch (e) {
    // We expect to get an error:
    // @TODO: the returned hash did not match
    // <https://github.com/ethers-io/ethers.js/issues/4233>
    if (
      e instanceof Error &&
      !e.message.includes("the returned hash did not match")
    ) {
      // Handle the specific error case here
      throw new Error("Transaction failed with error: " + e.message);
    }
  }
  nonce += 1;
  await Bun.sleep(interTransactionDelay);
}

// Results
if (startNonce === undefined || endNonce === undefined) {
  throw new Error("startNonce is undefined");
}
console.log(`Start nonce: ${startNonce} 
Start block number: ${startBlockNumber}
End nonce: ${endNonce}
End block number: ${endBlockNumber}
Total transactions: ${endNonce - startNonce}
Inferred transactions per second: ${
  (endNonce - startNonce) / ((endDelay - startDelay) / 1000)
}
`);
