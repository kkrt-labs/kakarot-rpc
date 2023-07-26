// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "kakarot/PlainOpcodes/GetCaller.sol";
import "kakarot/PlainOpcodes/PlainOpcodes.sol";
import "kakarot/PlainOpcodes/Counter.sol";

contract PlainOpcodesScript is Script {
    Counter public counter;
    PlainOpcodes public plainOpcodes;
    GetCaller public getCaller;

    function run() external {
        uint256 deployerPrivateKey = vm.envUint("EVM_PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        counter = new Counter();
        plainOpcodes = new PlainOpcodes(address(counter));
        getCaller = new GetCaller();

        counter.inc();
        plainOpcodes.opcodeCall();

        uint256 counter_value = plainOpcodes.opcodeStaticCall();
        address sender_address = getCaller.getCallerAddress();

        console.logAddress(sender_address);
        console.logUint(counter_value);
        require(counter_value == 2, "Counter should be 4");

        vm.stopBroadcast();
    }
}
