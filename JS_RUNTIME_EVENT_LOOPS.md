# Event Loop Architecture: rsvim vs. dune vs. LLRT vs. txiki.js

**libuv vs. tokio, and why it decides almost everything else.**

This document isolates one question — *who owns time, and how does JavaScript work become visible to
whoever does?* — and answers it for four embeddable runtimes. It is the deep-dive companion to
`JS_RUNTIME_COMPARISON.md`, which covers modules, API surface, startup and distribution.

Snapshots analyzed (local working trees; no runtime built or benchmarked):

| | repo | HEAD | language | engine | loop host |
|---|---|---|---|---|---|
| **rsvim** | `rsvim-dev/rsvim` | `ef9a345` (0.1.3-alpha.2) | Rust | V8 15.0.245.2 (`rusty_v8` 150.4.0) | **tokio** — the editor's `select!` |
| **dune** | `aalykiot/dune` | `5958b64` (v0.11.4) | Rust | V8 (`rusty_v8` 150.2.0) | **crabuv** — a libuv-shaped loop on `mio` + `rayon` |
| **LLRT** | `awslabs/llrt` | `d9fd873` (v0.9.0-beta-147) | Rust | QuickJS-NG 0.16.2 via `rquickjs` 0.14 | **rquickjs**'s `AsyncRuntime`, on tokio |
| **txiki.js** | `saghul/txiki.js` | `8a9b6044` (v26.6.0-136) | C | QuickJS-NG | **libuv** |

The 2×2 is neat enough to be a thesis: **two V8 runtimes, two QuickJS runtimes; two that drive a
loop, two that borrow one.** But the axes do not line up — dune is V8 *and* loop-owning, LLRT is
QuickJS *and* loop-borrowing — and the mismatches are where the interesting engineering lives.

---

## 0. The one-sentence version

| | Who owns the event loop? |
|---|---|
| rsvim | **The host editor does** — a `tokio::select!` over input, channels and cancellation; JS is a passenger |
| dune | **dune does** — `JsRuntime::run_event_loop` driving `crabuv`'s `RunMode::Once` iteration |
| LLRT | **rquickjs does** — `vm.idle().await`; LLRT has no *main* loop, only pumps |
| txiki.js | **txiki does** — `uv_run` plus a `uv_prepare`/`uv_check`/`uv_idle` job-queue bridge |

---

## 1. tokio and libuv are not two implementations of one idea

This is the part that gets glossed over, and everything downstream follows from it.

**libuv is a handle table with callbacks.** Every pending thing is a ref-counted handle —
`uv_timer_t`, `uv_tcp_t`, `uv_work_t`, `uv_idle_t`. `uv_run` runs one iteration: due timers,
poll, check, close. The loop is "alive" iff referenced handles remain, and that ref count is a
*public, inspectable* fact (`uv_ref`/`uv_unref`, `uv_loop_alive`). Time is a first-class loop
facility. A host calling `uv_run` regains control only when callbacks have run or the loop drained.

**tokio is a future executor with a reactor.** Time and I/O are exposed as futures; progress is
polling a task tree; the waker decides when to poll. There is no handle table to inspect, no
`loop_alive`, no notion of "callbacks for iteration N", and no ref counting. A host awaits, and the
executor decides what runs next. Time is a global facility (a timer wheel inside the runtime), not
a loop phase.

**A JS engine is a third party to both.** QuickJS has a *job queue* with a single entry point
(`JS_ExecutePendingJob`, with `JS_IsJobPending` to ask whether work exists). V8 has a *microtask
queue* with an explicit-policy checkpoint — but no equivalent of `JS_IsJobPending`; you can ask
whether background compilation is pending, not whether microtasks are queued.

So every runtime must answer a question its substrate cannot: **"is there JavaScript work left, and
when should it run?"** The four answers are the four architectures.

---

## 2. The four loops

### 2.1 rsvim — tokio as a message bus; JS has no loop at all

