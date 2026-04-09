// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Script} from "forge-std/Script.sol";
import {Gapura} from "../src/Gapura.sol";

/// @notice Deploy Gapura to Base Sepolia (or any RPC via `--rpc-url`).
contract GapuraScript is Script {
    function run() public returns (Gapura deployed) {
        vm.startBroadcast();
        deployed = new Gapura();
        vm.stopBroadcast();
    }
}
