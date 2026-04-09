// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title Gapura — chain-backed SSH authorized keys source of truth (PRD §5.A)
contract Gapura {
    address public owner;
    mapping(address => bool) public isAllowed;
    mapping(address => string[]) private _sshPublicKeys;

    event KeyGranted(address indexed wallet, string sshKey);
    event KeyRevoked(address indexed wallet);

    constructor() {
        owner = msg.sender;
    }

    modifier onlyOwner() {
        require(msg.sender == owner, "Only owner");
        _;
    }

    /// @notice Grant access and append one SSH public key for `wallet`.
    function grant(address wallet, string calldata sshKey) external onlyOwner {
        isAllowed[wallet] = true;
        _sshPublicKeys[wallet].push(sshKey);
        emit KeyGranted(wallet, sshKey);
    }

    /// @notice Revoke all keys for `wallet`.
    function revoke(address wallet) external onlyOwner {
        isAllowed[wallet] = false;
        delete _sshPublicKeys[wallet];
        emit KeyRevoked(wallet);
    }

    /// @notice Active keys for `wallet` when allowed; empty otherwise.
    function getActiveKeys(address wallet) external view returns (string[] memory) {
        if (!isAllowed[wallet]) {
            return new string[](0);
        }
        return _sshPublicKeys[wallet];
    }

    /// @dev Number of stored keys for `wallet` (for admin/UI without full copy).
    function keyCount(address wallet) external view returns (uint256) {
        return _sshPublicKeys[wallet].length;
    }
}
