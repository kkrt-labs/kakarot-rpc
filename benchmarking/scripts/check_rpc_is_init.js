const { waitForRPCInit } = require('./utils');

const main = async () => {
  console.log('checking RPC is up and running ...');
  await waitForRPCInit(100);
};

main();
