pragma solidity ^0.5.8;

contract ExposesBalance {
  function balance() public view returns (uint256) {
    return address(this).balance;
  }

  // Unpermissioned. Don't deploy for real :D
  function transferTo(address _recipient, uint256 _amount) public returns (uint256) {
    address(uint160(_recipient)).transfer(_amount);
    return balance();
  }
}


contract SolTests is ExposesBalance {

    constructor() public payable {}

    function () external payable {}

    function deployNewGuy(uint256 _aNumber) public returns (address) {
        SubContract _newGuy = new SubContract(_aNumber);
        return address(_newGuy);
    }
}

contract SubContract is ExposesBalance {

    uint256 public aNumber = 6;

    constructor(uint256 _aNumber) public payable {
      aNumber = _aNumber;
    }

    function () external payable {}
}
