module.exports = {
  accounts: {
    amount: 10, // Number of unlocked accounts
    ether: 100, // Initial balance of unlocked accounts (in ether)
  },

  contracts: {
    type: 'web3',
    artifactsDir: '../src/tests/build/contracts', // Directory where contract artifacts are stored
  },

  node: { // Options passed directly to Ganache client
    gasLimit: 8e6, // Maximum gas per block
    gasPrice: 20e9 // Sets the default gas price for transactions if not otherwise specified.
  },
};
