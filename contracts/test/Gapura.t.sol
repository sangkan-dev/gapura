// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {Gapura} from "../src/Gapura.sol";

contract GapuraTest is Test {
    Gapura internal gapura;
    address internal owner = address(this);
    address internal wallet = address(0xBEEF);
    address internal stranger = address(0xCAFE);

    string internal constant KEY_A = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGappa-test-a";
    string internal constant KEY_B = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGappa-test-b";

    function setUp() public {
        gapura = new Gapura();
    }

    function test_owner_is_deployer() public view {
        assertEq(gapura.owner(), owner);
    }

    function test_grant_sets_allowed_and_emits() public {
        vm.expectEmit(true, false, false, true);
        emit Gapura.KeyGranted(wallet, KEY_A);
        gapura.grant(wallet, KEY_A);
        assertTrue(gapura.isAllowed(wallet));
        string[] memory keys = gapura.getActiveKeys(wallet);
        assertEq(keys.length, 1);
        assertEq(keys[0], KEY_A);
    }

    function test_grant_twice_appends() public {
        gapura.grant(wallet, KEY_A);
        gapura.grant(wallet, KEY_B);
        string[] memory keys = gapura.getActiveKeys(wallet);
        assertEq(keys.length, 2);
        assertEq(keys[0], KEY_A);
        assertEq(keys[1], KEY_B);
        assertEq(gapura.keyCount(wallet), 2);
    }

    function test_getActiveKeys_empty_when_not_allowed() public view {
        string[] memory keys = gapura.getActiveKeys(wallet);
        assertEq(keys.length, 0);
    }

    function test_revoke_clears_and_blocks_getActiveKeys() public {
        gapura.grant(wallet, KEY_A);
        vm.expectEmit(true, false, false, false);
        emit Gapura.KeyRevoked(wallet);
        gapura.revoke(wallet);
        assertFalse(gapura.isAllowed(wallet));
        assertEq(gapura.getActiveKeys(wallet).length, 0);
        assertEq(gapura.keyCount(wallet), 0);
    }

    function test_only_owner_grant() public {
        vm.prank(stranger);
        vm.expectRevert(bytes("Only owner"));
        gapura.grant(wallet, KEY_A);
    }

    function test_only_owner_revoke() public {
        gapura.grant(wallet, KEY_A);
        vm.prank(stranger);
        vm.expectRevert(bytes("Only owner"));
        gapura.revoke(wallet);
    }

    function testFuzz_grant_revoke_keys_match(uint256 seed) public {
        address w = address(uint160(uint256(keccak256(abi.encode(seed)))));
        vm.assume(w != address(0));
        string memory k = string(abi.encodePacked("ssh-ed25519 ", vm.toString(seed)));
        gapura.grant(w, k);
        assertTrue(gapura.isAllowed(w));
        string[] memory keys = gapura.getActiveKeys(w);
        assertEq(keys.length, 1);
        assertEq(keys[0], k);
        gapura.revoke(w);
        assertEq(gapura.getActiveKeys(w).length, 0);
    }
}
