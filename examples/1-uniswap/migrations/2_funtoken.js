/* globals artifacts */
const funtoken = artifacts.require('FunToken')

module.exports = function(deployer) {
    deployer.deploy(funtoken)
}