`rsvim_cli/src/bin/rsvim.rs:107` creates a multi-threaded `tokio::runtime::Runtime::new()` and
`block_on`s the editor future, so V8 stays on the main thread even though worker threads exist for
spawned tasks. The program's entire control flow is one `select!`
(`rsvim_core/src/evloop.rs:1347-1365`) over terminal input, master messages, JS messages and
cancellation.

`JsRuntime` is a **field of the editor's `EventLoop` struct** (`evloop.rs:142`), and JS advances
synchronously inside a select arm:

```rust
// rsvim_core/src/evloop.rs:1321-1327
async fn forward_js_message(&mut self, message: Option<JsMessage>) {
    if let Some(message) = message {
        self.jsrt_tx.send(message).unwrap();
        self.js_runtime.tick_event_loop();
    }
}
```

A tick is `checkpoint → fast_forward_imports → run_pending_futures` (`js.rs:611-618`). There is no
`uv_run` analogue anywhere: the editor loop *is* the runtime loop, and JS is a subroutine of it.

Native async is an actor protocol. A binding posts a `MasterMessage`; the select arm spawns a tokio
task on `detached_tracker` (`evloop.rs:816-835`); the task performs the I/O and sends a `JsMessage`
back on a **loopback** channel, which wakes the select and re-enters `forward_js_message`.
Results cross as `postcard` bytes (`evloop.rs:828`) — a deliberate ownership boundary, and a
per-operation encode/decode cost.

The diagnostic moment is when JS needs a turn and tokio has nothing to do. rsvim messages itself:

```rust
// rsvim_core/src/js.rs:627-637
if self.has_promise_rejections()
  || self.isolate.has_pending_background_tasks()
  || (self.unresolved_imports_count() > self.pending_import_loaders_count())
  || (!self.has_unresolved_imports() && self.has_pending_imports())
{
  chan::send_to_master(..., MasterMessage::TickAgainReq);
}
```

The master handles it by spawning a task whose entire body is to echo `TickAgainResp` back
(`evloop.rs:810-814`). That is an `await` on the executor built out of a channel round trip: two
scheduler hops to buy one JS turn.

Timers are not a loop facility either. Each one spawns a `tokio::time::sleep_until` task that posts
a message on expiry (`evloop.rs:774-789`), and `setInterval` is re-armed by re-inserting the
callback (`js.rs:687-696`). Interval drift is therefore whatever the tokio scheduler gives you.

Microtasks are explicit (`js.rs:144-146`), and there is exactly **one** checkpoint, at the *top* of
a tick (`js.rs:1097-1131`). `run_next_tick_callbacks` is checkpoint-only; its nextTick body is
commented out — a vestige of the Deno skeleton. So a promise resolved while draining futures has
its continuations drained on the next tick.

Two consequences of the "JS is a passenger" stance, both worth stating plainly:

- The config module runs **once, before the TUI exists** (`evloop.rs:541-574`; echoed messages are
  buffered and flushed after initialization, `evloop.rs:671-689`). There is no REPL, no `eval`.
- `*Sync` APIs execute inline on the editor task (`binding/global_rsvim/fs/open.rs:176-197`), so a
  slow synchronous call freezes input *and* rendering. There is no second thread to hide it on.

### 2.2 dune — writing libuv in Rust, then driving it from JS

