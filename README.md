# EXAMINER

EXAMINER is our experimental modding toolkit for
[Exanima](https://store.steampowered.com/app/362490/Exanima/). It manages game
installations, profiles, mods, and load order. Our focus is direct, controllable
interaction and a workflow that can be automated further over time.

> [!WARNING]
> EXAMINER is under active development. Back up your save data and game files
> before experimenting with mods.

## Current capabilities

- Manage Exanima installations and profiles
- Enable, disable, and reorder mods
- Improved drag controls with a visible handle, continuous feedback, vertical
  movement constraints, and cancellation handling
- Asset and framework tools inherited from the Emtk foundation

## Building

Git and the Rust toolchain specified in `rust-toolchain.toml` are required.
Run this command from the project directory:

```powershell
cargo build
```

The launcher is the default Cargo workspace member. Our modified `iced_table`
component is stored reproducibly in this repository's `iced-table` branch and
is fetched automatically by Cargo.

## Project structure

- `emtk_launcher`: launcher and user interface
- `emtk_core`: installations, profiles, mods, and shared logic
- `emtk_framework`: runtime framework and plugin support
- `emtk_asset`: Exanima asset reading and writing
- `crates/detours`: native hooking dependency

## Public distribution policy

This repository contains tooling and original source changes only. Do not add
Exanima executables, game archives, extracted assets, save files, access keys,
or other proprietary Bare Mettle content. Users must own a legitimate copy of
Exanima and supply their own local game installation.

Community access or an invitation does not by itself grant intellectual
property rights. Obtain explicit written permission before distributing any
third-party assets or content whose licence is unclear.

## Origin and licence

EXAMINER is based on the open-source
[Exanima Modding Toolkit](https://codeberg.org/ExanimaModding/Toolkit). The
original authors and Git history remain credited. EXAMINER-specific changes are
developed in this repository.

The source code is available under MIT or Apache-2.0 as described by the
existing licence files. EXAMINER is an unofficial community project and is not
affiliated with or endorsed by Bare Mettle Entertainment, Exanima, or Sui
Generis.
