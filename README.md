# EXAMINER

Independent, from-scratch x64 Windows hooks for Exanima. Version 0.2.0 replaces
the previous experiment with original source. The current source has no external
Rust packages, modding frameworks, bundled libraries, fonts or image assets.
It uses Rust's standard library and Windows APIs. Earlier commits remain in Git
history; they are not part of this implementation.

## What the hook does

`examiner.exe` discovers Exanima's process and window thread, then installs two
thread-specific hooks from our own `examiner_runtime.dll`:

- `WH_GETMESSAGE` observes removed queue messages, including mouse input,
  Ctrl/Shift/Alt, F2/F6 and raw-input notifications.
- `WH_CALLWNDPROC` observes sent focus and cancellation messages.

Windows loads our callback DLL into the selected process. A shared-memory queue
reports its actual process/thread IDs and event records to our controller.
The controller writes JSONL logs and removes the hooks on timeout or Ctrl+C.
The hooks pass every message through unchanged. They are never global hooks.
Ordinary typed characters are not recorded; F2/F6 are observed, not intercepted.

This is a real in-process input hook, not an object-selection or physics hook.
Cursor movement does not prove that Exanima selected or moved a physical object.
`distance_px` is the sum of absolute cursor X/Y movement during a left-button
session. Raw-input payloads are not decoded in this version.

## Build

Requires stable Rust for `x86_64-pc-windows-msvc` and the Microsoft C++ Build Tools
with a Windows SDK. There are no packages to download.

```powershell
cargo build --release --offline
cargo test --offline
```

The release folder contains `examiner.exe`, `examiner_runtime.dll` and
`examiner_test_host.exe`. Keep them together.

## Use

Start Exanima normally, then run from the release folder:

```powershell
.\examiner.exe inspect
.\examiner.exe hook --seconds 120 --log .\logs\session.jsonl
```

Use `--pid PID` when more than one Exanima process is running. The controller
verifies the process image name and x64 PE header, and allows only one controller
per target window thread. The hook requires the target to have a window and pump
Windows messages. The controller and game must run at compatible integrity levels.

The program reports successful attachment only after confirming a callback from
the actual target process/thread. Timeouts, attach failures and dropped events
remain visible. The program does not modify game files or saves.

`Start-EXAMINER.cmd` attaches for up to one hour and keeps the console visible.
It finds either the packaged `bin` folder or a normal `target/release` build.
Start Exanima first. [Verification results](VERIFICATION.md) describe the tested
build and the limits of the live test.

## Native Self-Test

```powershell
.\examiner.exe self-test --log .\logs\self-test.jsonl
```

The test starts our own hidden window process. It exercises both real Windows
hooks, checks twelve known input events and a 65-pixel mouse session, verifies
the callback's process/thread identity, verifies that callbacks stop after
detachment, then repeats attachment to the same process. It does not synthesize
desktop keyboard/mouse input or attach to unrelated applications.

## Next Research

1. Correlate captured input with known light, heavy and rejected game objects.
2. Identify Exanima's real selection/interaction functions on the current build.
3. Implement our own verified function interception for a specific interaction.
4. Introduce an optional drag-force multiplier only after identifying its real
   parameter and calling convention.

Input hooks do not expose arbitrary engine objects. Engine function interception
will require additional reverse engineering and its own tests; it is not claimed
as implemented by this release.

## Windows API References

- [SetWindowsHookExW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexw)
- [GetMsgProc](https://learn.microsoft.com/en-us/windows/win32/winmsg/getmsgproc)
- [UnhookWindowsHookEx](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-unhookwindowshookex)

EXAMINER is unofficial and is not affiliated with Bare Mettle Entertainment.
Only our tooling source belongs in this repository; game binaries and game data
are supplied by the user's own installation.
