# EXAMINER

EXAMINER is an open-source experimental mod project for
[Exanima](https://store.steampowered.com/app/362490/Exanima/). We use it to
explore different aspects of the game, prototype new interactions, and learn
what is possible within Exanima's systems.

This is not intended to replace the Exanima Modding Toolkit. EXAMINER uses
EMTK as its technical foundation for launching the game, loading plugins,
scanning signatures, and applying hooks. Improvements that are generally useful
to EMTK should be contributed upstream whenever possible.

> [!WARNING]
> EXAMINER is early experimental software. Features may be incomplete,
> unstable, or incompatible with future Exanima updates. Back up your saves and
> game files before testing anything.

## First experiment: object interaction

Our first gameplay experiment focuses on Exanima's in-game object dragging. We
want to investigate whether we can make it:

- stronger, including support for heavier physical objects
- more precise and controllable
- usable at a more practical range
- capable of grabbing a wider variety of world objects
- better at positioning and rotating objects
- stable when grabbing, releasing, or cancelling an interaction

The gameplay modification itself has not been implemented yet. The current
repository contains the working injection, framework, build, test foundation,
and in-game input telemetry required to identify and hook the relevant
functions in Exanima 0.9.5.

### Diagnostic controls

- `F2`: show or hide the EXAMINER diagnostic overlay
- `F6`: arm or disarm experiments; telemetry remains observe-only for now

The overlay reports mouse-button and modifier-key state plus a captured input
event counter. It does not modify gameplay or physics values yet.

## Future experiments

EXAMINER is deliberately broader than one feature. Possible experiments may
include physics, interaction, controls, quality-of-life changes, gameplay
systems, debugging tools, or other ideas that help us understand and extend the
game. Each experiment should be documented, optional, and testable on its own.

## Building the development foundation

Git, the Rust toolchain specified in `rust-toolchain.toml`, and the Microsoft
Visual Studio 2022 C++ Build Tools with a Windows SDK are required on Windows.

```powershell
git clone --recurse-submodules https://github.com/reirao/EXAMINER.git
cd EXAMINER
cargo build
```

If the repository was cloned without submodules, initialize Detours once:

```powershell
git submodule update --init --recursive
```

## Repository foundation

- `emtk_launcher`: starts Exanima and injects the framework
- `emtk_framework`: runtime, plugin loading, memory scanning, and hooks
- `emtk_core`: instances, profiles, and shared infrastructure
- `emtk_asset`: Exanima asset research and tooling
- `crates/detours`: native process injection and function detouring

These components originate from EMTK and remain visible so the experimental
work can be developed transparently. Gameplay experiments will be kept separate
from general framework changes.

## Public development policy

This repository contains tooling and original source changes only. Do not add
Exanima executables, game archives, extracted assets, save files, access keys,
or other proprietary Bare Mettle content. Testers must own Exanima and provide
their own local game installation.

## Origin and licence

EXAMINER is based on the open-source
[Exanima Modding Toolkit](https://codeberg.org/ExanimaModding/Toolkit). Its
original authors, licences, and Git history remain credited.

The source code is available under MIT or Apache-2.0 as described by the
existing licence files. EXAMINER is an unofficial community project and is not
affiliated with or endorsed by Bare Mettle Entertainment, Exanima, or Sui
Generis.
