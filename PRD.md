### 1. Filosofi & Visi
**ꦒꦥꦸꦫ** (Gapura) berarti “pintu masuk utama”.  
Dalam ekosistem Sangkan, Gapura adalah **otoritas akses mutlak** yang mendesentralisasikan manajemen SSH di bare-metal/server cluster.

**Visi:**  
Menggantikan Single Point of Failure (file `~/.ssh/authorized_keys` yang tersebar) dengan **Smart Contract sebagai Source of Truth** yang immutable, transparan, dan memberikan **instant global revoke**.

Sekali kamu revoke di blockchain, **semua server di cluster langsung menolak akses** tanpa perlu menyentuh tiap mesin.

### 2. Konteks Masalah
- Mengelola `authorized_keys` di homelab/cluster (contoh: 3–10 node Dell PowerEdge R630 dengan LACP) adalah mimpi buruk operasional.
- Key bocor/hilang → harus manual edit tiap node (rawan ghost access).
- Tidak ada audit trail yang transparan.
- Sinkronisasi antar node sering gagal atau tertinggal.

### 3. User Stories (Core)

**Sebagai Admin (Sang Begawan):**
- Saya bisa menambahkan wallet address + SSH public key via CLI sekali → langsung berlaku di seluruh cluster.
- Saya bisa merevoke akses secara instan dengan 1 transaksi.
- Saya bisa melihat history grant/revoke secara transparan di blockchain.

**Sebagai User:**
- Saya SSH seperti biasa (`ssh user@server`) tanpa merasa ada perbedaan.
- Kalau hak akses saya dicabut, saya langsung tidak bisa login ke server mana pun yang pakai Gapura.

### 4. Arsitektur Solusi Tinggi
- **Source of Truth**: Smart Contract di L2 EVM (Base Sepolia / Polygon Amoy).
- **Authentication Layer**: OpenSSH `AuthorizedKeysCommand` memanggil `gapura-sentinel` (Rust binary).
- **Management Tool**: `gapura` CLI (Rust) untuk admin.
- **Fallback**: Cache lokal + emergency static key (hanya untuk super-admin).

### 5. Spesifikasi Teknis Inti

#### A. The Ledger (Smart Contract – Solidity)
**Chain rekomendasi**: Base Sepolia (testnet) atau Base mainnet nanti (murah + cepat).

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract Gapura {
    address public owner;
    mapping(address => bool) public isAllowed;
    mapping(address => string[]) public sshPublicKeys;

    event KeyGranted(address indexed wallet, string sshKey);
    event KeyRevoked(address indexed wallet);

    constructor() {
        owner = msg.sender;
    }

    modifier onlyOwner() {
        require(msg.sender == owner, "Only owner");
        _;
    }

    function grant(address wallet, string calldata sshKey) external onlyOwner {
        isAllowed[wallet] = true;
        sshPublicKeys[wallet].push(sshKey);
        emit KeyGranted(wallet, sshKey);
    }

    function revoke(address wallet) external onlyOwner {
        isAllowed[wallet] = false;
        delete sshPublicKeys[wallet];
        emit KeyRevoked(wallet);
    }

    function getActiveKeys(address wallet) external view returns (string[] memory) {
        if (!isAllowed[wallet]) {
            return new string[](0);
        }
        return sshPublicKeys[wallet];
    }

    // Untuk future: bulk grant/revoke
}
```

#### B. The Sentinel (Rust Daemon – gapura-sentinel)
- Binary Rust super ringan.
- Dipanggil oleh OpenSSH via `AuthorizedKeysCommand`.
- Pakai **Alloy** (bukan ethers-rs yang sudah deprecated).
- Logic:
  1. Terima argument `%u` (username) dari sshd.
  2. Mapping username → wallet address (via config file sederhana `/etc/gapura/users.toml`).
  3. Query contract `getActiveKeys(wallet)`.
  4. Print key dalam format authorized_keys ke stdout.
  5. In-memory cache (TTL 30 detik) + background updater setiap 5 menit.
  6. Fallback: cache JSON terenkripsi di disk.

**Contoh sshd_config:**
```config
AuthorizedKeysFile none
AuthorizedKeysCommand /usr/local/bin/gapura-sentinel %u
AuthorizedKeysCommandUser nobody
```

#### C. The Commander (Admin CLI – gapura)
Command utama:
```bash
gapura init                  # setup wallet admin
gapura grant <wallet> "<ssh-ed25519 ...>" 
gapura revoke <wallet>
gapura status                # cek semua server sync
gapura audit                 # tampilkan history transaksi
```

### 6. Keamanan & Guardrails
- RPC: Hanya private endpoint (Alchemy/QuickNode) dengan API key.
- Rate limiting & cache: In-memory + disk fallback (anti brute-force & RPC downtime).
- Sentinel dijalankan sebagai `nobody` (least privilege).
- Emergency Door: 1–2 key statis di `/root/.ssh/authorized_keys.emergency` (hanya root, di-comment saat normal).
- Semua transaksi admin dicatat di blockchain (immutable audit trail).
- Input validation ketat di sentinel (hindari command injection).

### 7. Non-Functional Requirements
- Latency SSH login: < 800 ms (dengan cache).
- Availability: 99.9% (dengan fallback cache).
- Resource usage: Sentinel < 20 MB RAM, < 5 ms CPU per call.
- Scalability: Cocok untuk 3–50 node homelab/edge cluster.
- Bahasa: 100% Rust + Solidity.
- Lisensi: Open source (MIT).

### 8. Milestone Pengembangan (Roadmap) - Contoh saja

**Milestone 1** (1–2 minggu)  
- Deploy smart contract + test grant/revoke di Base Sepolia.  
- Setup Foundry project.

**Milestone 2** (2–3 minggu)  
- Buat `gapura-sentinel` dengan Alloy + cache.  
- Integrasi `users.toml` mapping.

**Milestone 3** (1 minggu)  
- Test end-to-end di 1 server lokal (VM).  
- Konfigurasi sshd_config + login test.

**Milestone 4** (2 minggu)  
- Bangun full `gapura` CLI.  
- Tambah `status` & `audit`.

**Milestone 5** (Opsional)  
- Deploy ke 3 node PowerEdge R630.  
- Tambah signature verification (wallet sign challenge) untuk zero-trust level 2.

### 9. Assumptions, Dependencies & Out of Scope
**Assumptions:**
- Semua server punya koneksi internet stabil (atau cache fallback).
- Admin punya wallet EVM (Rabby direkomendasikan).

**Dependencies:**
- Rust 1.80+
- Foundry
- OpenSSH ≥ 8.0 (support AuthorizedKeysCommand)
- Alloy crate

**Out of Scope (v1):**
- Full PAM module (pakai AuthorizedKeysCommand dulu).
- Account Abstraction / passkey login.
- Multi-signature admin.
- Windows OpenSSH (hanya Linux).