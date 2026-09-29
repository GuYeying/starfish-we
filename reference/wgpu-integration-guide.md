# starfish-window Integration Guide (for the wgpu rendering layer / interpreter hosts)

> Audience: the agent / developer integrating this library. This is the
> canonical entry point; it complements CLAUDE.md (repository engineering
> conventions) and "android绑定运行问题与解决方案.md" (the platform-runtime
> incident archive). Every "verified" claim below is tagged with its
> verification level: **real-hardware tested** / emulator-tested /
> compile-time (check).

中文版：[wgpu接入指南.md](wgpu接入指南.md)。

---

## 1. What this library is — and is not

**It is**: a cross-platform **single-window + events** library with an
SDL-style poll model — the caller owns the main loop and pulls events via
`poll_event()`; window operations are plain method calls. Built for the
"scripting interpreter drives its own tick" scenario (winit's inversion of
control conflicts with that).

**It is not**: a rendering library. It paints nothing — you attach wgpu;
the window entity reaches wgpu through raw-window-handle / raw handles.

Two iron rules run through the whole API:

1. **Single window**: at most one `Window` per process; a second `new`
   returns `WindowError::AlreadyExists`. Re-creating after drop works.
2. **Physical pixels**: every size/coordinate is a physical pixel (not CSS
   logical pixels, not scaled logical coordinates) — they line up directly
   with `wgpu::SurfaceConfiguration::width/height`.

---

## 2. Integration in three steps

### 2.1 Dependency and features

```toml
[dependencies]
starfish-window = { path = "…", features = ["wgpu"] }
```

| feature | meaning |
|---|---|
| `event` (default) | window + events (implies `window`) |
| `window` | window creation only (`--no-default-features --features window`) |
| `wgpu` | implements the raw-window-handle 0.6 traits (implies `window`); **wgpu itself is not a dependency** |
| `android-demo` | built-in Android demo host (logcat event loop + exit contract) |

### 2.2 Minimal window creation

```rust
use starfish_window::{Window, WindowError};

let mut window = Window::new("title", (800, 600), Default::default())?;   // shorthand
// Or the full-option builder (a new option = a new method; non-breaking):
let mut window = Window::builder()
    .title("demo")
    .size((800, 600))          // client area, physical pixels
    .resizable(true)           // default true
    .borderless(false)
    .min_size((400, 300))      // constrains user resizing
    .max_size((1920, 1080))
    .always_on_top(false)
    .build()?;
```

### 2.3 Main-loop skeleton (SDL style)

```rust
loop {
    while let Some(event) = window.poll_event() {   // non-blocking; pumps the platform source
        match event { /* see section 3 */ }
    }
    // ← advance your own tick here: interpreter bytecode / render a frame
}
```

**Never** pump the platform message loop yourself (Win32 PeekMessage /
NSApp / ALooper) — `poll_event` already drains it; pumping externally
steals events.

---

## 3. Event semantics — the full table

```rust
pub enum Event {
    CloseRequested,                 // user asked to close. End the loop, drop Window
    Resized { width: u32, height: u32 },   // client area, physical pixels → reconfigure surface
    FocusGained, FocusLost,
    KeyDown { key: KeyCode, modifiers: Modifiers, repeat: bool },
    KeyUp { key: KeyCode, modifiers: Modifiers },
    TextInput { text: String },     // final text (independent of KeyDown; no IME composition in v1)
    MouseMoved { x: f64, y: f64 },  // client-area origin, physical pixels
    MouseDown { button: MouseButton }, MouseUp { button: MouseButton },
    MouseWheel { delta_x: f64, delta_y: f64 },  // positive = up / right
    Touch { phase: TouchPhase, finger: u64, x: f64, y: f64 },  // touch devices only
    RedrawRequested,                // the platform asks for a redraw (wgpu: present here)
    Suspended, Resumed,             // mobile & Web only: surface invalid / restored
    User(UserEvent),                // host-injected custom event (see 3.5)
}
```

`KeyCode` follows the W3C `KeyboardEvent.code` naming (`KeyA` / `Digit1` /
`ArrowUp` / `Numpad0`… about 105 keys + `Other(native code)`); text always
travels through `TextInput`. `Modifiers { shift, ctrl, alt, meta }`
(meta = Windows logo / macOS Command).

**Platform production matrix** (variants a platform never produces simply
never appear on it):

