const NearProvider = require("near-web3-provider");

module.exports = {
  networks: {
    near: {
        network_id: "99",
        provider: function() {
            return new NearProvider("http://localhost:3030") //"https://rpc.nearprotocol.com")
        },
    }
  }
}
