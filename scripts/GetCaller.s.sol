// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "kakarot/PlainOpcodes/GetCaller.sol";

contract GetCallerScript is Script {
    GetCaller public getCaller;

    function run() external {
        uint256 deployerPrivateKey = vm.envUint("EVM_PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        getCaller = new GetCaller();

        address sender_address = getCaller.getCallerAddress();

        console.logAddress(sender_address);
        require(
            sender_address ==
                address(0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266),
            "Address should be 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
        );
        vm.stopBroadcast();
    }
}
