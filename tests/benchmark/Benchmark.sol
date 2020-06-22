pragma solidity ^0.5.8;

contract Benchmark {
    function benchecrecover() public pure {
        // signed with hex"2222222222222222222222222222222222222222222222222222222222222222"
        address addr = ecrecover(
            hex"1111111111111111111111111111111111111111111111111111111111111111",
            27,
            hex"b9f0bb08640d3c1c00761cdd0121209268f6fd3816bc98b9e6f3cc77bf82b698", // r
            hex"12ac7a61788a0fdc0e19180f14c945a8e1088a27d92a74dce81c0981fb644744"  // s
        );

        require(
            addr == 0x1563915e194D8CfBA1943570603F7606A3115508,
            "ecrecover mismatch"
        );
    }
}

contract ERC20
