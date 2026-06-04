# vhdx-compactor

A self-contained Windows GUI tool that reclaims disk space from **WSL2** distros.
WSL2 stores each distro in an `ext4.vhdx` that **grows but never shrinks on its
own** — delete 30 GB inside your distro and Windows still shows the space as
used. This tool runs the full reclaim pipeline in one click.

## What it does

For each distro you select, in order:

1. **fstrim** — runs `fstrim -av` inside the running distro (as root) to discard
   freed ext4 blocks, zeroing them at the virtual-disk level. This is what makes
   the subsequent compaction actually reclaim space.
2. **`wsl --shutdown`** — stops the WSL2 VM so the `ext4.vhdx` files unlock.
3. **compact** — shrinks each `ext4.vhdx` via the Windows virtual-disk API
   (`CompactVirtualDisk`), falling back to `diskpart` if that fails.
4. **report** — shows before/after sizes and total bytes reclaimed.

> ⚠️ **`wsl --shutdown` is global.** It stops *every* WSL2 distro and Docker
> Desktop's WSL backend, not just the one you selected. The app shows an explicit
> confirmation before doing this. Save your work in all distros first.

### Note on sparse VHDX (Windows 11)

Recent WSL on Windows 11 creates **sparse** `ext4.vhdx` files by default. On a
sparse disk, the `fstrim` step's TRIM is passed through and the host reclaims the
freed blocks immediately — so `CompactVirtualDisk` finds little to do. The
explicit compaction step matters most for **non-sparse** disks (older WSL,
imported distros, or disks with `--set-sparse false`), where freed space is not
returned until you compact. The tool always runs the full pipeline; on sparse
disks the win mostly comes from the trim.

## Why a single binary

No PowerShell modules, no Hyper-V feature, no install. One `.exe`. Compaction
uses `virtdisk.dll` directly (works on Windows Home), with `diskpart` — which
ships on every Windows — as a fallback. The binary requests administrator rights
via an embedded manifest (one UAC prompt at launch), since compaction needs them.

## Architecture

Two-crate Rust workspace:

- **`vhdx-core`** — portable, OS-independent logic (domain types, `wsl -l -v`
  parsing, size formatting). Unit-tested on any host.
- **`vhdx-compactor`** — the `eframe`/`egui` GUI plus the `#[cfg(windows)]`
  syscall layer (`wsl.exe` orchestration, Lxss registry reads, virtdisk/diskpart
  compaction), and a worker-thread pipeline that streams progress events to the
  UI over a channel.

The pipeline depends on a `Platform` **trait**, not on the Windows modules
directly. The real `WindowsPlatform` is `#[cfg(windows)]`; tests use a fake. This
is what lets the reclaim *policy* (e.g. "fstrim failure is non-fatal, compact
failure aborts only that distro") be unit-tested on Linux even though the binary
only runs on Windows.

## Build

Requires [Nix](https://nixos.org) with flakes. The dev shell pins a Rust
toolchain with the `x86_64-pc-windows-gnu` cross target plus the mingw toolchain.

```bash
nix develop            # enter the dev shell
just                   # list recipes
just test              # host unit tests (vhdx-core + pipeline policy)
just lint              # clippy (host)
just lint-win          # clippy (Windows cross target — catches cfg(windows) issues)
just build-win         # release Windows binary
just ci                # fmt-check + lint + lint-win + test
```

The Windows binary is produced at:

```
target/x86_64-pc-windows-gnu/release/vhdx-compactor.exe
```

`just run` launches the GUI on the host (Linux/macOS) to preview the layout;
WSL operations there report "Windows only" since `wsl.exe`/virtdisk don't exist.

## Headless CLI

The Windows binary also runs without the GUI when given arguments — useful for
scripting and CI:

```
vhdx-compactor.exe list                 # discover distros: name, state, size, vhdx path
vhdx-compactor.exe compact <vhdx-path>  # compact one vhdx, print before/after + bytes saved
```

`list` is read-only (no elevation needed); `compact` performs the real
`CompactVirtualDisk` and needs administrator rights. The release build embeds an
admin manifest; debug builds omit it so the CLI can be exercised non-elevated.

## Verification status

- `just ci` (fmt + clippy host + clippy on the Windows target + tests) is green.
- **Discovery is verified live**: `vhdx-compactor.exe list` was run against a real
  Windows 11 + WSL2 host — it correctly drove `wsl.exe`, decoded the UTF‑16
  output, read the Lxss registry, and reported real distro sizes/paths.
- **Compaction runtime** (`CompactVirtualDisk`) is verified by compilation and
  clippy on the Windows target; confirm the actual reclaim with the checklist
  below on a Windows machine.

## Manual Windows test checklist

The deterministic gates (`just ci`) prove the code compiles and the portable
logic is correct, but actual reclaim can only be verified on Windows:

1. Copy `vhdx-compactor.exe` to a Windows machine with WSL2 installed.
2. Double-click → accept the UAC prompt.
3. Verify distros appear with correct names, states, and `ext4.vhdx` sizes.
4. Select a distro, click **Reclaim**, confirm the shutdown warning.
5. Watch: `trim …` log lines → "WSL shut down" → progress bar → "✓ saved N".
6. Verify the `ext4.vhdx` on disk shrank by roughly the reported amount.
7. Verify `wsl -l -v` still lists the distro and it boots normally afterward.

## License

MIT OR Apache-2.0.
