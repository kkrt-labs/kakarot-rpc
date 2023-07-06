// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "kakarot/PlainOpcodes/Counter.sol";
import "kakarot/PlainOpcodes/PlainOpcodes.sol";

contract EvmsheetScript is Script {
    Counter public counter;
    PlainOpcodes public plainOpcodes;

    function run() external {
        uint256 deployerPrivateKey = vm.envUint("EVM_PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        counter = new Counter();
        plainOpcodes = new PlainOpcodes(address(counter));

        vm.stopBroadcast();
    }
}