`crabuv` (aalykiot's crate, v0.2.0) is **not libuv**: it is a from-scratch reimplementation over
`mio` (readiness), `rayon` (thread pool), `notify` (fs events) and `signal-hook`. But it is a
faithful clone of the *API and iteration order*:

```rust
// crabuv-0.2.0/src/event_loop.rs:169-188
loop {
    self.current_time = Instant::now();
    self.process_requests();      // uv__run_pending
    self.run_timers();            // uv__run_timers
    let timeout = self.poll_timeout(&mode);
    self.run_poll(timeout);       // mio::Poll::poll
    self.run_check_callbacks();   // uv__run_check
    self.run_close();             // uv__run_closing_handles
    match mode { RunMode::Default if !self.has_pending_events() => break, .. }
}
```

`has_pending_events()` is the `uv_loop_alive` clone — resources + queued requests + in-flight
thread-pool tasks (`event_loop.rs:657-661`) — and `RunMode::{Default, Once, NoWait}` mirrors
`UV_RUN_*`. `handle.spawn(work, cb)` is `uv_queue_work`: the job runs on a **rayon** worker, which
sends `Event::ThreadPool` and calls `waker.wake()` to break the poll (`event_loop.rs:454-466`).

The runtime owns the outer loop, and its condition is where JS work becomes visible:

```rust
// dune/src/runtime.rs:442-465
while self.event_loop.has_pending_events()
   || self.has_promise_rejections()
   || self.isolate.has_pending_background_tasks()
   || self.has_pending_imports()
   || self.has_next_tick_callbacks()
{
    self.poll_inspect_session();
    self.tick_event_loop();
    // report unhandled rejections
}
```

The four JS predicates are dune's invention, because the loop cannot see V8's queue. A tick is
`nextTick → fast_forward_imports → event_loop.run(Once) → run_pending_futures` (`runtime.rs:427-432`).
Note the ordering consequence: **JS callbacks do not run inside the loop iteration.** A completion
callback pushes a `JsFuture` onto `pending_futures` (`src/file.rs:145-160`), and the runtime drains
that vector after `uv_run` returns (`runtime.rs:479-504`), resolving promises then.

Because a ready JS future can exist while libuv has nothing to poll, dune must interrupt the poll
itself — the same problem txiki solves with `uv_idle`, solved with a waker and a flag:

```rust
// dune/src/timers.rs — inside create_timeout's loop callback
state.pending_futures.push(Box::new(future));
if !state.wake_event_queued {
    state.interrupt_handle.interrupt();   // mio::Waker
    state.wake_event_queued = true;
}
```

`setImmediate` is a `CheckHandle` (uv check phase), not a timer. The REPL leans on the same
machinery in miniature: an empty `u32::MAX` interval timer keeps the loop alive while a
`rustyline` thread interrupts the loop whenever a line is entered (`src/repl.rs:281-294`).

### 2.3 LLRT — rquickjs *is* the loop

`#[tokio::main]` plus `vm.idle().await` (`llrt/src/main.rs:115,131`). The VM wrapper is 219 lines in
total (`llrt_core/src/vm.rs`), and `idle()` is a delegation:

```rust
// llrt_core/src/vm.rs:29-32, 194-197
pub struct Vm { pub runtime: AsyncRuntime, pub ctx: AsyncContext }
pub async fn idle(self) -> ... { self.runtime.idle().await; Ok(()) }
```

`rquickjs`'s `AsyncRuntime` keeps an intrusive scheduler of Rust futures next to the QuickJS job
queue; `idle()` interleaves `JS_ExecutePendingJob` with polling that scheduler, under one
`async_lock::Mutex`. JS futures are **not tokio tasks** — they live in that scheduler and are only
polled from `idle()` / `drive()`, which is why a multi-threaded tokio process still serializes JS.
A native async function needs no plumbing: `Func::from(Async(read_file))` becomes
`Promise::wrap_future` → `JS_NewPromiseCapability` + `ctx.spawn`; only the underlying I/O is
offloaded to tokio, and the future itself is polled on the JS thread.

Where `rquickjs` has no concept, LLRT writes the pump:

- **Timers** (`modules/llrt_timers/src/lib.rs:35-41`, `305-398`): a `RuntimeTimerState` with a
  `deadline` and a `Notify`, driven by a task with `select! { notify, sleep }`. It clamps sleeps to
  `MIN_SLEEP = 4ms`, drains the job queue once before the first callback and after each one, and
  makes `setImmediate` sort first by giving it a deadline 600 s in the past.
- **`require`** (`llrt_modules/src/module/require.rs:111-121`): a synchronous CJS require built on
  the async `Module::import` by spinning the job queue — and firing due timers inline — until the
  import settles.
- **Lambda invocations** (`runtime_client.rs:400-410`): a request/response loop over the Runtime
  API.

So LLRT writes loop code after all — just not the JS loop: roughly 150 lines of pumps that exist
precisely because `rquickjs` schedules futures but knows nothing about timers or synchronous
`require`.

Two semantic deviations follow from the single-queue design: `process.nextTick` is
`queueMicrotask` (both `cb.defer`), and ref/unref accounting does not exist — timer handles are bare
numeric IDs with no `.unref()`/`.ref()` at all, and `dgram`'s methods are documented no-ops
(`modules/llrt_dgram/src/socket.rs:527-539`).

### 2.4 txiki.js — real libuv, with the job queue grafted on

`TJS_Run` starts `uv_prepare`/`uv_check`, **unrefs** both (`src/vm.c:1073-1076`), and loops:

```c
/* src/vm.c:1091-1095 */
for (;;) {
    do {
        uv__maybe_idle(qrt);
        r = uv_run(&qrt->loop, UV_RUN_DEFAULT);
    } while (r == 0 && JS_IsJobPending(qrt->rt) && !JS_HasException(qrt->ctx));
    ...
}
```

The crux is one function:

```c
/* src/vm.c:923-930 */
static void uv__maybe_idle(TJSRuntime *qrt) {
    if (JS_IsJobPending(qrt->rt) || !list_empty(&qrt->pending_rejections)) {
        CHECK_EQ(uv_idle_start(&qrt->jobs.idle, uv__idle_cb), 0);   // ref'd
    } else {
        CHECK_EQ(uv_idle_stop(&qrt->jobs.idle), 0);
    }
}
```

A ref'd `uv_idle` handle both keeps `uv_run` alive *and* forces libuv's poll timeout to zero, so JS
work can never be stranded behind a blocking poll. `uv_check` runs the jobs and re-evaluates
(`vm.c:1039-1046`). This is the cleanest answer of the four, because it uses libuv's own ref
counting instead of inventing predicates.

Microtasks drain at **three** points: `uv__check_cb`; `tjs_call_handler` after every callback routed
through it (`src/utils.c:205`, 41 C call sites in 13 files); and `EventTarget.dispatchEvent`
(`src/js/polyfills/event-target.js:657` → `core.drainMicrotasks()`), with a re-entrancy guard
`draining_microtasks` living in the C binding (`src/vm.c:184-198`).

Native async uses libuv's threadpool: `uv_queue_work` at `src/mod_fs.c:1365`, an after-work callback
that settles the promise at `:1339`, created at `:1372`. Timers are one `uv_timer_t` each. The price
of never re-entering `uv_run` is that HTTP imports block the loop — a
`while (!load_ctx->done) lws_service(...)` loop with a hard 10 s deadline per attempt and up to 20
redirects (`src/lws-utils.c:798-843`), all inside a JS evaluation.

Exit semantics are the sharpest in the four: `tjs.exit(n)` calls `uv_tty_reset_mode(); exit(n);`
immediately (`src/mod_os.c:37-38`) and **bypasses** `beforeunload`. The cancellable `beforeunload`
fires only on *normal* loop termination, and is skipped entirely after an uncaught exception or
unhandled rejection (`src/vm.c:1103-1119`).

---

## 3. The problem the substrate doesn't solve

### 3.1 Liveness, side by side

| | "Is there JS work left?" | What keeps the process alive | Wakeup mechanism |
|---|---|---|---|
| rsvim | four predicates in `tick_event_loop` (`js.rs:627-637`) | editor runs forever; exit only via `Rsvim.rt.exit` | channels; `TickAgainReq` round trip when JS needs a turn |
| dune | four predicates + `has_pending_events()` (`runtime.rs:442-465`) | referenced `crabuv` resources | `mio::Waker` (`interrupt_handle`); workers call `waker.wake()` on completion |
| LLRT | `rquickjs` scheduler (not visible to LLRT) | outstanding driver future in `idle()` | tokio `Notify` + `Sleep` in the timer driver |
| txiki.js | `JS_IsJobPending`, consulted by `uv__maybe_idle` | libuv ref counts; `uv_idle` is ref'd while jobs pend | libuv itself |

The pattern: **libuv gives you a vocabulary for "JS pending" as handle state, and tokio does not.**
txiki uses it directly, in one function. dune has the same vocabulary but only an approximation of
libuv's guarantee, so it adds `has_next_tick_callbacks()` and friends *outside* the loop and then
interrupts the loop to act on them. rsvim, which cannot interrupt its own select, sends itself a
message. LLRT sidesteps the question by borrowing `rquickjs`, then pays for it with two pumps.

### 3.2 Native async → promise

| | Native op runs on | Result crossing | Promise created | Settled |
|---|---|---|---|---|
| rsvim | tokio worker / blocking pool | `postcard` bytes over `mpsc` | `v8::PromiseResolver` in a `JsFuture` | next tick, from `pending_futures` |
| dune | rayon pool | `Box<dyn Any + Send>`, downcast in the loop | `v8::PromiseResolver` in a `JsFuture` | after the loop iteration, from `pending_futures` |
| LLRT | tokio I/O; future polled on the JS thread | in place (borrowed `Ctx`) | `Promise::wrap_future` | on future poll during `idle()` |
| txiki.js | libuv threadpool / loop thread | in place (loop thread) | `JS_NewPromiseCapability` (fs/dns) or JS `withResolvers()` | after-work / C callback |

### 3.3 Microtask ordering

| | Policy | Checkpoints | Consequence |
|---|---|---|---|
| rsvim | `Explicit` (`js.rs:144-146`) | **one**, at the top of a tick (`js.rs:1097-1131`) | continuations of futures resolved this tick run on the next tick |
| dune | `Explicit` (`runtime.rs:142`) | after nextTick (`:718`), after imports (`:615`), after each future (`:500`), REPL (`:294`) | continuations run in the tick that resolved them |
| LLRT | engine-managed | around timer callbacks; inside `idle()` | callbacks always observe a drained queue |
| txiki.js | engine-managed (job queue) | after `uv_check`, after helper-routed callbacks, after each listener | per-callback granularity; pinned by `tests/test-async-task-sequence.js` |

rsvim's ordering is unusual in both directions: the checkpoint is at the *start* of the tick, and
`run_next_tick_callbacks` contains no nextTick logic at all (`js.rs:1097-1118`) — a vestige of its
heritage.

### 3.4 Timers

| | Mechanism | Interval handling | Notes |
|---|---|---|---|
| rsvim | one tokio `sleep_until` task per timer, posts a message | re-created after each fire, `start_at` reset | drifts; ordering vs I/O is scheduler-dependent |
| dune | loop-native timer collection, run before poll | `TimerKind::Interval` inside the loop | true loop timers; `setImmediate` is a check handle |
| LLRT | LLRT pump on tokio `Sleep` + `Notify` | rearmed in `poll_timers` | 4 ms floor; `setImmediate` via past deadline |
| txiki.js | one `uv_timer_t` per timer | re-armed by the loop | true loop timers, unaffected by the job queue |

### 3.5 Threading

| | JS thread | I/O threads | Serialization |
|---|---|---|---|
| rsvim | main thread, inside the editor task | tokio multi-thread runtime + blocking pool | channel + `postcard` (JS never touched off-thread) |
| dune | main thread | rayon pool (`--threadpool-size`, default = cores) | `Send` box + loop callback (JS never touched off-thread) |
| LLRT | one thread, behind a runtime mutex | tokio multi-thread | futures polled on the JS thread; no serialization of results |
| txiki.js | libuv thread | libuv threadpool (4) | in-place; JS touched only from the loop thread; workers get their own loop |

---

## 4. What each position buys and costs

**rsvim: the host owns time.** Best when JS must be a guest inside an application that already has
an event loop — an editor, a game, a server with its own reactor. JS waiting can never block the
host, and shutdown is a `TaskTracker` drain. The bill is threefold: results are serialized across
the boundary, a turn must be *requested* rather than observed (`TickAgainReq`), and `*Sync` calls
freeze the UI because there is nowhere else to run them.

**dune: libuv's model without libuv.** It gets the callback-phase programming model and typed Rust
callbacks, and integrates cleanly with `spawn`. The bill is that it must re-implement the parts that
make libuv trustworthy — poll-timeout math, ref semantics, signals, fs watching, TCP — and it still
needs JS-level liveness predicates plus a waker interrupt, because `mio` does not provide libuv's
guarantees. It is a loop, not yet a runtime-grade loop.

**LLRT: borrow a scheduler, write the pumps.** The least loop code of the four (a 219-line VM file),
because `rquickjs` already solved V8-less scheduling. The bill is rigid coupling: everything LLRT
wants that rquickjs doesn't model — timers, synchronous `require`, Lambda invocations — becomes a
hand-written pump that must know when to drain the job queue.

**txiki.js: own the loop, own the ordering.** The only one where "JS pending" is first-class loop
state. That buys precise callback ordering, a cancellable `beforeunload`, and workers as genuine
runtime + loop pairs. The bill is C-level manual bridging everywhere, a loop that HTTP imports can
stall for 10 s, and an exit path that bypasses the graceful machinery.

**The compression:** if you are writing a runtime in Rust today, the choice is not "tokio or libuv"
— it is "whose loop do I want to be a subroutine of?" rsvim answered *the host's*; LLRT answered
*rquickjs's*; dune answered *mine, reimplemented*; txiki answered *libuv's, inherited*.

---

## 5. Verification notes

Everything here was read out of the four trees at the pinned commits. Three source audits were run
against the repos; the corrections that touched *loop* claims were:

- **LLRT**: the scheduler is polled by `idle()` and `drive()`, not `async_with()`; `poll_timers`
  drains once before the first callback and after each one (not before every callback); the timer
  driver, `require` pump and Lambda loop are ~150 lines of LLRT-owned loop code, so "no loop code"
  was overstated; timer handles have no `unref`/`ref` at all (stronger than "no ref accounting").
- **txiki.js**: `tjs.exit` bypasses `beforeunload`; the event only fires on normal termination, and
  is skipped after fatal errors; microtask draining happens after callbacks routed through
  `tjs_call_handler` (41 sites), not every `JS_Call`; the core object is an internal ABI
  (`tjs:internal/*` is rejected for user importers), so the public surface is the frozen `tjs`
  global.
- **rsvim**: import maps are a stub, not a feature; WASM loading reads UTF-8 so real binaries fail;
  `File` exposes `[Symbol.dispose]`, not `asyncDispose`; config runs once before the TUI; `*Sync`
  calls block the editor; microtask checkpoint placement is as described above.

Unverifiable here, and therefore asserted only from call sites:

- **txiki.js**: all `deps/` submodules (including `quickjs`) are uninitialized, so engine-internal
  behaviour — native `import()` settlement, module cache keys, circular imports — is inferred from
  txiki's own code, which contributes none of it.
- **LLRT**: `rquickjs`'s Rust source is not vendored locally, so its scheduler is described from
  LLRT's usage and the crate's public API. The vendored QuickJS-NG is 0.16.2.
- **dune**: `crabuv` was read at its pinned rev `2fda099` (v0.2.0); no release binary was built, so
  size and startup are unmeasured.
- **rsvim**: startup latency and "~0 prologue cost" are architectural claims; the binary was
  measured (64,227,600 B) but not timed.
