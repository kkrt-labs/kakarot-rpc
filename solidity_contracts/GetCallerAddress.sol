// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract GetCallerAddress {
    // This function will return the address of the caller
    function getCallerAddress() public view returns (address) {
        return msg.sender;
    }
}
