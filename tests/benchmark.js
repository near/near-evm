const { accounts, contract, web3 } = require('@openzeppelin/test-environment')
const ERC20 = contract.fromArtifact('ERC20')
const ZombieAttack = contract.fromArtifact('ZombieAttack')
const [acct1, acct2 ] = accounts


const benchmarkCryptoZombies = async () => {
    // deploy
    const deploy_gas = await ZombieAttack.deploy().estimateGas()
    const cryptozombies = await ZombieAttack.deploy().send({from: acct1})

    // create random zombies
    const create_gas = await await cryptozombies.methods.createRandomZombie("Bob").estimateGas()

    console.log(`ZombieAttack
        deploy:             ${deploy_gas}
        createRandomZombie  ${create_gas}
    `)
}

const benchmarkERC20 = async () => {
    // deploy
    const deploy_gas = await ERC20.deploy().estimateGas()
    const erc20 = await ERC20.deploy().send({from: acct1})

    // transfer
    const transfer_gas = await erc20.methods.transfer(acct2, 20).estimateGas({from: acct1})

    // approve
    const approve_gas = await erc20.methods.approve(acct2, 20).estimateGas({from: acct1})
    await erc20.methods.approve(acct2, 2000).send({from: acct1})

    // transferFrom
    const transfer_from_gas = await erc20.methods.transferFrom(acct1, acct2, 2).estimateGas({from: acct2})

    // increase allowance
    const increase_allowance_gas = await erc20.methods.increaseAllowance(acct2, 100).estimateGas({from: acct1})
    console.log(`ERC20
        deploy:             ${deploy_gas}
        transfer:           ${transfer_gas}
        approve:            ${approve_gas}
        transferFrom:       ${transfer_from_gas}
        increaseAllowance   ${increase_allowance_gas}

    `)
}

benchmarkCryptoZombies()
benchmarkERC20()
