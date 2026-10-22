# Dynamic `import()`: rsvim vs. dune vs. Ende vs. Node.js

A source-level trace of one V8 embedder callback — `HostImportModuleDynamicallyCallback` — through
four runtimes that all implement it, and four different answers to the same question: *once V8 hands
you a specifier and expects a promise back, who owns the state machine that fills that promise?*

This is the module-loading companion to `JS_RUNTIME_COMPARISON.md` and
`JS_RUNTIME_EVENT_LOOPS.md`; the three share the same source-level, read-only methodology.

Snapshots analyzed (local working trees, read-only; **nothing was built or executed**):

| | repo | HEAD | engine | where the machine lives |
|---|---|---|---|---|
| **rsvim** | `rsvim-dev/rsvim` | `fd74024` (0.1.3-alpha.2) | V8 150.4.0 | embedder, hand-written (`rsvim_core/src/js/`) |
| **dune** | `aalykiot/dune` | `5958b64` (v0.11.4) | V8 150.2.0 | embedder, hand-written (`src/`) |
| **Ende** | `theende/ende` | `edbd53a78` | V8 via `ende_core` | engine layer (`libs/core/`), embedded in a Deno fork |
| **Node.js** | `nodejs/node` | `14ed05f86bd` (v27.0.0-pre) | V8 via `deps/v8` | JS layer (`lib/internal/modules/esm/`) |

rsvim and dune are the same design, one ported from the other. Ende and Node.js are the two escape
routes from that design: put the machine in the engine (Ende), or put it in JavaScript (Node).

---

## 0. One line each

- **dune** — module loading is *just another libuv resource*. A `handle.spawn` fetches the source on
  a rayon worker, a completion callback re-enters the loop, and a hand-written tree of `EsModule`
  nodes propagates "ready" upward one tick at a time.
- **rsvim** — the same tree and the same propagation, but the spawn channel is a tokio task plus two
  mpsc queues plus a `postcard` round trip, because in an editor every async operation already goes
  through that bus.
- **Ende** — module loading is *a stream of graph events* owned by `deno_core`. The embedder only
  implements `resolve`/`load`; `RecursiveModuleLoad` walks the graph, `ModuleMap::poll_progress`
  drives it, and V8's own evaluation promise decides when the import promise settles.
- **Node.js** — module loading is *JavaScript policy*. C++ is a synchronous trampoline that calls a
  JS async function; the ESM loader, the job cache, the hooks and the CJS bridge are all JS. Since
  PR #62530 (2026-10) the main-thread path is synchronous end to end, so only top-level await and
  the loader-hook worker are actually asynchronous.

---

## 1. What V8 actually gives you

V8's contract is deliberately thin, and everything in this document follows from it:

- The embedder installs `Isolate::SetHostImportModuleDynamicallyCallback` once
  (rsvim `js/hook.rs:181`, dune `src/hooks.rs:148`, Ende `libs/core/runtime/setup.rs:293`, Node
  `src/module_wrap.cc:1219`).
- The callback is **synchronous** and must return `MaybeLocal<Promise>`. V8's dynamic-import runtime
  call returns that promise verbatim as the value of the `import()` expression
  (`deps/v8/src/runtime/runtime-module.cc:28`), and V8 does not create, track or settle it.