| Event | Windows | Android | Web |
|---|---|---|---|
| CloseRequested | ✅ WM_CLOSE / Alt+F4 | ✅ Destroy / back key | ❌ (closing the page ends it) |
| Resized | ✅ | ✅ | ✅ |
| Focus | ✅ | ✅ | ✅ |
| Keyboard / TextInput | ✅ | ✅ (IME committed text) | ✅ (no IME composition) |
| Mouse | ✅ | synthesized from primary finger | ✅ |
| Touch | ❌ | ✅ | ✅ |
| RedrawRequested | ✅ WM_PAINT | ✅ RedrawNeeded/InitWindow | ✅ rAF, coalesced per frame |
| Suspended / Resumed | ❌ | ✅ Pause·Stop·TerminateWindow / Resume·InitWindow | ✅ page hidden/visible |
| User (custom) | ✅ host-injected | ✅ host-injected | ✅ host-injected |

---

## 3.5 Custom events (`Event::User`) — a typed replacement for SDL's `RegisterEvents`

The standard path for the host (interpreter worker threads, task systems,
any code) to inject messages into the main loop. **Platform independent**:
the channel lives in the common layer; behavior is identical on all six
platforms.

```rust
use starfish_window::{Event, EventProxy, UserEvent};

// ① Take a proxy (Clone, Send + Sync) — hand it to any worker thread
let proxy: EventProxy = window.event_proxy();
std::thread::spawn(move || {
    proxy.send(1, "script finished".to_string()).unwrap();   // code is host-defined
    proxy.send(2, Timer { id: 7 }).unwrap();
});

// ② Receive in the main loop
while let Some(event) = window.poll_event() {
    match event {
        Event::User(user) => match user.code {
            1 => { let msg = user.downcast_ref::<String>().unwrap(); }
            2 => { let timer: std::sync::Arc<Timer> = user.downcast().unwrap(); }
            _ => {}
        },
        _ => {}
    }
}

// ③ Same-thread convenience (equivalent to proxy.send, ignoring errors)
window.push_event(3, 12345u64);
```

Semantics and caveats:

- `UserEvent { code: u32, payload }`: `code` is the host-defined category;
  the payload is `Any + Send + Sync` **Arc-shared** (`Event: Clone` shares
  the payload — broadcast semantics; `downcast_mut` / `downcast` need
  exclusive ownership to succeed).
- **Ordering guarantee**: a `send` is always visible on the next
  `poll_event`; `Event::User` dispatches before that round's platform
  events. Batched injections dispatch in order (platform events are
  delayed briefly, never lost).
- `EventProxyClosed`: after the `Window` is destroyed, `send` fails
  explicitly (a protection SDL lacks).
- The platform event loop is unaware of user events: they are never eaten
  or reordered by the platform layer; Android's `WINDOW_READY` lifecycle
  filter does not apply to them either.
- `Event` intentionally does not implement `PartialEq` (the `User` variant
  carries an incomparable dynamic payload). Match with `match`/`matches!`.

---

## 4. The wgpu integration contract (rendering-loop template)

```rust
use starfish_window::{Event, Window};

let mut window = Window::builder().title("app").size((1280, 720)).build()?;
let instance = wgpu::Instance::default();
let mut surface = Some(instance.create_surface(&window)?);   // zero unsafe
let mut size = window.size();
// configure(&device, &SurfaceConfiguration { width: size.0, height: size.1, … })

loop {
    while let Some(event) = window.poll_event() {
        match event {
            Event::Resized { width, height } => {
                size = (width, height);
                // surface.configure(...) with the new size (also
                // reconfigure when get_current_texture returns
                // Suboptimal/Outdated — see the wgpu docs)
            }
            Event::Suspended => { surface = None; }   // Android: the system reclaimed the surface
            Event::Resumed => { /* rebuild + configure with the current size */ }
            Event::CloseRequested => { return Ok(()); }
            _ => {}
        }
    }
    if let Some(surface) = &surface {
        // get_current_texture → render → present (RedrawRequested marks the pacing)
    }
    // The interpreter tick advances here
}
```

Executable reference: `tests/wgpu_integration.rs` (verified on real
Windows hardware: create_surface → configure → acquire a frame →
re-configure → acquire again, all zero-unsafe).

Per-platform surface sources:

| Platform | surface source | status |
|---|---|---|
| Windows | `create_surface(&window)` (HWND+HINSTANCE carried automatically) | **verified on real hardware** ✅ |
| Android | same (ANativeWindow; valid only while `Resumed`) | traits implemented, device-side untested |
| Web | `window.canvas()` → wgpu's Web backend (needs a WebGPU-capable browser) | accessor ready, path not yet exercised |
| Linux / macOS / iOS | backends not implemented (stubs return `UnsupportedPlatform`) | see section 8 |

