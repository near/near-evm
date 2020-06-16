pragma solidity ^0.5.8;

contract TestPrecompiles {
    function testSha2() external pure returns (bytes32) {
        bytes32 digest = hex"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        sha256(abi.encodePacked('AAA'));
        return digest;
    }

}