- V8 passes `host_defined_options` (an embedder tag attached at compile time), the referrer name,
  the specifier, import attributes, and — in current V8 — an import phase. The tag is the stable way to identify the referrer when a module
  can be compiled more than once (Node's `vm` depends on it); dune and rsvim use the referrer name
  string instead.
- At instantiate time V8 asks the embedder to resolve each static request to a `v8::Module`
  (`module_resolve_cb` in rsvim `js/hook.rs:22`, dune `src/hooks.rs:18`), so the module graph is
  entirely the embedder's business.

Consequences, visible in all four implementations:

1. The promise is host-allocated, so "async import" is only as asynchronous as the host makes it.
2. Resolution *can* happen synchronously inside the callback (all four do it), which is why a
   bad specifier rejects without a loop turn.
3. Everything after resolution — fetching, compiling, recursing, readiness, settlement — is a state
   machine the host must build.

---

## 2. Four implementations

### 2.1 dune — the reference design

```
import("x")
  → host_import_module_dynamically_cb                     src/hooks.rs:148
      resolver := PromiseResolver::new                    src/hooks.rs:160
      specifier := resolve_import(import_map, ...)        src/hooks.rs:168   sync
      if by_path.contains && !being_fetched               src/hooks.rs:187   already evaluated
          resolver.resolve(module.get_module_namespace())
      if being_fetched                                    src/hooks.rs:200
          graph.same_origin.push_back(resolver)
      graph := ModuleGraph::dynamic_import                src/hooks.rs:216
      module_map.pending.push(graph); seen[specifier] = Fetching
      handle.spawn(task, Some(task_cb))                   src/hooks.rs:241
  → tick_event_loop                                       src/runtime.rs:427
      run_next_tick_callbacks()
      fast_forward_imports()                              src/runtime.rs:507
      event_loop.run(RunMode::Once)                       src/runtime.rs:430
      run_pending_futures()                               src/runtime.rs:479
  → EsModuleFuture::run                                   src/modules.rs:292
      compile_module → ModuleMap::insert → for each ModuleRequest
          if unseen: handle.spawn(load_import(child))     src/modules.rs:412
  → a later tick: fast_forward propagates Ready up the tree (src/modules.rs:182)
  → instantiate_module + evaluate → resolve the namespace (src/runtime.rs:598-608)
```

The interesting part is the substrate. `handle.spawn` belongs to dune's own loop (`crabuv`), which
runs the work closure on a **rayon** thread pool (`crabuv/src/thread_pool.rs:33`) and invokes the
completion callback back on the loop thread (`crabuv/src/event_loop.rs:732`). Because the callback
runs where V8 lives, the fetched source travels as a plain Rust value — `ModuleSource = String`
(`src/modules.rs:79`) boxed into `Box<dyn Any>` and downcast — with no serialization.

dune then applies the same pattern to `fs`, `dns` and `net` (`src/file.rs`, `src/dns.rs`): module
loading is not special. The module-specific part is only the bookkeeping:

- **Graph**: a hand-written tree (`EsModule { path, status, dependencies, exception,
  is_dynamic_import }`, `src/modules.rs:172`) plus a `seen: HashMap<path, ModuleStatus>`.
- **Readiness**: `fast_forward` (`src/modules.rs:182`) walks bottom-up and flips
  `Resolving → Ready` when all children are `Ready`; `Duplicate` nodes consult `seen`.
- **Coalescing**: concurrent imports of the same root share one graph through `same_origin`, a
  `LinkedList` of pending resolvers (`src/modules.rs:227`).
- **Settlement**: after `instantiate_module` + `evaluate`, the namespace is resolved into the graph's
  resolver and every `same_origin` resolver (`src/runtime.rs:598-608`).
- **Errors**: load/compile failures are stringified onto the tree node
  (`exception.replace(e.to_string())`, `src/modules.rs:283`). Static-import failures print and
  `exit(1)` (`src/modules.rs:288-289`).

Sources are fetched by a loader chosen at resolution time: core modules, filesystem, or URL
(`load_import`, `src/modules.rs:462`); the URL loader is blocking `ureq` plus a SHA-1 disk cache
(`src/loaders.rs:198-213`). The import map is real and sorted-prefix based (`src/modules.rs:484`).

### 2.2 rsvim — dune's design on a message bus

rsvim's module subsystem is a near-verbatim port of dune's, down to field names, comment text and
branch order:

| dune | rsvim |
|---|---|
| `src/modules.rs:82` `ModuleMap { main, by_path, by_id, seen, pending }` | `js/module/module_map.rs:147` |
| `src/modules.rs:152/160` `ImportKind`, `ModuleStatus` | `js/module/module_map.rs:49`, `js/module/module.rs:64` |
| `src/modules.rs:172/182` `EsModule`, `fast_forward` | `js/module/es_module.rs:21/86` |
| `src/modules.rs:227` `ModuleGraph { kind, root_rc, same_origin }` | `js/module/module_map.rs:58` (same, `Vec` instead of `LinkedList`) |
| `src/modules.rs:271` `EsModuleFuture` | `js/module/es_module.rs:162` |
| `src/hooks.rs:148-243` dynamic-import callback | `js/hook.rs:181-300` |
| `src/runtime.rs:427/479/507` tick, futures, fast-forward | `js.rs:610/648/776` |

The structural change is *where the fetched bytes come from*; the rest of the delta is substrate
and surroundings:

```
host_import_module_dynamically_cb                  rsvim_core/src/js/hook.rs:181
  ... same resolve / cache-hit / same_origin logic ...
  pending::create_import_loader                    rsvim_core/src/js/pending.rs:45
      pending_import_loaders[task_id] = cb
      send_to_master(MasterMessage::LoadImportReq)
→ EventLoop::handle                                 rsvim_core/src/evloop.rs:791
      detached_tracker.spawn(async { async_load_import(...) })
      async_load_import                             rsvim_core/src/js/module.rs:205
      send(JsMessage::LoadImportResp { postcard::to_allocvec(source) })
→ forward_js_message                                rsvim_core/src/evloop.rs:1321
      jsrt_tx.send(msg); js_runtime.tick_event_loop()
→ run_pending_futures                               rsvim_core/src/js.rs:648
      LoadImportResp → loader_cb → pending_futures.push(EsModuleFuture)
→ EsModuleFuture::run                               rsvim_core/src/js/es_module.rs:183
      compile → insert → for each request: create_import_loader(child)
```

Everything else — `fast_forward` (`js.rs:776`), the `seen`/`Duplicate` lattice, `same_origin`, the
settlement loop (`js.rs:836-905`) — is dune's code. The remaining deltas are substrate and
surroundings:

- **Two channels, not one.** Worker tasks send on `jsrt_forwarder_tx`; the select loop copies to
  `jsrt_tx` and ticks the runtime (`evloop.rs:1321`). The source comment calls channel-2 "simply for
  trigger the `tokio::select!` loop" (`evloop.rs:274-276`).
- **Serialization where dune has none.** dune's completion callback runs on the loop thread and moves
  a `String`; rsvim's task hands bytes across threads and pays `postcard` encode/decode because the
  message enum is the universal transport.
- **A different owner for the loader.** dune dispatches to `FsModuleLoader`/`UrlModuleLoader`/
  `CoreModuleLoader`; rsvim has `oxc_resolver` plus a filesystem loader with swc transpilation and a
  `node_modules` package resolver, an `import_map.rs` that is explicitly "just a mock-up"
  (`parse_from_json` returns empty, `lookup` returns `None`, `js/module/import_map.rs:35-41`), and a
  `url_loader.rs` that is commented out of the loader module (`js/loader.rs:5`).
- **Non-fatal errors.** Where dune prints and exits for a static import failure, rsvim routes
  `report_js_error` to the editor's command line (`js/err.rs:164` → `send_cmdline_message`).
- **Dead knobs.** dune's `skip_cache` (`--reload` / test mode, `src/modules.rs:344`) is commented
  out in rsvim (`js/module/es_module.rs:241-244`), together with the cache-busting it implies.

### 2.3 Ende — the engine owns the machine

```
import("x")
  → host_import_module_dynamically_callback          libs/core/runtime/bindings.rs:1163
      resolver := PromiseResolver::new
      ModuleMap::load_dynamic_import                 libs/core/modules/map/dynamic.rs:46
          fast path: already Evaluated               dynamic.rs:64   (or queue a TLA waiter)
          fast path: lazy ESM / synthetic ESM        dynamic.rs:109/133
          else: RecursiveModuleLoad                  recursive_load.rs:131
                dynamic_import_map[load_id] = resolver
                preparing_dynamic_imports.push(load.prepare())
                notify_new_dynamic_import()          jsruntime.rs:2955
      return promise.catch(catch_dynamic_import_promise_error)   bindings.rs:1317
  → poll_event_loop                                  libs/core/runtime/jsruntime.rs:2385
      ModuleMap::poll_progress                       dynamic.rs:435
          drain_prepare_dyn_imports                  dynamic.rs:483
          drain_dyn_imports                          dynamic.rs:511   register_and_recurse
          evaluate_dyn_imports                       dynamic.rs:300   promise.state()
  → dynamic_import_module_evaluate                   dynamic.rs:203
      module.evaluate() → pending_dyn_mod_evaluations
  → dynamic_import_resolve / reject                  dynamic.rs:394 / 373
```

`RecursiveModuleLoad` (`libs/core/modules/recursive_load.rs:76`) is a `Stream`: it resolves the root,
holds a `FuturesUnordered` of `loader.load()` futures, registers each module into the `ModuleMap` as
it arrives, and recurses into that module's requests (`Poll` at `recursive_load.rs:741`). There is no
second tree and no status enum — readiness is simply "the stream returned `None`".

The loader is an embedder trait (`libs/core/modules/loaders.rs:61`) whose `load` returns
`ModuleLoadResponse::Sync` or `::Async` (`loaders.rs:34`). Ende implements it in
`cli/module_loader.rs:1070` (with `load_inner` at `:576` doing the fetch/transpile path), so the
embedder never sees the graph.

Two details distinguish this design from dune/rsvim:

- **TLA is first class.** The evaluation promise is stored (`DynImportModEvaluate`,
  `dynamic.rs:30`), polled via `promise.state()`, and the user resolver settles only on
  `Fulfilled`; imports that arrive for a module already `Evaluated` but still running TLA are queued
  in `pending_tla_waiters` (`dynamic.rs:332`).
- **The promise V8 sees is a derived one.** `promise.catch(map_err)` (`bindings.rs:1317`) exists so
  resolution errors can be re-thrown with the stack of the `import()` site.

### 2.4 Node.js — the JS layer owns the machine

```
import("x")
  → Runtime_DynamicImportCall                        deps/v8/src/runtime/runtime-module.cc:28
  → Isolate::RunHostImportModuleDynamicallyCallback  deps/v8/src/execution/isolate.cc:7314
  → ImportModuleDynamicallyWithPhase                 src/module_wrap.cc:1126
      id := host_defined_options[kID]                module_wrap.cc:1155
      call JS importModuleDynamicallyCallback        module_wrap.cc:1175   (async fn)
      new Promise::Resolver ← resolve with the JS promise     module_wrap.cc:1184-1191
  → importModuleDynamicallyCallback                  lib/internal/modules/esm/utils.js:266
  → ModuleLoader.prototype.import                    lib/internal/modules/esm/loader.js:682
      getOrCreateModuleJob                           loader.js:647
      #getOrCreateModuleJobAfterResolve              loader.js:568
          loadCache.get(url, type)                   loader.js:588
          loadAndTranslate → #loadSync → defaultLoadSync → readFileSync   load.js:38
          new ModuleJobSync                          loader.js:620   (main thread)
      moduleJob.run()                                module_job.js:643
          instantiateSync → evaluateSync             module_job.js:633/697
          or await module.evaluate() if hasAsyncGraph (TLA)   module_job.js:671
      module.getNamespace()                          loader.js:699
```

Three properties make Node the outlier:

- **Double promise.** The C++ callback creates a fresh `Promise::Resolver` and resolves it with the
  JS async function's promise (`module_wrap.cc:1184-1191`); the value of `import()` is therefore not
  the loader's promise but a wrapper around it. For a TLA graph there is a third promise, V8's
  evaluation promise, which the loader awaits.
- **Sync on the main thread.** Since PR #62530 (`c89ec290f0f`, "module: synchronously load most ES
  modules"), `resolveSync`/`#loadSync` run inline: `defaultLoadSync` ends in `readFileSync`
  (`load.js:38`), linking is `ModuleJobBase.syncLink` (`module_job.js:312`), evaluation is
  `ModuleWrap::EvaluateSync` (`module_wrap.cc:915`). Async remains only for `hasAsyncGraph` (TLA)
  and for the loader-hook worker.
- **Policy in JS.** `loadCache` keyed by (url, attribute type) is the coalescing mechanism
  (`loader.js:588/634`); `module.registerHooks()` runs synchronous hooks in-thread, while
  `module.register()` keeps its async hooks on a dedicated worker (`hooks.js:528-538`) that the main
  thread drives with `AtomicsWait` + `receiveMessageSync` (`hooks.js:542-548`, `624-640`). CJS is
  bridged by synthesizing a namespace module with a static export list (`createCJSModuleWrap`,
  `translators.js:208`). `require(esm)` does not go through the callback at all: it reuses
  `ModuleJobSync` via `importSyncForRequire` (`loader.js:270`) and throws
  `ERR_REQUIRE_ASYNC_MODULE` for async graphs (`module_job.js:365`).

---

## 3. Side by side

### 3.1 Feature matrix

| | dune | rsvim | Ende | Node.js |
|---|---|---|---|---|
| specifier resolution | sync, in callback: import map → core/fs/url | sync, in callback: `oxc_resolver`, import map is a stub | sync, in callback: `ModuleMap::resolve` → loaders | sync on main thread: `resolveSync` → `defaultResolve` |
| source fetch | rayon task, blocking fs / HTTP | tokio task, async fs + swc | polled future: `ModuleLoader::load` (file/`npm:`/`jsr:`) | main thread, `readFileSync` |
| child loads | concurrent, one spawn each | concurrent, one message each | concurrent, `FuturesUnordered` | serial |
| dedupe | `seen` + `Duplicate` + `same_origin` | same | `visited` + shared `ModuleMap` | `loadCache` per (url, type) |
| top-level await | no (evaluation promise dropped) | no (same) | yes (`pending_dyn_mod_evaluations`) | yes (`hasAsyncGraph`) |
| import attributes | ignored (`_` parameter) | ignored | parsed | parsed |
| import phases | no | no | `defer`, `source` | `defer`, `source` |
| CJS | no | no | no | yes (synthetic namespace) |
| error value | `String` on the tree node | `String` on the tree node | `v8::Global<Value>` | JS object with `code` |

### 3.2 Mechanism matrix

| | dune | rsvim | Ende | Node.js |
|---|---|---|---|---|
| who allocates the promise | callback | callback | callback | callback, then C++ re-wraps it |
| what `import()` returns | the resolver's promise | the resolver's promise | `promise.catch(...)` | a new resolver promise fed by the JS fn |
| graph representation | `ModuleMap` + `EsModule` tree | same | `ModuleMap` + `RecursiveModuleLoad` stream | V8 `ModuleWrap` + JS `ModuleJob` |
| progress driven by | loop tick | loop tick, per forwarded message | `poll_event_loop` polling | microtasks (and sync work) |
| settled by | `fast_forward_imports` → resolve namespace | same | `evaluate_dyn_imports` → namespace | `ModuleLoader.import` return |
| referrer identity | referrer name | referrer name | referrer name (host-defined options for `vm`) | host-defined options (module symbol) |
| re-entrancy safety | borrow discipline | borrow discipline | explicit "no borrows held while resolving" comment | JS job cache |

### 3.3 Turn accounting

For a fresh file with no TLA:

| | loop turns before the promise settles | notes |
|---|---|---|
| dune | ≥ 1 per graph level | each level is discovered only after the parent compiles |
| rsvim | ≥ 1 tick per graph level, plus one per message | `fast_forward_imports` runs before the queue is drained |
| Ende | ≥ 1 poll per graph level | all ready work is drained in a `while has_evaluated` loop |
| Node.js | 0 turns | load + link + instantiate + evaluate happen inline; only microtasks remain |

Other observable differences:

- **Bad specifier**: dune/rsvim/Ende reject inside the callback (an already-rejected promise is
  returned); Node rejects from the async loader function.
- **Top-level throw**: dune/rsvim resolve the import anyway (see §5); Ende/Node reject.
- **Duplicate concurrent import**: dune/rsvim attach a resolver to `same_origin`; Ende uses one
  resolver per import but one module instance; Node returns the cached `ModuleJob`, so the second
  import awaits the first one's chain.

---

## 4. Four answers to one question

All four implementations share the same fan-out shape — resolve root, fetch, compile, discover
requests, repeat — and differ only in *where readiness propagation lives*:

- **a hand-written tree with a four-state lattice** (dune, rsvim). Every node knows its children and
  flips to `Ready` when they are; the tick loop walks the pending graphs and settles finished ones.
  This is the easiest thing to write from scratch, and it is exactly what dune did.
- **a per-graph stream** (Ende). There is no tree: `RecursiveModuleLoad` is the traversal, and the
  `ModuleMap` is the only registry. Completion is "the stream ended", nothing to poll for readiness.
- **a promise chain of JS jobs** (Node). `ModuleJob` memoizes itself in `loadCache`; `linked` and
  `instantiated` are promises, so cycles and duplicates are handled by promise identity, and the
  graph is V8's own.

The costs follow from that choice, not from V8:

- dune/rsvim buy **uniformity and control** — modules are just another async resource, the code has
  no framework dependency, and the same `spawn`/message path serves fs, dns, timers and imports.
  They pay with **module semantics re-implemented by hand**: TLA, evaluation errors, phases,
  attributes, code caches and "already loading" fast paths all have to be added. In rsvim's case the
  port also inherited dune's actual bugs, not just its design.
- Ende buys **the semantics** (TLA waiting, phases, lazy ESM and synthetic modules, error fidelity,
  `.catch`-based stack augmentation) at the price of a large runtime layer that tracks source formats,
  phases and code caches, plus an embedder trait that must expose sync-or-async loading per module.
- Node buys **maximum policy flexibility** (hooks in JS, per-scheme loaders, CJS bridging) at the
  price of re-entrant C++→JS calls, two or three promises per import, and — since #62530 — blocking
  file I/O on the main thread whenever hooks are not async.

A useful tell: the three hosts that must let *other code* decide where module bytes come from
(Ende for Deno's loaders, Node for user hooks) both moved the machine out of the embedder's hot
path; the two hosts that own the whole application (dune, an editor's script engine) kept it
inline.

---

## 5. What rsvim inherited from dune

These are code-level observations, not test results — the dynamic-import tests in
`module_map_tests.rs` cover only success paths (`failed1` exercises a `require` reference error,
`resolve_failed1` exercises `import.meta.resolve`).

1. **Top-level await is not awaited.** `let _ = module.evaluate(tc_scope)` (`js.rs:865`, dune
   `runtime.rs:579`) drops the evaluation promise, then the namespace is resolved immediately
   (`js.rs:895-905`). `await import("./tla.js")` resumes before the awaited module does, and a TLA
   rejection surfaces as an unhandled promise rejection instead of an import rejection.
   Fix: keep the promise on the graph node and poll `promise.state()` each tick — the shape
   `dynamic_import_module_evaluate` / `evaluate_dyn_imports` uses in Ende
   (`libs/core/modules/map/dynamic.rs:203/300`). rsvim's `pending_futures` (`JsFuture`) is the natural
   place to park such a poller, so this is a local change.

2. **Dynamic imports never reject on instantiate/evaluate errors.** The error branch is guarded by
   `is_root_module = !graph.root_rc().borrow().is_dynamic_import()` (`js.rs:872`), so a dynamic root
   that fails to instantiate takes `report_js_error` and `continue`s (`js.rs:853-863`), leaving the
   promise pending forever; a dynamic root that throws at evaluation falls through and *resolves*
   with the namespace. dune has the identical guard (`runtime.rs:580`). Fix: on either status,
   reject the graph's resolvers and drop the entry from `seen`.

3. **Errors are strings across the async boundary.** `EsModule::exception` is `Option<String>` and
   the rejection is rebuilt as `v8::Exception::error(String)` (`js.rs:799-802`), so `err.code` /
   `err.stack` from the loader are lost. Since V8 handles cannot cross threads, the portable fix is
   to send a structured error (`code`, `message`, `stack`) and rebuild it on the JS thread, the way
   `report_js_error` already formats `JsError`.

4. **One message and one `postcard` round trip per module.** dune's completion callback runs on the
   loop thread and moves the `String` directly; rsvim serializes it only because its channel payload
   is uniform. This is a cost choice, not a constraint: `String` is `Send`, so the payload could
   travel as-is, and batching child loads per graph would remove a message per module.

5. **dune features that did not survive the port.** `skip_cache` (dune `modules.rs:344`) is commented
   out (`es_module.rs:241-244`), the import map is a mock-up (`import_map.rs:35-41`), and the URL
   loader is commented out of the module tree (`js/loader.rs:5`). None of these are needed by an
   editor, but they are the difference between dune's loader and rsvim's if behaviour is ever
   compared.

---

## 6. Provenance / caveats

- All four trees were read at the commits listed at the top; **no runtime was built, no test was
  run**. Timing statements ("turns", "inline") are derived from control flow, not measurement.
- dune's `crabuv` was read at the revision pinned by its `Cargo.lock` (`2fda099`); the thread pool it
  uses is rayon (`crabuv/src/thread_pool.rs:33`), and completion callbacks are delivered on the loop
  thread (`crabuv/src/event_loop.rs:732`).
- V8-side citations for Node.js point into node's vendored `deps/v8` tree, so they describe the V8
  revision Node ships, not rsvim's `rusty_v8` 150.4.0. The V8 contract itself is stable across both.
- Ende is a Deno fork; `libs/core` is `deno_core`. Line numbers there move quickly upstream, and the
  `TODO(joyeecheung)`-style comments in Node's loader indicate the sync-loading design is still
  being consolidated.
- QuickJS-based runtimes (LLRT, txiki.js) get dynamic import from the engine
  (`js_dynamic_import` + a job queue in QuickJS-NG) and write no host code for it; they are covered
  in `JS_RUNTIME_COMPARISON.md` §3 and are intentionally out of scope here, where every runtime has
  to satisfy the V8 callback.