---

## 5. Per-platform driving principles (the part an agent must understand)

### 5.1 Windows (Win32, `windows-sys` FFI)

- **Mechanism**: the OS message queue natively supports polling.
  `poll_event` drains this thread's messages non-blockingly with
  `PeekMessageW(PM_REMOVE)` → `DispatchMessageW` → `wndproc` translates
  into a pending queue → one event is popped.
- The translation layer lives in `platform/windows/translate.rs`,
  compiled only under the `event` feature; messages returning `None` fall
  back to `DefWindowProcW` (Alt+F4, menu mnemonics, etc. keep their system
  defaults).
- The process declares Per-Monitor V2 DPI awareness → coordinates are
  always physical pixels.
- `WM_CLOSE` only reports `CloseRequested` and **does not destroy** the
  window; destruction happens when the `Window` is dropped
  (`DestroyWindow`). Continuing to poll after CloseRequested is safe.
- Mouse drags outside the window: `SetCapture` on the first button down,
  `ReleaseCapture` when all buttons are up.

### 5.2 Android (NativeActivity, `android-activity 0.6.1`)

- **Mechanism inversion**: the entry point and lifecycle are
  system-driven. The host integrates with one line:
  ```rust
  starfish_window::android_main!(my_app_main);   // or a hand-written android_main + android_init
  ```
  After that, the library translates `AndroidApp::poll_events(timeout=0)`
  + `input_events_iter()` callback streams **into a buffer queue** drained
  by `poll_event` — pure poll for the host.
- **Thread**: every call must happen on the `android_main` thread
  (android-activity asserts this in `poll_events`).
- **Lifecycle mapping**: `InitWindow→Resized+Resumed+Redraw`,
  `TerminateWindow→Suspended`, `Pause/Stop→Suspended`,
  `Destroy→CloseRequested`, `Back key→CloseRequested`.
- **Process reuse (three fixed pitfalls — do not regress)**:
  1. "Back" only destroys the Activity; **the process is cached**. On the
     next launch `android_main` runs again in the same process, on a new
     thread → `android_init` must update the app by overwrite (a
     `OnceLock` once silently ignored the second wiring, so every call
     landed on the previous run's destroyed app — a crash on relaunch via
     android-activity's thread assertion), and reset the event buffer /
     readiness flag / single-window slot.
  2. The old instance's late `Destroy` can cross-talk into the new
     instance's event stream → this used to cause "instant exit on
     relaunch" → now dropped by the `WINDOW_READY` gate (a Destroy before
     InitWindow is untrustworthy).
  3. **The glue's statics do not support same-process
     "destroy-mid-rebuild"** (unfixed upstream in 0.6.1): during
     old/new overlap its UI-thread callbacks abort.
     **Host exit contract: terminate the process immediately upon
     `CloseRequested` (or `Suspended`)** (the demo uses
     `libc::raise(SIGKILL)`; `exit(0)` leaves RenderThread SIGABRT exit
     noise). For an interpreter host (engine = process) the semantics are
     exactly equivalent.
- **Back-key trap**: the back KeyDown may be swallowed by the system (you
  only see `Suspended`) — never rely solely on the key event for exit
  decisions.
- **TextInput**: comes from the system IME's committed text
  (`InputEvent::TextEvent`).
- Packaging: the built-in demo host is the `android-demo` feature
  (`src/platform/android/entry.rs`). Building needs `ANDROID_NDK_ROOT`:
  `cargo apk build --features android-demo` (root package; APK metadata
  lives in the root Cargo.toml).

### 5.3 Web (wasm32-unknown-unknown, `wasm-bindgen` + `web-sys`)

- **Mechanism inversion**: the DOM is callback-driven. Listeners buffer
  events into a queue; `poll_event` drains it; the main loop's pacing is
  driven by the host (interpreter tick with `setTimeout` /
  `requestAnimationFrame`).
- **The window entity is a `<canvas>`**: looked up by id
  `starfish-window`, else the page's first canvas, else created filling
  the page. Rendering attaches via `window.canvas()` → wgpu's Web backend
  (needs a WebGPU-capable browser, Chrome/Edge 113+).
- Keys follow the W3C `KeyboardEvent.code` — same naming origin as
  `KeyCode`, a direct mapping.
