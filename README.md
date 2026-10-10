# starfish-window

[中文说明](README.zh-CN.md)

A cross-platform **single-window + events** library for Rust with an
SDL-style poll model — the caller owns the main loop and pulls events,
instead of handing control flow to event-loop callbacks. Designed for
[wgpu](https://wgpu.rs) rendering layers and scripting-language
interpreter hosts, where winit's inversion of control conflicts with the
host's own main loop.

```toml
[dependencies]
starfish-window = { path = ".", features = ["wgpu"] }
```

```rust
use starfish_window::{Event, Window};

let mut window = Window::new("demo", (800, 600), Default::default())?;
let surface = instance.create_surface(&window)?;   // wgpu feature: zero-unsafe passthrough

loop {
    while let Some(event) = window.poll_event() {  // non-blocking pull
        match event {
            Event::Resized { width, height } => { /* reconfigure surface (physical pixels) */ }
            Event::Suspended => { /* Android/Web: surface invalid, destroy until rebuilt */ }
            Event::Resumed => { /* rebuild the surface */ }
            Event::CloseRequested => break,
            _ => {}
        }
    }
    // ← your tick: run bytecode / render a frame
}
```

## Highlights

- **SDL-style run model** — the caller owns the main loop;
  `poll_event()` pulls events one at a time, non-blocking
- **Single window** — at most one `Window` per process; no window-id
  routing; a minimal API surface
- **Physical-pixel convention** — sizes/coordinates line up directly with
  `wgpu::SurfaceConfiguration`
- **Zero-dependency base layer** — the default features pull no
  third-party crates; the `wgpu` feature only depends on
  `raw-window-handle 0.6` (wgpu itself is deliberately not a dependency)
- **Custom events** — `EventProxy` injects typed payloads into the main
  loop from any thread (a typed replacement for SDL's `RegisterEvents`)
- **Lean feature set** — events are core; the only optional features are
  `wgpu` and `android-demo`

## Platform status

| Platform | Window | Events | Rendering | Status |
|---|---|---|---|---|
| Windows | ✅ | ✅ keys/mouse/wheel/focus | ✅ wgpu verified on real hardware | usable |
| Android | ✅ | ✅ incl. touch + lifecycle | ✅ handles ready | usable (emulator-verified) |
| Web (wasm) | ✅ canvas | ✅ incl. touch + page visibility | `Window::canvas()` | usable |
| Linux / macOS / iOS | — | — | — | planned (stubs) |

Platform driving principles, the event semantics matrix, caveats and
diagnostics live in the
[integration guide](reference/wgpu-integration-guide.md)
(Chinese: [reference/wgpu接入指南.md](reference/wgpu接入指南.md)).

## Development

```bash
cargo check                                   # the six-platform compile matrix
cargo test                                    # Windows smoke + real-hardware wgpu test
cargo run --example basic                     # Windows event demo
cargo apk build --features android-demo       # Android demo APK (needs ANDROID_NDK_ROOT)
scripts/build_web.sh                          # Web packaging → testweb/
```

### Run the Web demo

```bash
scripts/build_web.sh                          # build → wasm-bindgen → testweb/
python -m http.server 8000 -d testweb         # serve the bundle
# open http://localhost:8000 — events print on the page overlay and in the
# browser console
```

## License

Dual-licensed, pick either:

- [MIT](LICENSE-MIT)
- [Apache-2.0](LICENSE-APACHE)
