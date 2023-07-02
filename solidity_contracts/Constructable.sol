// SPDX-License-Identifier: MIT
pragma solidity >=0.8.0;

contract Constructable {
    uint256 public count;
    address public zeroAddress;

    constructor(uint256 number, address _zeroAddress) {
        count = number;
        zeroAddress = _zeroAddress;
    }
}
