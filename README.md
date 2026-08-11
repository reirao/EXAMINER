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

The gameplay modification itself has not been implemented yet. We are currently
building and validating the observation layer required to identify the real
selection and physics functions instead of guessing at offsets or presenting a
cosmetic change as a gameplay mod.

## What works today

The current development build provides:

- an EMTK-based launcher that starts Exanima and injects `emtk_framework`
- an in-game OpenGL3/ImGui diagnostic overlay
- safe observe-only and explicitly armed experiment states
- persistent framework logs under
  `%APPDATA%\exanima-modding-toolkit\log\emtk_framework.log.*`
- numbered drag sessions with `start`, `move`, and `end` phases
- cursor coordinates and per-event `dx`/`dy` movement
- cumulative pixel distance for every drag session
- debounced left/right mouse and `Ctrl`, `Shift`, and `Alt` state telemetry
- a working native Windows build using Rust, Detours, MSVC, and the Windows SDK

The telemetry is observational. It currently does **not** identify the selected
world object, read object mass, alter interaction range, or change forces.

### Diagnostic controls

- `F2`: show or hide the EXAMINER diagnostic overlay
- `F6`: arm or disarm experiments; telemetry remains observe-only for now

The overlay reports mouse-button and modifier-key state plus a captured input
event counter. The persistent log contains the more detailed drag-session data.
`F6` currently changes experiment state only; it does not modify gameplay or
physics values.

## Current research results

We have built the modified framework successfully and completed live Exanima
runs with it. Injection was confirmed by the `Main Hook Running` and
`Running Original Program Entrypoint` markers. A recent controlled session
recorded 37 drag attempts, including complete movement traces and modifier
transitions. This established that the sensor can distinguish stationary or
failed attempts (`0 px`) from real cursor-driven drags and can compare movement
distance across repeated interactions.

The run also exposed two limitations that we are keeping visible:

- input telemetry alone cannot tell us which Exanima object was under the
  cursor or whether the game accepted it as a physical interaction
- Hudhook's OpenGL3 renderer repeatedly reports `Insufficient display size` at
  `3440x1440` when the overlay is toggled, so overlay rendering needs a separate
  compatibility fix even though persistent telemetry continues to work

x64dbg and Ghidra have been prepared locally for controlled dynamic and static
analysis. We have confirmed that x64dbg can attach to the EMTK-launched Exanima
process. No permanent patch, guessed address, or game binary modification has
been committed.

## Planned work

The path from diagnostics to the first real mod is intentionally staged:

1. Reduce overlay/log noise and fix the ultrawide OpenGL3 display-size issue.
2. Add explicit test markers so light, heavy, and rejected objects can be
   correlated with individual drag sessions.
3. Use controlled x64dbg runs to identify the input-to-interaction call path
   while dragging one known movable object.
4. Use Ghidra and EMTK's signature scanner to replace temporary addresses with
   update-tolerant signatures and documented function hypotheses.
5. Observe object selection, interaction range, mass/weight response, applied
   force, release, and cancellation without changing them.
6. Implement the smallest reversible experiment: a configurable drag-strength
   multiplier guarded by the armed state.
7. Add range and object-eligibility experiments only after the force hook is
   stable, then investigate rotation and precision controls.
8. Keep every gameplay experiment optional, logged, and easy to disable; move
   generally useful framework improvements upstream to EMTK where appropriate.

The first milestone is not "drag anything" by assertion. It is a reproducible
hook that changes one verified force parameter for one verified interaction,
with an observe-only fallback and enough evidence to explain what changed.

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
