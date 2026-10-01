# Phase 0: machine snapshot (2026-09-30, main PC)

Measured in this session unless marked otherwise.

## Drives

| Letter | Device | Bus | Size | Free | Role |
|---|---|---|---|---|---|
| C: | Samsung SSD 990 PRO 2 TB | NVMe | 1,862 GB | ~200 GB (after freeing 20 GB of old Rust build output) | System; code repo, model files, eval directory, synthetic benchmark data |
| D: | WD_BLACK SN770 1 TB | NVMe | 931 GB | 61 GB | Owner's games and content drive (has its own librarian rules in `D:\CLAUDE.md`). **Not the vault.** Also holds the gathering output (`D:\Corpus-pilot2`, `D:\oaf-v-*`). |
| E: | WD Elements 25A3 (USB HDD) | USB | 6,001 GB | 560 GB | **Stand-in vault until the SSD arrives:** `E:\Stand-In Vault` (empty, NTFS) |

- The 1 TB vault SSD is not attached yet. The owner designated `E:\Stand-In Vault` as the stand-in until it arrives.
- BitLocker: the shell property `System.Volume.BitLockerProtection` reads 0 (not protected) on C:, D: and E:. This PC
  runs Windows 11 Home, which has no BitLocker management for data drives.
- Stand-in ACL: owner `SETHPC\sethy`, with inherited ACEs: Administrators and SYSTEM full control, Authenticated
  Users modify, Users read and execute. Because these ACEs name well-known groups, they also grant access on
  another machine. The owner SID itself will be unknown there.

### I/O of the stand-in (tools/iobench, unbuffered, 4 GiB file, 10 s per random test)

| Test | Result |
|---|---|
| Sequential write, 1 MiB | 118.9 MB/s |
| Sequential read, 1 MiB | 124.9 MB/s |
| Random 4 KiB read, 1 reader | 70 IOPS, p50 14.2 ms, p99 25.0 ms |
| Random 4 KiB read, 32 readers | 69 IOPS, p50 14.9 ms, p99 10.2 s (no queueing benefit through the USB bridge) |
| 16 KiB file create + flush, 2,000 files | 1,057 files/s (the drive's write cache absorbs the flushes) |

Conclusion: fine for staging files for a day. Unsuitable for any index: tantivy and vector search need
thousands of random reads per second. No vault data is written there before the SSD arrives.

## GPU

- RTX 5090, 32,607 MiB total, **29,771 MiB in use at 1% utilisation** (driver 617.14). The holder is not visible
  from Windows. It is presumably the owner's engine (Cinference) in WSL2 with the tutor model loaded. About
  2.8 GB is free.
- The owner will free the GPU for the 4B and 8B model benchmarks. Nothing in this project stops Cinference or
  Salad.

## Memory and CPU

- 63.4 GB RAM, about 24 GB free. The WSL2 VM is capped at 35 GB in `.wslconfig`, with
  `autoMemoryReclaim=disabled`; WSL networking is the default (NAT).
- Ryzen 7 9800X3D (8 cores, 16 threads).

## Tools

- cargo and rustc 1.98.1 (`C:\Users\sethy\.cargo\bin`, not on the PowerShell PATH), Python 3.14.6, uv, git,
  gh (authenticated), Temurin JDK 21 (scoop).
- protoc: `C:\Users\sethy\Projects\tutor\crates\rag-engine\vendor\protoc\bin\protoc.exe`.
- **Absent:** Docker, CMake, nvcc, llama.cpp. GROBID has no native Windows support, so it needs Docker
  Desktop or a WSL distro (owner decision; see the proposal).
- WSL distros: `Ubuntu` (holds Cinference), `salad-enterprise-linux`, `gpu-host-paralon-lab`. These belong to
  other owners' systems; this project does not touch them.

## Other processes seen

- A Java process with a 6.8 GB working set (not ours), Ollama (idle), several Python processes from other
  projects.
