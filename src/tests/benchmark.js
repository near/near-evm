const fs = require('fs');

const Web3 = require('web3');
const { NearProvider, nearlib, nearWeb3Extensions } = require('near-web3-provider');
const { Contract, KeyPair, connect } = nearlib;
const { InMemoryKeyStore, MergeKeyStore, UnencryptedFileSystemKeyStore } = nearlib.keyStores;
const { parseNearAmount } = nearlib.utils.format;
const { getTransactionLastResult } = nearlib.providers;

function gasBurnt(result) {
    let gas = result.transaction_outcome.outcome.gas_burnt;
    result.receipts_outcome.forEach((receipt) => {
        gas += receipt.outcome.gas_burnt;
    })
    return gas / (10 ** 12);
}

async function deployContract(evmContract, contractPath) {
    let bytecode = fs.readFileSync(contractPath).toString();
    let rawResult = await evmContract.account.functionCall(evmContract.contractId, 'deploy_code', { bytecode });
    console.log(`Deploy Contract: ${gasBurnt(rawResult)} Tgas`);
    return getTransactionLastResult(rawResult);
}

async function runBenchmark() {
    const contractConfig = {
        viewMethods: [],
        changeMethods: ['deploy_code'],
    }    
    const config = require('./config')(process.env.NEAR_ENV || 'local');
    const keyStore = new MergeKeyStore([
        new InMemoryKeyStore(),
        new UnencryptedFileSystemKeyStore('./neardev')
    ]);
    const near = await connect({ ...config, deps: { keyStore } });
    
    let account = await near.account(config.accountId);
    let evmContract = new Contract(account, config.evmContract, contractConfig);
    const contractId = await deployContract(evmContract, 'zombieAttack.bin');
    console.log(contractId);
    // const contractId = "9ecae05be9e959360223bc631f99f14e07758df5";

    const web = new Web3();
    web.extend(nearWeb3Extensions(web));
    web.setProvider(new NearProvider(config.nodeUrl, near.connection.signer.keyStore, config.accountId, config.networkId, config.evmContract));

    const contract = new web.eth.Contract(JSON.parse(fs.readFileSync('zombieAttack.abi')), contractId);
    let res = await contract.methods.createRandomZombie('blah').send({ from: web._provider.accountEvmAddress });
    console.log(`Create Random Zombie: ${res.gasUsed / 10**12} Tgas`);
}

runBenchmark().catch(console.error);
