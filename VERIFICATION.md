# Verification: 0.2.0

Verified on Windows x64 on 2026-10-06.

- Offline release build: passed.
- Unit tests: 4 passed (native structure layout, queue ordering/overflow,
  cursor session/cancellation, exclusion of ordinary typed characters).
- Clippy, all targets, warnings denied: passed.
- Rustfmt check: passed.
- Native hidden-host integration: passed twice against the same process.
  Each attachment captured twelve known input events, one completed cursor
  session with 65 pixels of movement, and a sent cancellation message.
  Callback process/thread IDs matched the test host. No events were dropped.
  No further callbacks were observed after detachment.
- Live Exanima 0.9.5.2f / Steam build 25733142: passed an eight-second attach
  and detach test. The runtime executed inside Exanima's actual window thread.
  There were 76 callbacks, two recorded messages and zero dropped events.
  No mouse/keyboard input was exercised in this live run; controlled input
  coverage comes from the native test host, not from gameplay.

The live test establishes process attachment and callback execution. It does
not establish world-object identification, physical dragging, force modification
or compatibility with other Exanima builds. Those remain future experiments.

On the development machine the sandbox hid external game processes. The live
test ran in the ordinary desktop process context; the native hidden-host tests
also passed inside the sandbox.
