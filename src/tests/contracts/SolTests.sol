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

    event SomeEvent(uint256 _number);

    function () external payable {}

    function deployNewGuy(uint256 _aNumber) public payable returns (address, uint256) {
        SubContract _newGuy = new SubContract(_aNumber);
        address(_newGuy).transfer(msg.value);
        return (address(_newGuy), msg.value);
    }

    function payNewGuy(uint256 _aNumber) public payable returns (address, uint256) {
        SubContract _newGuy = (new SubContract).value(msg.value)(_aNumber);
        return (address(_newGuy), msg.value);
    }

    function returnSomeFunds() public payable returns (address, uint256) {
        address(msg.sender).transfer(msg.value / 2);
        return (msg.sender, msg.value);
    }

    function emitIt(uint256 _aNumber) public returns (bool) {
        emit SomeEvent(_aNumber);
        return true;
    }
}

contract SubContract is ExposesBalance {
    uint256 public aNumber = 6;

    constructor(uint256 _aNumber) public payable {
        aNumber = _aNumber;
    }

    function aFunction() public pure returns (bool) {
        return true;
    }

    function () external payable {}
}

contract Create2Factory {
    function deploy(bytes32 _salt, bytes memory _contractBytecode) public returns (address payable addr) {
        assembly {
            addr := create2(0, add(_contractBytecode, 0x20), mload(_contractBytecode), _salt)
        }
    }

    function doubleDeployTest(bytes32 _salt, bytes memory _contractBytecode) public returns (uint) {
        SelfDestruct other = SelfDestruct(deploy(_salt, _contractBytecode));
        other.storeUint(5);
        require(other.storedUint() == 7, "pre-destruction wrong uint");

        /* other.destruction(msg.sender); */
        /* require(address(other).balance == 0); */

        /* deploy(_salt, _contractBytecode); */
        /* require(other.storedUint() == 500, "post-redeploy wrong uint"); */
        return other.storedUint();
    }

    function () external payable {}
}

contract SelfDestruct {
    address public storedAddress;
    uint public storedUint;

    function () external payable {}

    function storeAddress() public {
        storedAddress = msg.sender;
    }

    function storeUint(uint _number) public {
        storedUint = _number;
    }

    function destruction(address payable addr) public returns (bool) {
        selfdestruct(addr);
    }
}