- Coordinates = CSS coordinates × `devicePixelRatio` (physical pixels,
  same convention as desktop); on resize the library syncs the
  canvas.width/height attributes to the physical size.
- The `requestAnimationFrame` loop only sets a redraw flag (coalesced);
  `poll_event` yields at most one `RedrawRequested` per frame — treat it
  as the frame tick.
- **There is no `CloseRequested`**: closing the page ends everything.
  Switching tabs → `Suspended`/`Resumed`.
- Known edges: IME composition is not reported (final text only);
  surrogate-pair characters are dropped in v1; the wheel listener does
  not `preventDefault` (fine as long as the page itself doesn't scroll).

### 5.4 Linux / macOS / iOS (not implemented)

Stub behavior: `Window::new` returns
`WindowError::UnsupportedPlatform`; the six-target compile matrix is
green (the event/type layer is fully usable). Plan: X11 first for Linux
(`x11rb`, testable under WSLg), Wayland later as a horizontal split; the
Apple pair via the `objc2` family (macOS = NSWindow + NSEvent pump, iOS =
UIWindow + CAMetalLayer + buffer queue). **Do not promise shipments on
these three until they land.**

---

## 6. Red lines (violating any of these is a bug)

1. All `Window` methods must be called on the creating thread (Win32
   thread affinity / AppKit main thread / android_main thread). `Sync`
   exists only to satisfy renderer bounds such as wgpu's
   `create_surface`; calling methods cross-thread violates the contract.
2. At most one `Window` per process; on Android the slot is counted per
   "run" (`android_init` resets it automatically).
3. Sizes/coordinates are physical pixels — feed them straight into
   `SurfaceConfiguration`.
4. Android: never use the surface outside `Resumed` (`raw_handle` reports
   unavailable); the render loop must handle `Suspended` (destroy the
   surface) / `Resumed` (rebuild).
5. Android: exit by terminating the process (see 5.2); do not expect the
   process to be reusable after returning from `android_main`.
6. Never pump the platform message loop yourself.
7. The event enum is `#[non_exhaustive]`: always leave a `_` arm in your
   matches.

---

## 7. Diagnostics quick reference

| Platform | What to look at | Command |
|---|---|---|
| Windows | window + events demo | `cargo run --example basic` |
| Windows | wgpu integration regression | `cargo test --features wgpu --test wgpu_integration` |
| Windows | full smoke suite | `cargo test` |
| Android | event / lifecycle logs | `adb logcat -s starfish`; panics additionally under `RustStdoutStderr` |
| Android | build + install | `export ANDROID_NDK_ROOT=<ndk> && cargo apk build --features android-demo && adb install -r target/debug/apk/starfish-demo.apk` |
| Web | page overlay + console | `python -m http.server 8000 -d testweb` → http://localhost:8000 (`scripts/build_web.sh` repackages) |
| All platforms | compile matrix | `cargo check --target <t> --features wgpu` (six targets should all be green) |

Incident methodology (from "android绑定运行问题与解决方案.md",
universally applicable): reproduce first and harvest the crash output
(logcat / tombstones); the crash site ≠ the root cause; a green platform
compile matrix ≠ the platform runs (pure runtime differences like TLS
keys or process reuse are invisible to check); anything the system hands
over asynchronously (surfaces, canvases, adapters) should only be taken
after the platform's own readiness signal.

---

## 8. Current status (2026-09-26 — read before integrating)

| Platform | Implementation | Verification level |
|---|---|---|
| Windows | ~95% (missing: IME composition, logical-pixel DPI API, fullscreen/icon/position, runtime resizable) | real-hardware tested + automated tests |
| Android | ~90% (missing: GameActivity, gesture nav, soft-input control API, saveInstanceState) | emulator-tested (4 rapid relaunch cycles, 0 panics); real device pending |
| Web | ~85% (missing: IME, rwh Web registry, wheel preventDefault) | event layer manually verified ✅; the wgpu-WebGPU path not yet exercised |
| Linux / macOS / iOS | 0% (stubs) | compile-only |

Version facts: raw-window-handle **0.6** (`#[non_exhaustive]`; use the
`new()` constructors; `WindowsDisplayHandle` / `AndroidDisplayHandle` are
empty structs; the Android variant is named `AndroidNdk`); compatible
wgpu version **30** (`get_current_texture` returns the enum
`CurrentSurfaceTexture`, not a Result; `SurfaceConfiguration` gained the
required `color_space` field); wasm-bindgen pinned at **0.2.127** (must
match wasm-bindgen-cli exactly).
