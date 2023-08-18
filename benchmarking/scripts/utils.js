const axios = require('axios');

const sleep = async ms => {
  return new Promise(resolve => setTimeout(resolve, ms));
};

// wait for Madara to be initialized, throw error if more than 5 retries
const waitForMadaraInit = async (maxRetries = 5) => {
  const rpcEndpoint = 'http://127.0.0.1:9944/health';

  for (let i = 0; i < maxRetries; i += 1) {
    try {
      const { data } = await axios.get(rpcEndpoint);

      // we succesfully recieved data from Madara RPC
      if (data) {
        console.log('madara is up and running succesfully ✅');
        return;
      }
    } catch (err) {
      // log error only when all retries have been attempted
      if (i == maxRetries - 1) {
        console.error(err);
      }

      // wait for 5 seconds
      await sleep(5 * 1000);
    }
  }

  // if we reach here then we have maxed out on retries
  throw new Error("couldn't connect with madara, max retries attempted");
};

// wait for Madara to be initialized, throw error if more than 5 retries
const waitForRPCInit = async (maxRetries = 5) => {
  const data = JSON.stringify({
    jsonrpc: '2.0',
    method: 'eth_chainId',
    params: [],
    id: '1',
  });

  const config = {
    method: 'post',
    maxBodyLength: Infinity,
    url: 'http://127.0.0.1:3030',
    headers: {
      'Content-Type': 'application/json',
    },
    data: data,
  };

  for (let i = 0; i < maxRetries; i += 1) {
    try {
      const { data } = await axios.request(config);

      // we succesfully recieved data from Madara RPC
      if (data) {
        console.log('rpc is up and running succesfully ✅');
        return;
      }
    } catch (err) {
      // log error only when all retries have been attempted
      if (i == maxRetries - 1) {
        console.error(err);
      }
      // wait for 5 seconds
      await sleep(5 * 1000);
    }
  }

  // if we reach here then we have maxed out on retries
  throw new Error("couldn't connect with rpc, max retries attempted");
};

module.exports = {
  sleep,
  waitForMadaraInit,
  waitForRPCInit,
};
