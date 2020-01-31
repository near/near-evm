pragma solidity ^0.5.8;


contract SolTests {
  function deployNewGuy(uint256 _aNumber) public returns (address) {
      SubContract _newGuy = new SubContract(_aNumber);
      return address(_newGuy);
  }
}

contract SubContract{
    uint256 public aNumber = 6;
    constructor(uint256 _aNumber) public {
      aNumber = _aNumber;
    }
}
