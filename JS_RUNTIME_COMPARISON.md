# JavaScript Runtime Architecture: rsvim vs. dune vs. LLRT vs. txiki.js

A source-level comparison of four embeddable JS runtimes, organized around the question that
actually determines a runtime's shape: **who owns time?** The event loop, the module loader and
the async bridge are the same problem seen from three angles, so the document starts there and
then works outward into modules, API surface, startup and distribution.

Snapshots analyzed (local working trees, read-only):

| | repo | HEAD | language | engine | loop host |
|---|---|---|---|---|---|
| **rsvim** | `rsvim-dev/rsvim` | `ef9a345` (0.1.3-alpha.2) | Rust | V8 15.0.245.2 (`rusty_v8` crate 150.4.0) | **tokio** (the editor's `select!`) |
| **dune** | `aalykiot/dune` | `5958b64` (v0.11.4) | Rust | V8 (`rusty_v8` crate 150.2.0) | **crabuv** — a libuv-shaped loop on `mio` + `rayon` |
| **LLRT** | `awslabs/llrt` | `d9fd873` (v0.9.0-beta-147) | Rust | QuickJS-NG via `rquickjs` 0.14 | **rquickjs**'s `AsyncRuntime`, on tokio |
| **txiki.js** | `saghul/txiki.js` | `8a9b6044` (v26.6.0-136) | C | QuickJS-NG | **libuv** |

Two of them are V8 + a Deno-style Rust skeleton; two are QuickJS-NG. Two drive their own loop;
two borrow someone else's. That 2×2 is the map for everything below.

---

## 0. Positioning in one line each

- **rsvim** — a *V8 + tokio scripting layer embedded in an editor*. There is no runtime loop; the
  editor's event loop is the runtime's loop, and JS is advanced by it. The public surface is
  `Rsvim.*`, not `globalThis.fetch`.
- **dune** — a *standalone V8 + libuv-shaped runtime shaped like Node*. It owns a loop, but a
  hand-written one: `crabuv` reproduces libuv's API and semantics over `mio` and `rayon`.
- **LLRT** — a *QuickJS + tokio runtime shaped like Node*, optimized for Lambda cold starts. It
  owns no *main* loop: `rquickjs` schedules JS futures, while LLRT supplies the APIs plus two
  hand-written pumps (timers, `require`) and the Lambda invocation loop.
- **txiki.js** — a *QuickJS-NG + libuv runtime shaped like the Web*. It owns a real libuv loop,
  hand-rolls the job-queue bridge, and treats workers as separate runtime + loop pairs.

The single most diagnostic fact about each:

| | Who owns the event loop? |
|---|---|
| rsvim | **The host editor does** — `tokio::select!` over input, channels and cancellation |
| dune | **dune does** — `JsRuntime::run_event_loop` driving `crabuv`'s `RunMode::Once` |
| LLRT | **rquickjs does** — `vm.idle().await`; LLRT has *no* loop code of its own |
| txiki.js | **txiki does** — `uv_run` plus a `uv_prepare`/`uv_check`/`uv_idle` job-queue bridge |

---

## 1. Architecture at a glance

| Dimension | rsvim | dune | LLRT | txiki.js |
|---|---|---|---|---|
| Engine binding | `v8` crate, raw `v8::` API | `v8` crate, raw `v8::` API | `rquickjs` (`ModuleDef`, no fork) | QuickJS-NG C API, direct |
| Host / platform | tokio | `crabuv` (mio + rayon) | tokio (via `AsyncRuntime`) | libuv |
| Loop owner | editor loop | runtime | `rquickjs` | runtime |
| JS threads | 1 (shares the editor task) | 1 (main thread) | 1 (serialized behind a mutex) | 1 + N workers (own runtime + own loop) |
| Microtasks | **explicit**, one checkpoint at the top of a tick | **explicit**, several checkpoints | managed by `rquickjs` `idle()` | drained at 3 points around `uv_run` |
| Module system | **ESM only** (no CJS, no `require`) | **ESM only** (no CJS, no `require`) | **ESM + CJS** (`require`, interop) | **ESM only** (no CJS, no `require`) |
| npm / bare specifiers | `node_modules` via `oxc_resolver`, `default` condition only | **none** — import maps only | resolve at *build time* (esbuild + shims), runtime resolver as fallback | **none** — import maps only |
| TypeScript | `.ts` via swc (`rsvim_cli` default feature) | `.ts` / `.tsx` / `.jsx` via swc | no (pre-compile yourself) | no |
| Startup accelerator | **V8 startup snapshot** built by the CLI crate | none (JS sources recompiled per run) | precompiled bytecode in a `phf` map | `qjsc` bytecode; `core`+`polyfills` eager |
| Embeddable as a lib | yes (`rsvim_core`) | no (bin-only; `main.rs` owns the runtime) | `llrt_core` is a lib, `llrt` is bin-only | yes (`add_library(tjs STATIC)`) but no `install()` rule |
| Runtime core size | ~7.9k LOC non-test in `rsvim_core/src/js/` (~18k incl. tests); ~31k non-test repo-wide | ~9.1k LOC non-test Rust + ~6.4k LOC JS | ~3.8k core glue (`llrt_core/src`) + ~2.6k `llrt_modules`; `modules/` ≈ 57.9k, `libs/` ≈ 7.2k, ~73k Rust total | ~35k LOC C (excl. generated bundles) + ~22k LOC JS |
| Release binary | ~64 MB (64,227,600 B at `ef9a345`) | not measured here | intentionally small | small when built with the documented size flags (`BUILD_WITH_GC_SECTIONS` is off by default) |
| Permission model | none | none | network allow/deny lists only | none |
| REPL / debugger | no REPL; inspector code commented out | REPL + Chrome DevTools inspector | REPL (non-lambda build) | REPL; no inspector |

A note that matters for the rest of the document: **rsvim and dune are near-siblings.** Both carry
the same Deno-style skeleton — `ModuleMap` / `ModuleGraph` / `EsModule` / `same_origin` /
`JsFuture` / `pending_futures` / `ImportMap` — with the same `tick_event_loop` shape. Compare
`rsvim_core/src/js.rs:611-618` with `dune/src/runtime.rs:427-432`: both do *nextTick → imports →
futures*, and both call `v8::Isolate::set_microtasks_policy(Explicit)`. The interesting difference
is what each plugged in underneath: tokio (rsvim) versus a hand-written libuv clone (dune).

---

## 2. Event loops: two substrates, four answers

A standalone, deeper treatment of this section — including the audit trail — is in
`JS_RUNTIME_EVENT_LOOPS.md`.

### 2.1 tokio and libuv are not two implementations of one idea

- **libuv** is a *handle table with callbacks*. Every pending thing is a ref-counted handle
  (`uv_timer_t`, `uv_tcp_t`, `uv_work_t`, `uv_idle_t`); `uv_run` blocks in the poll phase until an
  fd, a timer or a wakeup fires; the loop is "alive" iff referenced handles remain. Time is a
  first-class loop facility. The host calls `uv_run` and regains control only when callbacks ran or
  the loop drained.
- **tokio** is a *future executor with a reactor*. Time and I/O expose futures; progress is driven
  by polling a task tree. There is no handle table to inspect, no `uv_loop_alive`, no notion of
  "callbacks for iteration N". The host awaits, and the executor decides what runs next.

Everything below follows from that. A JS engine, meanwhile, offers a third model: QuickJS has a
**job queue** with one entry point (`JS_ExecutePendingJob`, `JS_IsJobPending`), and V8 has a
**microtask queue** with an explicit-policy checkpoint. Both are invisible to tokio and to libuv.
Each runtime therefore has to answer a question its substrate cannot: *"is there JavaScript work
left, and when should it run?"*

### 2.2 rsvim: the host owns the loop, JS is a passenger

The whole program is one `tokio::select!` (`rsvim_core/src/evloop.rs:1347-1365`) over terminal
input, master messages, JS messages and cancellation. `JsRuntime` is a **field of the editor's
`EventLoop` struct** (`evloop.rs:142`), so V8 is driven inline from the select arm:

```rust
// rsvim_core/src/evloop.rs:1321 — the only place JS advances on a message
self.jsrt_tx.send(message).unwrap();
self.js_runtime.tick_event_loop();
```

There is no separate JS thread and no `uv_run` analogue. A tick is
`checkpoint → fast_forward_imports → run_pending_futures` (`js.rs:611-618`).

Async JS is an actor protocol. A native op posts a `MasterMessage` over an unbounded channel; the
select arm spawns a tokio task on `detached_tracker` (`evloop.rs:816-835`); the task runs the real
I/O and sends a `JsMessage` back on a **loopback** channel, which wakes the select, gets pushed
into the JS queue and triggers a tick (`evloop.rs:1321-1327`). Results cross the boundary as
`postcard` bytes (`evloop.rs:828`).

The tell-tale consequence: when JS has work pending but the tokio reactor has nothing, rsvim has to
message itself for another turn. That is `TickAgainReq` (`js.rs:627-637`), handled by spawning a
task whose entire body is to echo `TickAgainResp` back (`evloop.rs:810-814`). It is an `await` on
the executor, built out of a channel round trip.

Timers are not a timer facility: each one spawns a `tokio::time::sleep_until` task that posts a
message when it expires (`evloop.rs:774-789`), and `setInterval` is re-armed by re-inserting the
callback (`js.rs:687-696`).

Two runtime-level facts worth knowing: the config module is evaluated **once, before the TUI
exists** (`evloop.rs:541-574`; echoed messages are buffered and flushed after initialization,
`evloop.rs:671-689`), and there is no REPL or `eval`. Also, `*Sync` APIs run inline on the editor
task (`binding/global_rsvim/fs/open.rs:176-197`), so a slow synchronous fs call freezes input and
rendering.

### 2.3 dune: a libuv clone, driven from JS

`crabuv` is aalykiot's own crate (v0.2.0; deps `mio`, `rayon`, `notify`, `signal-hook`, `slotmap`).
It is **not libuv**; it is a faithful re-implementation of libuv's API surface and iteration order:

```rust
// crabuv-0.2.0/src/event_loop.rs:169-188 — the uv_run shape
loop {
    self.current_time = Instant::now();
    self.process_requests();
    self.run_timers();
    let timeout = self.poll_timeout(&mode);
    self.run_poll(timeout);        // mio::Poll::poll
    self.run_check_callbacks();
    self.run_close();
    match mode { RunMode::Default if !self.has_pending_events() => break, .. }
}
```

The runtime owns the outer loop (`dune/src/runtime.rs:442-465`), and its condition is where the JS
work becomes visible:

```rust
while self.event_loop.has_pending_events()
   || self.has_promise_rejections()
   || self.isolate.has_pending_background_tasks()
   || self.has_pending_imports()
   || self.has_next_tick_callbacks() { self.tick_event_loop(); }
```

`has_pending_events()` is the `uv_loop_alive` clone: resources + queued requests + in-flight
thread-pool tasks (`event_loop.rs:657-661`). The four JS predicates are dune's invention, because
the loop cannot see V8's microtask queue.

Async work uses the `uv_queue_work` equivalent — `state.handle.spawn(work, Some(cb))` sends the job
to a **rayon** pool; the worker sends `Event::ThreadPool` and calls `waker.wake()` to break the
poll (`event_loop.rs:454-466`). The completion does *not* call JS: it pushes a `JsFuture` onto
`pending_futures` (`src/file.rs:145-160`), which is drained after the loop iteration returns
(`runtime.rs:479-504`).

And when a timer makes a JS callback ready while the loop could block, dune interrupts the poll on
purpose, guarded by a flag so it only does it once:

```rust
// dune/src/timers.rs — inside create_timeout's loop callback
state.pending_futures.push(Box::new(future));
if !state.wake_event_queued {
    state.interrupt_handle.interrupt();   // mio::Waker
    state.wake_event_queued = true;
}
```

`setImmediate` is a `CheckHandle` (uv check phase), not a timer. The REPL shows the same style: it
keeps the loop alive with an empty `u32::MAX` interval timer and a reader thread interrupts the
loop when a line is entered (`src/repl.rs:281-294`).

### 2.4 LLRT: rquickjs *is* the loop

`#[tokio::main]` + `vm.idle().await` (`llrt/src/main.rs:115,131`). The whole runtime core is a
219-line file, and `idle()` is a delegation:

```rust
// llrt_core/src/vm.rs:29-32, 194-197
pub struct Vm { pub runtime: AsyncRuntime, pub ctx: AsyncContext }
pub async fn idle(self) -> ... { self.runtime.idle().await; Ok(()) }
```

`rquickjs`'s `AsyncRuntime` keeps an intrusive scheduler of Rust futures next to the QuickJS job
queue; `idle()` interleaves `JS_ExecutePendingJob` with polling that scheduler, all under one
`async_lock::Mutex`. JS futures are **not tokio tasks** — they live in that scheduler and are only
polled from `idle()` / `drive()` (`rquickjs-core-0.14.0/src/runtime/async.rs:301-355`), which is
why a multi-threaded tokio process still serializes JS. A native async function needs no plumbing:
`Func::from(Async(read_file))` becomes `Promise::wrap_future` → `JS_NewPromiseCapability` +
`ctx.spawn`.

Where `rquickjs` has no concept, LLRT writes the pump itself:

- **Timers** (`modules/llrt_timers/src/lib.rs:35-41`, `305-398`): a `RuntimeTimerState` with a
  `deadline` and a `Notify`, driven by a spawned task with `select! { notify, sleep }`. It clamps
  sleeps to `MIN_SLEEP = 4ms`, drains the job queue once before the first callback and after each
  one, and makes `setImmediate` sort first by giving it a deadline 600 s in the past.
- **`require`** (`llrt_modules/src/module/require.rs:111-121`): a synchronous CJS require
  implemented by spinning the job queue — and firing due timers inline — until `Module::import`
  settles. It is the strangest corner of the runtime.

Because everything is one queue, two semantics deviate from Node: `process.nextTick` is
`queueMicrotask` (both `cb.defer`), and ref/unref accounting does not exist — a pending timer keeps
the process alive because the driver future is outstanding, and `dgram`'s `unref()`/`ref()` are
documented no-ops (`modules/llrt_dgram/src/socket.rs:527-536`).

### 2.5 txiki.js: real libuv, with the job queue grafted on

`TJS_Run` starts `uv_prepare`/`uv_check`, **unrefs** both, and loops:

```c
/* src/vm.c — the outer loop */
for (;;) {
    do {
        uv__maybe_idle(qrt);
        r = uv_run(&qrt->loop, UV_RUN_DEFAULT);
    } while (r == 0 && JS_IsJobPending(qrt->rt) && !JS_HasException(qrt->ctx));
    ...
}
```

The trick is `uv__maybe_idle` (`src/vm.c:923-929`): if QuickJS has a pending job or a pending
rejection, start a **ref'd** `uv_idle` handle; otherwise stop it. A ref'd handle keeps `uv_run`
alive *and* forces libuv's poll timeout to 0, so JS work can never be stranded behind a blocking
poll. `uv_check` runs the jobs, then re-evaluates. This is the cleanest answer of the four to the
liveness question, because it uses libuv's own ref-counting instead of inventing predicates.

Microtasks drain at **three** points: `uv__check_cb`, `tjs_call_handler` after every callback
routed through it (`src/utils.c:205`, 41 C call sites), and `EventTarget.dispatchEvent`
(`src/js/polyfills/event-target.js:657` → `core.drainMicrotasks()`), with a re-entrancy guard
`draining_microtasks` living in the binding (`src/vm.c:184-198`).

Native async uses libuv's threadpool: `uv_queue_work` (`src/mod_fs.c:1365`) plus an after-work
callback that settles the promise at `:1339`, created at `:1372`. Timers are one `uv_timer_t`
each. The price of never re-entering `uv_run` is that HTTP imports block the loop: a
`while (!load_ctx->done) lws_service(...)` loop with a hard 10 s deadline per attempt and up to
20 redirects (`src/lws-utils.c:798-843`), all inside a JS evaluation. `tjs.exit(n)` bypasses
`beforeunload` and calls `exit()` immediately (`src/mod_os.c:37-38`); *normal* loop termination
fires a cancellable `beforeunload` that can re-enter the loop, except after an uncaught exception
or unhandled rejection (`src/vm.c:1103-1119`).

### 2.6 The liveness problem, side by side

| | "Is there JS work left?" | What keeps the process alive | Wakeup mechanism |
|---|---|---|---|
| rsvim | four predicates in `tick_event_loop` (`js.rs:627-637`) | editor runs forever; exit only via `Rsvim.rt.exit` | channels; `TickAgainReq` round trip when JS needs a turn |
| dune | four predicates + loop's own `has_pending_events` (`runtime.rs:442-465`) | referenced `crabuv` resources | `mio::Waker` from `interrupt_handle`; worker completion calls `waker.wake()` |
| LLRT | `rquickjs` scheduler (not visible to LLRT) | outstanding driver future in `idle()` | tokio `Notify` + `Sleep` in the timer driver |
| txiki.js | `JS_IsJobPending` consulted by `uv__maybe_idle` | libuv ref counts; `uv_idle` is ref'd while jobs pending | libuv itself |

The pattern is unmistakable: **libuv gives you a vocabulary for "JS pending" as handle state, and
tokio does not.** txiki uses it directly, in one function. dune has the same vocabulary but only an
approximation of libuv's guarantee, so it adds `has_next_tick_callbacks()` and friends *outside*
the loop and then interrupts the loop to act on them. rsvim, which cannot interrupt its own select,
sends itself a message. LLRT sidesteps the question entirely by borrowing `rquickjs`, then pays for
it with two hand-written pumps.

### 2.7 Native async → promise, side by side

| | Native op runs on | Result crossing | Promise created | Settled |
|---|---|---|---|---|
| rsvim | tokio worker threads / blocking pool | `postcard` bytes over `mpsc` | `v8::PromiseResolver` stored in a `JsFuture` | next tick, from `pending_futures` |
| dune | rayon thread pool | `Box<dyn Any + Send>`, downcast in the loop | `v8::PromiseResolver` in a `JsFuture` | after the loop iteration, from `pending_futures` |
| LLRT | tokio I/O offloaded; the future itself is polled on the JS thread | in place (same thread, borrowed `Ctx`) | `Promise::wrap_future` (`JS_NewPromiseCapability`) | on future poll during `idle()` |
| txiki.js | libuv threadpool / loop thread | in place (loop thread) | `JS_NewPromiseCapability` (fs/dns) or JS `withResolvers()` | after-work / C callback |

### 2.8 Microtask checkpoints and ordering

| | Policy | Checkpoints | Ordering consequence |
|---|---|---|---|
| rsvim | `Explicit` (`js.rs:144-146`) | **one**, at the top of a tick (`js.rs:1097-1131`) | promises resolved while draining futures have their continuations drained on the *next* tick |
| dune | `Explicit` (`runtime.rs:142`) | after `nextTick` drain (`runtime.rs:718`), after imports (`:615`), after each future (`:500`), REPL dispatch (`:294`) | continuations run in the same tick that resolved them |
| LLRT | engine-managed | inside `poll_timers` before/after each callback; inside `idle()` | JS callbacks always run with a drained queue |
| txiki.js | engine-managed (job queue) | after `uv_check`, after each callback routed through the helper, after every listener dispatch | per-callback granularity; pinned by `tests/test-async-task-sequence.js` |

rsvim's ordering is worth flagging because it is unusual in both directions: the checkpoint is at
the *start* of the tick, and `run_next_tick_callbacks` is checkpoint-only — the actual nextTick
queue code is commented out (`js.rs:1097-1118`), a vestige of the Deno skeleton it inherited.

### 2.9 Timers

| | Mechanism | Interval handling | Notes |
|---|---|---|---|
| rsvim | one tokio `sleep_until` task per timer, posts a message | re-created after each fire, `start_at` reset | drifts; ordering relative to I/O is tokio-scheduler dependent |
| dune | loop-native timer collection, run before poll | `TimerKind::Interval` inside the loop | true loop timers; `setImmediate` is a check handle |
| LLRT | LLRT-owned pump on tokio `Sleep` + `Notify` | rearmed inside `poll_timers` | 4 ms floor; `setImmediate` via past deadline |
| txiki.js | one `uv_timer_t` per timer | re-armed by the loop | true loop timers; unaffected by job queue |

---

## 3. ES module support

### 3.1 Feature matrix

| | rsvim | dune | LLRT | txiki.js |
|---|---|---|---|---|
| static `import`/`export` | ✅ (V8) | ✅ | ✅ | ✅ (QuickJS) |
| dynamic `import()` | ✅ custom host hook | ✅ custom host hook | ✅ | ✅ engine-side (txiki writes none of it) |
| `import.meta` | ✅ `filename`, `dirname`, `url`, `main`, `resolve()` | ✅ `url`, `main`, `resolve()` | ✅ | ✅ `url`, `main`, `dirname`, `basename`, `path` |
| top-level await | ⚠️ evaluates, host does **not** await (the editor loop keeps running) | ⚠️ evaluates, host does **not** await (the loop keeps running) | ✅ (entry run through `import()`) | ✅ (entry `await core.evalFile(...)`) |
| JSON modules | ⚠️ source rewrite via template literal | ⚠️ same rewrite | ✅ via loader | ✅ import attribute or `.json` suffix |
| WASM modules | ⚠️ loader reads UTF-8, so real binaries fail | ⚠️ same defect | ❌ (no engine support) | ❌ as an ESM type; ✅ `WebAssembly` API on WAMR + `tjs:wasi` |
| import attributes | ❌ ignored | ❌ ignored | ⚠️ JSON only, and *required* for JSON | ✅ `type: "json" \| "text" \| "bytes"` |
| import maps | ❌ **stub only** | ✅ (WICG subset) | ❌ (loader *hooks* instead) | ✅ (WICG spec) |
| remote `http(s)://` imports | ❌ (loader commented out) | ✅ with sha1-keyed disk cache | ❌ | ✅ **yes** |
| CommonJS / `require` | ❌ explicitly unsupported | ❌ unsupported | ✅ | ❌ |
| `node_modules` | ✅ resolve, ESM only, `default` condition | ❌ | ✅ always-on resolver, bundling recommended | ❌ |
| TypeScript | ✅ `.ts` via swc | ✅ `.ts`/`.tsx`/`.jsx` via swc | ❌ | ❌ |

Two corrections to the obvious reading of that table:

- rsvim's import-map support is a **mock-up**. `ImportMap` says so in a doc comment,
  `parse_from_json()` returns an empty map, `lookup()` always returns `None`, and the real
  implementation is commented out (`js/module/import_map.rs:27-41`, `:43-92`). Nothing ever
  populates the option (`js.rs:329-349`). dune's import map, by contrast, is real and handles
  trailing-slash packages and extensionless specifiers (`src/modules.rs:488-538`).
- Both V8 runtimes' JSON support is the same one-liner,
  ``format!("export default JSON.parse(`{source}`);")`` (`rsvim .../fs_loader.rs:26-28`,
  `dune/src/loaders.rs:49-51`). Any JSON containing a backtick or `${` is corrupted, and both wasm
  loaders read the module with `read_to_string` before re-emitting bytes, which rejects valid
  binary `.wasm`. They are inherited defects, not independent mistakes.

### 3.2 Resolution algorithms: four different models

**rsvim** picks a loader by specifier and delegates path math to `oxc_resolver`:

```rust
// rsvim_core/src/js/loader/fs_loader.rs:107-135
ResolveOptions {
  extensions: vec![".js", ".ts", ".mjs", ".json", ".wasm"],
  modules: vec![config_home(), config_home()/node_modules, "node_modules"],
  ..   // note: extension_alias, but no condition_names
}
```

No `condition_names` are set, so `package.json#exports` is matched for the `default` condition only
(`import`/`node` keys do not resolve). CJS is an explicit non-goal in a doc comment
(`fs_loader.rs:166-177`), `.jsx`/`.tsx` are commented out (`:211-216`, `:253-258`), and `.ts` is the
only transpiled extension. The practical consequence: a package like `lodash` *resolves* and then
fails to evaluate, because it is CJS.

**dune** dispatches on three loader classes — core modules, URLs, filesystem — in
`resolve_import`/`load_import` (`src/modules.rs:428-477`). Bare specifiers only resolve if an
import map rewrites them; otherwise `FsModuleLoader::resolve` bails with `Module not found`
(`src/loaders.rs:98-118`). URL imports are fetched with `ureq`, hashed with SHA-1 and cached in
`~/.dune/cache` (`.cache` in debug builds), and `--reload` bypasses the cache. The import map
implementation is a sorted prefix table with WICG's trailing-slash rule
(`src/modules.rs:488-538`).

**LLRT** keeps a full runtime `PackageResolver` — a port of Node's `LOAD_AS_FILE` /
`LOAD_AS_DIRECTORY` / `LOAD_NODE_MODULES` — in the default resolver tuple
(`llrt_core/src/vm.rs:106-112`), so unbundled `node_modules` work. Bundling is still strongly
recommended: `build.mjs` runs **esbuild** (`platform: "browser"`, `format: "esm"`) over LLRT's own
JS entrypoints and the AWS SDK, inlining dependencies and swapping in `shims/` for packages that
reach into Node internals; user code is bundled by the user's own esbuild/rollup/webpack recipe.
QuickJS' resolver is told to strip the `node:` scheme
(`llrt_modules/src/module/resolver.rs:43`).

**txiki.js** is the web purist: a single `JS_SetModuleLoaderFunc2` (`src/vm.c:553`).
`tjs:internal/core` and `tjs:internal/worker` are C-built modules (the rest of `tjs:internal/*` is
bytecode), `tjs:*` deserializes precompiled bytecode (`src/builtins.c:78-103`), `http(s)://`
**downloads and compiles at runtime** via libwebsockets, and everything else is a file, optionally
JS/JSON/text/bytes by import attribute or `.json` suffix. Bare specifiers hit the import map;
unmatched ones pass through unchanged (`src/modules.c:356-378`), which means no `node_modules` by
design.

### 3.3 The dynamic-import promise

- **rsvim**: `host_import_module_dynamically_cb` (`js/hook.rs:181`) creates a
  `v8::PromiseResolver`, registers a `ModuleGraph` in `ModuleMap::pending`, and settles it later in
  `fast_forward_imports` (`js.rs:843-906`, resolve at `:903`). Duplicate concurrent imports are
  coalesced via `graph.same_origin()`.
- **dune**: the same design, one file over (`src/hooks.rs:145+`, settle in
  `runtime.rs:600-609`), including the `same_origin` coalescing list.
- **LLRT**: delegated to `rquickjs`'s loader; `run_file` literally becomes
  `import("<path>").catch((e) => {console.error(e);process.exit(1)})` (`vm.rs:171-176`).
- **txiki.js**: QuickJS-ng implements `import()` natively on top of the loader function, so there is
  no hand-written promise.

### 3.4 CommonJS interop — the real dividing line

Only LLRT does it, because AWS SDK v3 leaks CJS-shaped packages into ESM graphs. The mechanism is
two magic prefixes: `__cjs:` ("this path came from a CJS import") and `__cjsm:` ("force CJS handling
in the loader") (`llrt_modules/src/lib.rs:79-81`).

- **`import` of a CJS file** → `PackageLoader::load_cjs_module` *synthesizes module source*
  (`package/loader.rs:20-58`):
  `const value = require("..."); export default value.default||value;` plus named exports derived
  from the runtime `Object.keys(module.exports)`.
- **`require` of an ESM file** → `Module::import` + the nested pump described in §2.4, then
  default-object export promotion (`require.rs:130-187`).
- **Cycles** are handled through `RequireState { cache, exports, progress }`; the HEAD commit
  (`#1824`) added an identity check so a normalized path reaching the same in-progress object
  returns it instead of copying properties.

rsvim, dune and txiki refuse CJS structurally, and say so in code: rsvim lists it as a non-goal,
and dune and txiki contain no `require` calls at all (txiki has one comment that mentions it).

### 3.5 Where each one hurts

- rsvim: **CJS wall** plus a `default`-only `exports` resolver. Built-in TypeScript is `.ts` only,
  and only if you build the `rsvim` binary (the `rsvim_core` default feature set excludes it).
- dune: **no npm story at all** unless an import map or a URL import is used — a deliberate trade
  that buys it the cleanest resolution code of the four. `.jsx`/`.tsx` support is real, unlike
  rsvim's commented-out arms.
- LLRT: **bundling is effectively mandatory**; the runtime resolver is the escape hatch. No
  TypeScript, by design.
- txiki.js: **no npm without a bundler or import map**, but it is one of two that can `import`
  from a CDN URL directly (txiki compiles it via libwebsockets; dune caches it).

---

## 4. Builtin runtime APIs

### 4.1 rsvim — minimal by intent, editor-first

`globalThis` implements only the WinterTC minimum (`js/runtime/00__web.js:486-493`): timers,
`queueMicrotask`, `reportError`, `TextEncoder`/`TextDecoder` (backed by `encoding_rs`, so
independent of the `icudata` feature). Everything else hangs off `Rsvim`
(`01__rsvim.js:1795-1801`): `buf`, `cmd`, `fs`, `opt`, `proc`, `rt`, `syn`.

| Group | Present | Notes |
|---|---|---|
| Timers | ✅ | tokio sleeps under the hood |
| Text | ✅ | `encoding_rs` |
| **`console`** | ❌ | use `Rsvim.cmd.echo()` |
| fetch / URL / Headers / Blob | ❌ | `url_loader` commented out |
| streams / EventTarget / AbortController | ❌ | |
| crypto / WebCrypto | ❌ | |
| `fs` | ✅ Deno-style: `open`/`readFile`/`readTextFile`/`readDir`/`stat`/`lstat`/`mkdir`/`symlink`/`link` + `*Sync`; `File` with `[Symbol.dispose]` | not Node-shaped |
| `process` / `child_process` | ✅ `Rsvim.proc` (Deno.Command-shaped) | `ChildProcess` has `[Symbol.asyncDispose]` |
| stdio | ⚠️ stdout/stderr readable only | no writable stdin |
| Editor API | ✅ buffers, commands, options, syntax/tree-sitter | the actual product |
| workers | ❌ | |

Errors are UI events, not process exits: uncaught exceptions and unhandled rejections become
command-line messages (`js/err.rs:164-166`, `js.rs:1133-1225`), and `Rsvim.rt.exit` is the only exit
path. There is no `console` because an editor has a command line.

### 4.2 dune — "Node, for a hobby runtime"

19 modules are registered in `CORE_MODULES` (`src/modules.rs:28-52`) and installed as
`process.binding(name)` namespaces plus top-level modules: `console`, `events`, `process`, `timers`,
`assert`, `util`, `fs`, `perf_hooks`, `colors`, `dns`, `net`, `test`, `stream`, `http`, `sqlite`,
`@web/abort`, `@web/text_encoding`, `@web/clone`, `@web/fetch`. Globals are `process`, `console`,
`queueMicrotask`, `prompt`, `setTimeout`/`setInterval`/`setImmediate` (+ clears),
`TextEncoder`/`TextDecoder`, `structuredClone`, `AbortController`/`AbortSignal`, `fetch`, `print`,
`reportError` (`src/js/main.js:3-66`, `src/bindings.rs:56-58`).

| Group | Present | Notes |
|---|---|---|
| `fs` | ✅ Node-shaped: 10 async + 10 `*Sync` + `createReadStream`/`createWriteStream`/`watch` | `File` class, `open`/`stat`/`mkdir`/`rm`/`copyFile` |
| `net` | ✅ `createServer`/`createConnection`, `Socket`/`Server` on `EventEmitter` | `TimeoutError` included |
| `http` | ✅ **client + server** (`createServer`, `request`, `Server`, `ServerRequest`) | hand-written over `httparse` + net |
| `sqlite` | ✅ `Database` / `DatabaseSync` over rusqlite (bundled) | |
| `dns` | ✅ `lookup` + IPv4/IPv6 validators | |
| `events` / `stream` / `assert` / `util` / `perf_hooks` | ✅ / ⚠️ | `stream` is only `pipeline`/`compose`; `util` is one function; `assert` is a 692-line reimplementation |
| `console` / `process` / `timers` | ✅ | `console` wraps V8's console; `process` has argv/env/cwd/exit/nextTick/binding/kill/uptime/memoryUsage |
| `test` | ✅ built-in runner (`dune test`) | Deno-inspired |
| `fetch` | ⚠️ wrapper over `http.request` | "not fully compatible with WHATWG fetch" (README) |
| URL / Buffer / EventTarget / crypto / child_process / workers / WebSocket | ❌ | no `path`, no `os` either |
| Inspector | ✅ Chrome DevTools protocol over axum WebSocket | `--inspect[=addr]` |

`process.binding` is the load-bearing innovation relative to rsvim: native namespaces are looked up
in a `lazy_static` registry of initializers (`src/bindings.rs`, `src/process.rs:229-237`) and cached
in JS (`src/js/process.js:4-21`). It gives dune a Node-shaped internal ABI without a Node-shaped
public one.

### 4.3 LLRT — "Node subset for Lambda", the widest surface

Enabled by default via `base` — 27 feature entries that register **29** module names
(`llrt_modules/Cargo.toml:43-71`; five of the entries are global installers, not modules).
Verified against the `modules/` tree rather than the README matrix:

| Node API | Status | Notes |
|---|---|---|
| `fs`, `fs/promises` | ✅ | most async calls have a `*Sync` twin; `open` has no `openSync` |
| `path`, `os`, `url`, `util`, `buffer`, `events`, `zlib` | ✅ | `events` = `EventEmitter` + `EventTarget` |
| `net`, `dns`, `dgram`, `tty` | ✅ | |
| `tls` | ❌ | no `node:tls`; the crate only holds client config for `https`/`fetch` |
| `child_process` | ✅ | `spawn` / `execFile` |
| `crypto` | ✅ | RustCrypto by default; ring / graviola / openssl selectable |
| `module` | ✅ | `builtinModules`, `createRequire`, `isBuiltin`, `registerHooks` |
| `perf_hooks`, `string_decoder`, `assert` | ✅ / ✅ / ⚠️ | `assert` is 78 LOC |
| `timers`, `console`, `abort`, `async_hooks` | ✅ | `async_hooks` opt-in via `LLRT_ASYNC_HOOKS=1` |
| `stream`, `stream/promises` | ⚠️ JS-only | a ~230-line adapter over bundled `readable-stream` 4.7.0 |
| `stream/web` | ✅ | WHATWG streams, ~14.9k LOC — the largest module |
| `http` | ❌ absent | there is no `http` module |
| `https` | ⚠️ | registered, but only exports `Agent`; hosts the internal hyper client |
| `http2`, `cluster`, `vm`, `v8`, `wasi`, `inspector`, `readline`, `querystring`, `sqlite`, `worker_threads` | ❌ | absent as modules — HTTP/2 *is* reachable through `fetch` (`LLRT_HTTP_VERSION=2`) |

Web platform: `fetch` (Request/Response/Headers/FormData/Blob over hyper + rustls with selectable
backends), WHATWG streams, `URL`, `TextEncoder`, WebCrypto, `EventTarget`/`Event`/`CustomEvent`,
`AbortController`, `structuredClone` (incl. `transfer`), `performance`, `Blob`/`File`, compressed
bodies. Absent: **`WebSocket`** (zero hits), `XMLHttpRequest`, WebAssembly, `EventSource`.
`navigator` is a 19-line stub. `Intl` is a minimal `DateTimeFormat`/`supportedValuesOf` subset over
`jiff` behind a feature, not ECMA-402; `Temporal` is likewise gated.

LLRT-only: `llrt:qjs`, `llrt:codec`, `llrt:util`, `llrt:xml` (the last exists so `fast-xml-parser`
can be shimmed for the AWS SDK). Plus the raison d'être: `sdk.cfg` + `build.mjs` pre-register
`@aws-sdk/client-*` entry points, and `llrt_core/src/runtime_client.rs` speaks the Lambda Runtime
API over hyper.

### 4.4 txiki.js — "the Web, plus a knife drawer"

Two layers: C exposes one `core` object of primitives (27 namespace initializers — 26 `mod_*` plus
WebCrypto — `src/vm.c:241-268`) installed as `tjs:internal/core`, and `src/js/` wraps it into
standards-shaped globals. The `core` object is an internal ABI: `tjs:internal/*` is rejected unless
the importer is itself a `tjs:` module (`src/modules.c:343-351`); the public surface is the frozen
`tjs` global (`src/js/core/index.js:231`).

Web platform: `fetch` (H1/H2/H3, proxy, cookie jar), `WebSocket` + `WebSocketStream`, `EventSource`,
`XMLHttpRequest`, streams + `CompressionStream`, `Blob`/`File`/`FileReader`/`FormData`,
`AbortController`, `EventTarget`/`Event`/`CustomEvent`, `URL`/`URLPattern`/`URLSearchParams`,
`TextEncoder`/`TextDecoder` (+ stream variants), WebCrypto (mbedtls; TweetNaCl for
Ed25519/X25519), `structuredClone`, `performance`, `navigator`, `localStorage` (SQLite-backed) /
`sessionStorage` (in-memory), `Worker`, `MessageChannel`, `BroadcastChannel`, `console`, timers,
`WebAssembly` (WAMR **interpreter**), Direct Sockets.

Node-ish / systems: address lookup, TCP/UDP/TLS/pipe sockets (via `tjs.lookup` and
`tjs.connect`/`tjs.listen`, plus Direct Sockets), an HTTP+H2+H3 **server** with WS upgrade, child
processes, signals, stdio, `path`. txiki-only stdlib: `tjs:sqlite`, `tjs:ffi` (libffi),
`tjs:hashing`, `tjs:ipaddr`, `tjs:readline`, `tjs:uuid`, `tjs:getopts`, `tjs:posix-socket`,
`tjs:wasi`, `tjs:utils`, `tjs:assert`, `tjs:path`.

Absent: **any `node:` namespace**, `require`, `Buffer`, `EventEmitter`, `process`. There is a frozen
`tjs.*` global with fs-like helpers, but it is not Node-shaped. Also no public `*Sync` fs API.

Build-time features change the surface: `BUILD_WITH_WASM=OFF` removes the `WebAssembly` global and
`tjs:wasi`; `BUILD_WITH_SQLITE=OFF` removes `tjs:sqlite` and degrades `localStorage` to memory.
`tjs.engine.{compile,serialize,deserialize,evalBytecode,gc}` (`src/js/core/engine.js:5-31`) is the
public bytecode API that `tjs compile` and embedded precompiled code build on.

### 4.5 Coverage verdict

| | rsvim | dune | LLRT | txiki.js |
|---|---|---|---|---|
| Node.js compatibility | ✗ (~0) | ★★ (fs/net/http/events/assert/process) | ★★★☆ (deliberate subset) | ✗ (none, by policy) |
| Web/WinterTC | ☆ (timers + text) | ★ (fetch-lite, text, abort) | ★★★ | ★★★★ |
| Systems / native | `fs`, child process | fs, TCP, HTTP server, SQLite | `net`, `dns`, `dgram`, `zlib` | sockets, TLS, HTTP **server**, sqlite, FFI, WASM, Worker |
| Editor domain | ★★★★ | ✗ | ✗ | ✗ |
| Concurrency | ✗ | ✗ | ✗ (single isolate) | ★★★★ (`Worker` = thread + runtime + loop) |
| Tooling | snapshot build | REPL, test runner, bundler, compiler, inspector | REPL, test runner, bytecode compiler | REPL, test runner, esbuild bundler, bytecode/standalone compiler, app packager |
| The gap | `console`, `URL`, `fetch`, streams, `EventTarget` | `URL`, `Buffer`, crypto, workers, npm | `WebSocket`, workers, `node:http2` module, `vm`, WASM | Node compat (intentional) |

---

## 5. How host functions actually reach JS

This determines how expensive a new API is to add.

- **rsvim** — hand-written `v8::Function` callbacks installed into one hidden global,
  `__InternalRsvimGlobalObject` (`js/binding.rs:58`), with argument conversion via the
  `rsvim_macro::ToV8`/`FromV8` derives. The JS layer is typed, hand-written, and **minified with
  terser at build time** (`dev.py:360-365`); the checked-in `*.min.js` is what actually runs
  (`js.rs:157-173`), so editing `01__rsvim.js` alone changes nothing.
- **dune** — hand-written `v8::Function` callbacks installed into `process.binding(name)` namespace
  objects, then wrapped by readable Node-style JS modules (`src/js/*.js`). No macro layer: argument
  validation is manual `v8::Local::<T>::try_from`. Adding an API = a Rust fn + a registry entry +
  a JS wrapper.
- **LLRT** — `rquickjs`'s `ModuleDef` trait (not the `#[rquickjs::module]` attribute macro, of which
  there are zero uses). Each module registers a name and `LoadFn`; async functions need no special
  handling: `Func::from(Async(read_file))`.
- **txiki.js** — plain C functions with `TJS_CFUNC_DEF` in a `JSCFunctionListEntry` table, attached
  to the flat `core` object, then a JS wrapper compiled to bytecode with `qjsc`.

A useful tell for codebase maturity: rsvim and dune both hand-roll V8 callbacks at roughly the same
level of abstraction, but rsvim generates bindings from Rust types while dune validates at runtime.

---

## 6. Startup and distribution

| | rsvim | dune | LLRT | txiki.js |
|---|---|---|---|---|
| Mechanism | V8 **startup snapshot**: `build.rs` instantiates a snapshot isolate, evaluates both builtin modules, `create_blob(FunctionCodeHandling::Keep)` → `RSVIM_SNAPSHOT.BIN` → `include_bytes!` (`rsvim_cli/build.rs:91-102`, `js.rs:157-202`) | no snapshot: `include_str!` all 19 core modules, recompiled at first import each run | `build.rs` compiles every builtin JS to QuickJS bytecode, zstd-compressed against a 40 KB trained dictionary (`--maxdict=40K`), embedded in a `phf` map (`llrt_core/build.rs`) | `qjsc`-compiled bytecode in `src/bundles/c/**`, loaded with `JS_ReadObject` |
| Who builds it | the **CLI** crate, with `rsvim_core` as a build-dependency, `--predictable --random-seed=42` | n/a | the core crate | the build system |
| Laziness | builtins already **evaluated** inside the snapshot | no: every core module parsed on first import | **lazy**, decompressed per import | `core` (~72 KB) + `polyfills` (~291 KB) **eager per runtime**; `tjs:*` lazy |
| User code | parsed at load | parsed at load; `dune compile` bundles (swc) + zstd + 8-byte trailer `4tr31d1s` appended to the binary | `.lrt` bytecode via `llrt compile`; `--executable`/`pack` → self-contained binary | bytecode via `tjs compile`; 12-byte `tx1k1.js` trailer |
| Cost | ~64 MB binary, fast start | small binary, slower start (parses ~6.4k lines of JS each run) | small binary, fast start | small binary, pays for eager polyfills on every runtime — including every worker |

The trade-off triangle: rsvim buys startup with binary size; LLRT buys it with a build step; dune
buys simplicity and pays at startup; txiki buys simplicity and pays per-runtime.

One distribution detail worth flagging for rsvim: because the snapshot is produced by
`rsvim_cli/build.rs`, `rsvim_core` alone is not a runnable-embedded runtime with builtins — an
embedder inherits the raw `v8::` API plus the JS sources, not the prepared heap.

---

## 7. What each architecture optimizes for — and what it costs

**rsvim: safety and editor coherence over throughput.**
The actor/channel boundary means no binding can corrupt editor state, and the codebase is already
structured for a future where JS lives on its own thread. Cost: every async op serializes its result,
the loop needs explicit `TickAgainReq` nudges, microtasks are manual and checkpointed at the tick
boundary, `*Sync` calls freeze the UI, and the JS surface is still tiny (`console` is missing;
`fetch`/`URL`/streams are absent). For its actual job — running an `rsvim.{ts,js}` config that
registers commands and edits buffers, once, before the TUI appears — that is a defensible trade.

**dune: a Node-shaped programming model without a Node engine.**
It is the most "batteries included" of the four relative to its size: Node-style `fs` with sync twins,
an HTTP server, SQLite, a test runner, a bundler, standalone compilation, a REPL and a real Chrome
DevTools inspector. The cost is that the loop is a re-implementation: poll-timeout math, ref
semantics, signals, fs watching and TCP all live in a 0.2.0 crate, and the JS-level liveness
predicates plus the waker interrupt exist precisely because the loop cannot see V8's queue. It is a
loop, not yet a runtime-grade loop.

**LLRT: surface area over loop innovation.**
It is the only one with real CJS interop and a Node-shaped API, and the only one that treats npm as a
*build* problem while still shipping a runtime resolver. Its event loop is borrowed wholesale from
`rquickjs`, which is why its own glue is only ~3.8k LOC even though the whole tree is ~73k Rust
lines. The cost is rigid coupling to that runtime's
scheduling (patched at the seams with hand-written pumps for timers and `require`), no workers, and a
build pipeline that must bundle before deploy.

**txiki.js: control over the loop, standards over compatibility.**
Hand-owning the `uv_prepare`/`uv_check`/`uv_idle` dance gives precise control of when JS runs, and
that is what makes real OS threads tractable: each `Worker` gets its own `JSRuntime`, its own context
and its own `uv_loop_t` on a new thread, exchanging messages via `JS_WriteObject2`/`JS_ReadObject2`
structured clone over heap mailboxes + `uv_async`. The Web-first API plus FFI/SQLite/WAMR makes it the
most capable "small" runtime of the four. The cost is C memory management everywhere, no CJS, and no
npm without a bundler or import map.

A caveat on "embeddable": txiki is *architecturally* embeddable (`add_library(tjs STATIC)` + a
48-line `tjs.h` with nine `TJS_*` entry points), but there is no `install()` rule, no exported CMake
package, no pkg-config file and no versioned ABI — and `TJS_Run` always evaluates the CLI `run-main`
bundle unless the runtime is a worker. The embedder knobs are correspondingly thin:
`TJS_NewRuntimeOptions` exposes only `mem_limit`/`stack_size` (`tjs.h:33-36`), only worker runtimes
may block (`JS_SetCanBlock`, `src/vm.c:527`), and `TJS_Stop` is thread-safe via `uv_async_send`
(`src/vm.c:1130-1133`). It is a library by construction, not by contract.

---

## 8. If you are deciding

- **Embedding JS in a host application that already owns time** (an editor, a game, a server) →
  rsvim's shape. Let the host's loop drive everything, never let JS block it, accept the channel and
  serialization tax. The alternative — embedding a libuv-shaped runtime and pumping it from your
  loop — is dune's shape, and it works, but you inherit a second scheduler.
- **Shipping a standalone Node-flavoured tool with broad API coverage** → dune's shape for breadth
  (HTTP server, SQLite, REPL, inspector) or LLRT's shape for Lambda cold starts and CJS interop.
- **Shipping a small general-purpose runtime with web APIs and workers** → txiki's shape. Static lib,
  libuv, standards, FFI escape hatch, WAMR.
- **Minimizing runtime code** → LLRT. Outsource the loop to `rquickjs` and write only the pumps.

The common thread is **where each one puts the npm problem**:

- rsvim — *solves it at resolution time* (`oxc_resolver` over `node_modules`, `default` condition),
  then loses most of the ecosystem because CJS is banned.
- dune — *refuses it* (import maps, URL imports, or pre-bundling).
- LLRT — *solves it at build time* (esbuild + shims), with a Node-faithful resolver as the escape hatch.
- txiki.js — *refuses it*, and treats that refusal as a feature.

And on the loop question specifically: **tokio wants to own time; libuv wants to own handles; V8 and
QuickJS both own a queue that neither can see.** A runtime is largely the glue that reconciles those
three claims — and the four differ mostly in how much glue they were willing to write.

---

## 9. Provenance / caveats

Everything above was read out of the four working trees at the commits listed at the top; no runtime
was built or benchmarked. Known gaps:

- **txiki.js**: the `deps/` submodules (including `quickjs`) are **not checked out** (`git submodule
  status` shows `-` for all), so anything that lives inside the engine is asserted from txiki's call
  sites, not from QuickJS-ng source. That covers dynamic `import()` promise settlement, module cache
  keys and circular-import semantics — txiki contributes no code for these, which is itself the
  verified finding.
- **LLRT**: `rquickjs-sys` 0.14.0 vendors **QuickJS-NG 0.16.2** (`quickjs/quickjs.h:1450-1453`; no
  git revision recorded), but the crate's Rust source is not vendored locally, so its scheduler is
  described from LLRT's usage and the crate's public API. `THIRD_PARTY_LICENSES` is stale (it still
  cites QuickJS 2021-03-27 / rquickjs 0.4.0) and the README compatibility matrix is stale in places
  (it still marks `dgram` ✘); prefer the `modules/` tree.
- **dune**: no release binary was built here, so binary size and startup latency are unmeasured, and
  `crabuv` was read at its pinned rev `2fda099` (v0.2.0). V8 engine version is not asserted beyond
  the `rusty_v8` crate version (`150.2.0`); `process.versions.v8` would report it at runtime.
- **rsvim**: "~0 prologue cost" and startup latency are architectural claims, not measurements; the
  binary size (64,227,600 B) and version string were read from an existing release build at HEAD.
- **Both V8 runtimes** share inherited ``JSON.parse(`…`)`` and UTF-8-read `.wasm` loader defects;
  these are reported as code-level observations, not exercised against fixtures.
