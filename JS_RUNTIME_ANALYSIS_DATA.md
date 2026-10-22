# JS Runtime Analysis — Data & Session Handoff

A self-contained record of the analysis performed in a DeepSeek agent session on 2026-10-20/21:
what was asked, what was read, what was verified, what was concluded, and how to re-verify it
without the original conversation.

**Read this first in a new session.** It is deliberately redundant with the three architecture docs —
this file records *inputs, evidence and caveats*; the docs record *narrative*.

Companion documents (all in this repo):

| file | lines | content |
|---|---|---|
| `JS_RUNTIME_COMPARISON.md` | 700 | rsvim vs. dune vs. LLRT vs. txiki.js, broad architecture |
| `JS_RUNTIME_EVENT_LOOPS.md` | 397 | event-loop substrate deep dive, same four |
| `JS_RUNTIME_DYNAMIC_IMPORT.md` | 408 | dynamic `import()` trace: rsvim, dune, Ende, Node.js |
| `JS_RUNTIME_ANALYSIS_DATA.md` | this file | data, evidence index, session state, next steps |

Status at hand-off (HEAD `fd74024`): `JS_RUNTIME_COMPARISON.md` is tracked with a 3-line edit,
`JS_RUNTIME_EVENT_LOOPS.md` is tracked and unchanged, and both `JS_RUNTIME_DYNAMIC_IMPORT.md` and this
file are new and untracked. Nothing was built, benchmarked or executed in any of the six trees.

---

## 0. Resume prompt (paste into a fresh session)

> I keep a source-level analysis of JS runtime module loading in
> `/home/linrongbin/github/rsvim-dev/rsvim`. Read `JS_RUNTIME_ANALYSIS_DATA.md` first — it lists the
> pinned commits of the six local repos, the verified findings with `file:line` evidence, the
> uncommitted doc state, and the open questions. Then read `JS_RUNTIME_DYNAMIC_IMPORT.md` for the
> rsvim/dune/Ende/Node.js trace and `JS_RUNTIME_EVENT_LOOPS.md` for the loop substrates. Continue
> from §8 (open questions); re-verify any line number before citing it, since the trees may have
> moved.

---

## 1. Inputs: the six local trees

| repo | path | HEAD | date | role in this analysis |
|---|---|---|---|---|
| **rsvim** | `~/github/rsvim-dev/rsvim` | `fd74024` (0.1.3-alpha.2) | 2026-10-20 | subject; also where the docs live |
| **dune** | `~/github/aalykiot/dune` | `5958b64` (v0.11.4) | 2026-10-15 | the design rsvim's module loader was ported from |
| **Ende** | `~/github/theende/ende` | `edbd53a78` | 2026-10-20 | Deno fork; `libs/core` is `deno_core` |
| **Node.js** | `~/github/nodejs/node` | `14ed05f86bd` (v27.0.0, main) | 2026-10-21 | JS-layer ESM loader + V8 internals |
| **txiki.js** | `~/github/saghul/txiki.js` | `8a9b6044` | 2026-10-17 | QuickJS-NG: dynamic import comes from the engine |
| **LLRT** | `~/github/awslabs/llrt` | `d9fd873` | 2026-10-19 | QuickJS-NG via `rquickjs` 0.14 |

Tree quirks worth remembering:

- **txiki.js**: all `deps/*` submodules are **not checked out** (`git submodule status` shows a `-`
  prefix for `deps/quickjs` at `10b99693`), so QuickJS-ng source is not available there. Call sites
  and tests still prove dynamic import works.
- **Ende**: `Cargo.toml` depends on `deno_ast`, `deno_core_icudata`, `deno_doc`, `deno_error`;
  `libs/core/**` headers read "Copyright 2018-2026 the Deno authors". Treat it as a Deno fork.
- **LLRT**: pins `rquickjs` 0.14.0 (`Cargo.lock:3129`), which vendors QuickJS-NG 0.16.2; that
  vendored source was **not** in the local crates cache. The cache has `rquickjs-sys` 0.12.2, whose
  vendored QuickJS-NG was used as a stand-in for engine behaviour.
- **Node.js**: `src/node_version.h` says 27.0.0. PR #62530 (commit `c89ec290f0f`, 2026-10-06,
  "module: synchronously load most ES modules") is on `main` in this tree; it may not be in a
  released Node version.

---

## 2. Environment facts (re-learned the hard way)

- Sandbox is **workspace-write** with the workspace at `~/github/rsvim-dev/rsvim`: writes outside are
  denied, reads outside are allowed. All six trees were read, none modified.
- **Each `bash` call gets a fresh `/tmp`.** A directory extracted to `/tmp` in one call is gone in the
  next. Extract-and-use in a single command, or extract under the workspace (and delete afterwards —
  a `.tmp_qjs/` scratch dir was created and removed this way).
- Network: `https://static.crates.io/` is reachable (a bare `curl -I` returns 403, which is normal
  for the bucket root). `npm install`-style writes fail with `EROFS`, so no `markdownlint`/`prettier`
  could be run; formatting was checked by hand (fences, trailing whitespace, table shape).
- Rust toolchain is present (`~/.cargo/bin/cargo`, `rustc`), and rsvim has prebuilt artifacts under
  `target/`, but **nothing was compiled or run** for this analysis (V8 builds are expensive).
- `~/.cargo/git/checkouts/crabuv-111f09bf8eabb1d0/2fda099` holds dune's loop crate; read it at that
  revision (`crabuv` is pinned in dune's `Cargo.lock`).

---

## 3. Session log

1. **Round 1 — claim check.** The user stated that txiki.js and LLRT "have not implemented async
   `import()`, only static import". Verification (§4.6) showed both **do** support `import()`; what
   they lack is *host-side* implementation, because QuickJS-NG does it. The user moved on before the
   reply was delivered.
2. **Round 2 — four-way comparison.** Asked to compare rsvim, Ende and Node.js `import()` at the
   architecture level. Traced all three to promise settlement; produced a chat-level comparison.
   Node.js was delegated to a subagent, then spot-checked against the tree.
3. **Round 3 — dune.** The user revealed that rsvim's implementation was based on dune. Reading dune
   confirmed a near-verbatim port (structs, functions, comments); the delta is the async substrate.
4. **Round 4 — write-up.** `JS_RUNTIME_DYNAMIC_IMPORT.md` was written (408 lines), every citation
   re-verified against the trees (several line numbers in the first draft were off by 2-20 and were
   corrected), and a 3-line cross-reference was added to `JS_RUNTIME_COMPARISON.md` §3.3.

---

## 4. Findings (verified)

Every claim below was read from source at the pinned commits. "Verified" means *read*, not *executed*.

### 4.1 The V8 contract (the reason all four look similar)

- The embedder installs `SetHostImportModuleDynamicallyCallback` once:
  rsvim `rsvim_core/src/js/hook.rs:181`, dune `src/hooks.rs:148`, Ende
  `libs/core/runtime/setup.rs:293`, Node `src/module_wrap.cc:1219`.
- The callback is synchronous and must return `MaybeLocal<Promise>`; V8 returns it **verbatim** as the
  value of `import()` (`deps/v8/src/runtime/runtime-module.cc:28`, via
  `deps/v8/src/execution/isolate.cc:7314`). V8 never allocates or settles that promise.
- `host_defined_options` is a compile-time embedder tag: Node uses it to recover the referrer module
  symbol (`src/module_wrap.cc:1155`); `deno_core` uses it for `vm` dynamic-import callbacks
  (`libs/core/runtime/bindings.rs:1195-1216`); dune and rsvim ignore it (`_host_defined_options`).
- Static requests are resolved through a second callback at instantiate time:
  rsvim `js/hook.rs:22`, dune `src/hooks.rs:18`.

### 4.2 dune — the reference design

- Flow: `src/hooks.rs:148` → resolver `:160` → `resolve_import` `:168` → already-loaded fast path
  `:187` → `same_origin` merge `:200` → `ModuleGraph::dynamic_import` `:216` →
  `handle.spawn(task, Some(task_cb))` `:241`.
- `handle.spawn` is crabuv's: work runs on a **rayon** pool (`crabuv/src/thread_pool.rs:33`),
  completion callback runs on the **loop thread** (`crabuv/src/event_loop.rs:732`); the fetched
  `ModuleSource = String` (`src/modules.rs:79`) crosses as `Box<dyn Any>` — no serialization.
- Bookkeeping: `ModuleMap { main, by_path, by_id, seen, pending }` `src/modules.rs:82`;
  `EsModule` `:172`; `fast_forward` `:182`; `ModuleGraph { kind, root_rc, same_origin }` `:227`;
  `EsModuleFuture` `:271`; child spawn `:412`.
- Tick order: `src/runtime.rs:427-432` = nextTick → `fast_forward_imports` (`:507`) →
  `event_loop.run(RunMode::Once)` → `run_pending_futures` (`:479`).
- Settlement: `instantiate_module` + `evaluate` then resolve the namespace into the graph resolver and
  every `same_origin` resolver (`src/runtime.rs:598-608`).
- TLA is not awaited: `let _ = module.evaluate(...)` (`src/runtime.rs:579`); the error branch is
  guarded by `is_root_module` (`:580`), so dynamic roots never reject; errors are strings
  (`src/modules.rs:283`), and static-import failures `exit(1)` (`:288-289`).
- Loaders: `src/loaders.rs` — fs, core modules, and a blocking `ureq` + SHA-1 cached URL loader
  (`:198-213`); import map is real (`src/modules.rs:484`); `skip_cache` for `--reload`/test mode
  (`src/modules.rs:344`).

### 4.3 rsvim — dune's design over a message bus

Port evidence (dune → rsvim): `modules.rs:82` → `js/module/module_map.rs:147`;
`modules.rs:152/160` → `module_map.rs:49` / `js/module/module.rs:64`;
`modules.rs:172/182` → `js/module/es_module.rs:21/86`; `modules.rs:227` → `module_map.rs:58`;
`modules.rs:271` → `es_module.rs:162`; `hooks.rs:148-243` → `js/hook.rs:181-300`;
`runtime.rs:427/479/507` → `js.rs:610/648/776`. Comments are copied too (namespace-object note,
nextTick-checkpoint note).

Delta:

- Loading goes through the editor's message bus: `pending::create_import_loader`
  (`js/pending.rs:45`) → `MasterMessage::LoadImportReq` → `EventLoop` spawns a tokio task
  (`evloop.rs:791`) → `async_load_import` (`js/module.rs:205`) → `postcard` bytes →
  `JsMessage::LoadImportResp` → `forward_js_message` calls `tick_event_loop` (`evloop.rs:1321`) →
  `run_pending_futures` (`js.rs:648`) → `EsModuleFuture::run` (`es_module.rs:183`).
- Two channels (`jsrt_forwarder_tx` → select loop → `jsrt_tx`) exist only to wake `tokio::select!`
  (`evloop.rs:274-276` comment admits it).
- Loader: `oxc_resolver` + fs loader + swc transpile; import map is a stub
  (`js/module/import_map.rs:35-41`); URL loader commented out of the module tree (`js/loader.rs:5`).
- Errors are non-fatal (`report_js_error` → cmdline, `js/err.rs:164`), unlike dune's `exit(1)`.
- `skip_cache` logic is commented out (`js/module/es_module.rs:241-244`).
- Inherited gaps: `js.rs:865` drops V8's evaluation promise (TLA); `js.rs:872` guards error handling
  with `is_root_module`, so dynamic roots never reject; instantiate failure `continue`s at
  `js.rs:853-863` leaving the promise pending; errors are `Option<String>` rebuilt at `js.rs:799-802`.

### 4.4 Ende — the engine owns the machine

- `libs/core/runtime/bindings.rs:1163` callback → `ModuleMap::load_dynamic_import`
  (`libs/core/modules/map/dynamic.rs:46`); fast paths: already-evaluated/TLA waiters `:64`,
  lazy ESM `:109`, synthetic ESM `:133`; otherwise `RecursiveModuleLoad::new_dynamic_import`
  (`recursive_load.rs:131`), `dynamic_import_map[load_id]`, `preparing_dynamic_imports`,
  `notify_new_dynamic_import` (`jsruntime.rs:2955`); returns `promise.catch(...)` (`bindings.rs:1317`).
- Progress: `poll_event_loop` (`jsruntime.rs:2385`) → `poll_progress` (`dynamic.rs:435`) →
  `drain_prepare` `:483` / `drain_loads` `:511` / `evaluate_dyn_imports` `:300` (polls
  `promise.state()`); `dynamic_import_module_evaluate` `:203` stores V8's evaluation promise;
  resolve/reject `:394/:373`; TLA waiters `:332`.
- `RecursiveModuleLoad` (`recursive_load.rs:76`) is a `Stream` over `FuturesUnordered` of
  `loader.load()` futures (`poll_next` at `:741`); `ModuleLoader` trait at
  `libs/core/modules/loaders.rs:61` with `ModuleLoadResponse::Sync|Async` (`:34`); Ende's loader
  `cli/module_loader.rs:1070`, `load_inner` `:576`.
- Consequences: TLA correct, phases (`defer`/`source`) supported, exceptions preserved as
  `v8::Global<Value>`, one module instance shared across imports but one resolver per load id.

### 4.5 Node.js — the JS layer owns the machine

- `src/module_wrap.cc:1126` reads the referrer symbol from host-defined options (`:1155`), calls the
  JS callback synchronously (`:1175`), then **re-wraps** the result in a fresh `Promise::Resolver`
  (`:1184-1191`); registration at `:1219`. JS side: `lib/internal/modules/esm/utils.js:266`,
  registered at `:312`.
- `ModuleLoader.import` (`loader.js:682`) → `getOrCreateModuleJob` (`:647`) →
  `#getOrCreateModuleJobAfterResolve` (`:568`) → `loadCache.get(url, type)` (`:588`) →
  `loadAndTranslate` (`:538`) → `#loadSync` (`:869`) → `defaultLoadSync` → **`readFileSync`**
  (`load.js:38`) → translator → `ModuleJobSync` selected at `loader.js:620` (class at `module_job.js:595`).
- Link/evaluate: `syncLink` (`module_job.js:312`), `instantiateSync` (`:633`), `run` (`:643`),
  `evaluateSync` (`:697`); only `hasAsyncGraph` (TLA) goes async (`:671`).
- Since PR #62530 (`c89ec290f0f`, 2026-10-06) the main-thread path is synchronous end to end; async
  remains only for TLA and the loader-hook worker.
- Hooks: `module.registerHooks()` synchronous in-thread; `module.register()` async hooks on a
  dedicated worker (`hooks.js:528-538`), main thread blocks via `AtomicsWait`/`receiveMessageSync`
  (`:542-548`, `:624-640`).
- CJS: synthetic namespace module (`translators.js:208`). `require(esm)` bypasses the import
  callback: `importSyncForRequire` (`loader.js:270`), `ERR_REQUIRE_ASYNC_MODULE`
  (`module_job.js:365`).

### 4.6 QuickJS runtimes (round 1)

- **txiki.js** supports `import()`: `JS_SetModuleLoaderFunc2` at `src/vm.c:550`; tests use it at
  `tests/test-import-map.js:13/24`, `tests/test-import-unknown-builtin-dynamic.js:6`,
  `tests/test-import-http.js:8`, `tests/test-worker-ffi-classes.js:43`.
- **LLRT** supports `import()`: its entry point is literally
  `import("<file>").catch(...)` (`llrt_core/src/vm.rs:170-179`); tests at
  `tests/unit/import.test.ts:5/25/31/39/57`; the loader impl is `llrt_modules/src/module/loader.rs:45`
  (`impl Loader for ModuleLoader`).
- Engine side: QuickJS-NG has `js_dynamic_import` (`quickjs/quickjs.c:1397`) that creates a promise and
  enqueues `js_dynamic_import_job` (`:31060`, `JS_EnqueueJob` at `:31182`); loader installed via
  `JS_SetModuleLoaderFunc2` (`:29893`).
- Evidence source for the engine: `~/.cargo/registry/cache/index.crates.io-*/rquickjs-sys-0.12.2.crate`
  (extracted in one command; `/tmp` does not persist between calls).

### 4.7 Condensed comparison (details in `JS_RUNTIME_DYNAMIC_IMPORT.md` §3)

| | dune | rsvim | Ende | Node.js |
|---|---|---|---|---|
| state machine | embedder tree | embedder tree (port) | deno_core stream | JS loader |
| promise seen by V8 | resolver's | resolver's | `promise.catch(...)` | C++ re-wrap |
| fetch | rayon, blocking | tokio, async | polled future | `readFileSync` |
| TLA | no | no | yes | yes |
| child-load concurrency | yes | yes | yes | no |
| error value | string | string | V8 exception | JS error + code |

---

## 5. Conclusions (derived, not quoted from any source)

1. All four implementations share one fan-out shape (resolve → fetch → compile → discover → repeat);
   they differ only in where readiness propagation lives: a tree (dune/rsvim), a stream (Ende), or a
   promise chain (Node).
2. rsvim's module loader is dune's code; the only real change is the async substrate
   (crabuv/rayon/typed-callback → tokio/mpsc/postcard/`task_id`). The `same_origin`, `seen`,
   `Duplicate`, `fast_forward` machinery is unchanged.
3. Ende is the "framework" answer: `RecursiveModuleLoad` + `poll_progress` absorb the module
   semantics (TLA, phases, error fidelity, code cache) that dune/rsvim re-implement by hand.
4. Node is the "policy in JS" answer, and since #62530 it has become the "sync loading" answer:
   `import()` is a promise boundary over synchronous I/O, with async only for TLA and the hook worker.
5. For rsvim the practical consequence is that its defects are *inherited structural* ones (TLA,
   dynamic-root rejection, string errors), not incidental bugs — fixing them is a localized change
   in `js.rs`/`es_module.rs`, not a redesign.

---

## 6. rsvim gaps and fix sketches (from `JS_RUNTIME_DYNAMIC_IMPORT.md` §5)

1. **TLA**: keep `module.evaluate()`'s promise, poll `promise.state()` per tick, settle on
   `Fulfilled` — mirror `dynamic.rs:203/300`; `pending_futures` (`JsFuture`) is the place to park it.
2. **Dynamic-root rejection**: remove the `is_root_module` guard for dynamic graphs; on instantiate
   failure reject the graph's resolvers instead of `continue`-ing (`js.rs:853-863`), and drop the
   `seen` entry so a retry re-fetches.
3. **Error fidelity**: transport a structured `{code, message, stack}` instead of `String`
   (V8 handles cannot cross threads), rebuild with the existing `JsError` machinery.
4. **Transport cost**: `postcard` is a choice, not a constraint (`String` is `Send`); batching child
   load requests per graph would remove one message per module.
5. **Dead dune knobs**: `skip_cache`, real import map, URL loader — none needed by an editor, but they
   explain dune/rsvim loader divergence.

---

## 7. Corrections and falsified premises (keep these; they are easy to re-introduce)

- **"txiki.js and LLRT have no async `import()`."** False at the JS level (§4.6). True only in the
  narrow sense that neither writes host-side dynamic-import code — QuickJS-NG does it.
- **Node symbol names.** `node::loader::ImportModuleDynamically`, `src/node_module_loader.*`,
  `setupModuleLoader`, `ModuleLoader#getModuleJob`, `#getModuleJobForImport` and a `ModuleMap` class
  **do not exist** at `14ed05f86bd`. The real names are `ModuleWrap::ImportModuleDynamically`,
  `getOrCreateModuleJob`, `#getOrCreateModuleJobAfterResolve`, `LoadCache`/`ResolveCache`
  (`lib/internal/modules/esm/module_map.js`), `ModuleJobSync`/`ModuleJob`.
- **Line numbers drifted while writing.** First-draft citations were approximate; all were re-pinned.
  Examples of corrections: dune `hooks.rs:162→160`, `:174→168`, `:190→187`, `:207→200`, `:221→216`;
  Ende `dynamic.rs:50→46`, `:317→300`, `:419→435`, `:472→483`, `:518→511`;
  rsvim `ModuleStatus` is `js/module/module.rs:64` (not `es_module.rs`). Re-verify before citing.
- **Node's sync loading is new.** `ModuleJobSync` and `readFileSync` on the main thread are the state
  of `main` at this commit; older Node versions used async `ModuleJob` for everything.

---

## 8. Open questions / suggested next steps

1. **Add rsvim tests** for `import()` of a throwing module, an unresolved specifier, a missing named
   export, and a TLA module. Current tests only cover success paths (`module_map_tests.rs`;
   `failed1` tests `require`, `resolve_failed1` tests `import.meta.resolve`). This would turn §6.1/6.2
   from predictions into observations.
2. **Implement the TLA poller** (§6.1) — smallest change with the largest semantic gain.
3. Decide whether to commit the docs: `JS_RUNTIME_DYNAMIC_IMPORT.md` is untracked and
   `JS_RUNTIME_COMPARISON.md` has a 3-line edit.
4. Optionally fold the QuickJS note into `JS_RUNTIME_COMPARISON.md` §3.1: LLRT/txiki dynamic import is
   engine-provided, not host-implemented. The current ✅✅ row is correct but under-explained.
5. No numbers exist for startup/memory/per-import latency; all "turn" statements are control-flow
   reasoning. Any benchmarking would be new work.

---

## 9. Reproduction recipes

```bash
# 1. Confirm the six HEADs and dates
for r in rsvim-dev/rsvim aalykiot/dune theende/ende nodejs/node saghul/txiki.js awslabs/llrt; do
  git -C ~/github/$r log -1 --format='%h %ad %s' --date=short
done

# 2. dune: the spawn/rayon loop
grep -n "state.handle.spawn" ~/github/aalykiot/dune/src/hooks.rs
grep -n "pub fn spawn" ~/.cargo/git/checkouts/crabuv-*/2fda099/src/event_loop.rs
grep -n "ThreadPoolBuilder" ~/.cargo/git/checkouts/crabuv-*/2fda099/src/thread_pool.rs

# 3. rsvim: the message-bus detour
grep -n "host_import_module_dynamically_cb" ~/github/rsvim-dev/rsvim/rsvim_core/src/js/hook.rs
grep -n "LoadImportReq" ~/github/rsvim-dev/rsvim/rsvim_core/src/evloop.rs
grep -n "LoadImportResp" ~/github/rsvim-dev/rsvim/rsvim_core/src/js.rs

# 4. Ende: stream + poll
grep -n "fn load_dynamic_import\|fn poll_progress\|fn evaluate_dyn_imports" \
  ~/github/theende/ende/libs/core/modules/map/dynamic.rs
grep -n "impl Stream for RecursiveModuleLoad" \
  ~/github/theende/ende/libs/core/modules/recursive_load.rs

# 5. Node: sync loading
grep -n "ModuleJobSync\|readFileSync" ~/github/nodejs/node/lib/internal/modules/esm/loader.js \
  ~/github/nodejs/node/lib/internal/modules/esm/load.js
git -C ~/github/nodejs/node log --oneline -3 -- lib/internal/modules/esm/module_job.js

# 6. QuickJS-NG dynamic import (single command; /tmp is not persistent)
W=$(mktemp -d) && tar xf ~/.cargo/registry/cache/index.crates.io-*/rquickjs-sys-0.12.2.crate -C "$W" \
  && grep -n "js_dynamic_import" "$W"/rquickjs-sys-0.12.2/quickjs/quickjs.c | head

# 7. txiki.js submodules are not checked out
git -C ~/github/saghul/txiki.js submodule status deps/quickjs
```

---

## 10. Key-file index (what to read for what)

| question | file | lines of interest |
|---|---|---|
| dune callback + graph | `dune/src/hooks.rs`, `dune/src/modules.rs` | 148-243 / 79-283, 344-412 |
| dune tick + settle | `dune/src/runtime.rs` | 427-432, 479, 507, 566-608 |
| rsvim callback | `rsvim_core/src/js/hook.rs` | 181-300 |
| rsvim graph/compile | `rsvim_core/src/js/module/es_module.rs`, `module_map.rs` | 86, 162-322 / 49-160 |
| rsvim tick/settle | `rsvim_core/src/js.rs` | 610, 648, 713, 776, 836-905 |
| rsvim bus | `rsvim_core/src/evloop.rs`, `js/pending.rs`, `js/module.rs` | 791, 1321 / 45 / 205 |
| Ende bindings | `ende/libs/core/runtime/bindings.rs` | 1163-1327 |
| Ende dynamic map | `ende/libs/core/modules/map/dynamic.rs` | 30-520 |
| Ende stream | `ende/libs/core/modules/recursive_load.rs` | 76-180, 741-780 |
| Ende loader impl | `ende/cli/module_loader.rs` | 576, 1048-1110 |
| Node C++ bridge | `node/src/module_wrap.cc` | 1126-1226 |
| Node JS loader | `node/lib/internal/modules/esm/loader.js` | 538-705, 869-893 |
| Node job | `node/lib/internal/modules/esm/module_job.js` | 312-351, 595-700 |
| Node hooks worker | `node/lib/internal/modules/esm/hooks.js` | 528-548, 590-640 |
| QuickJS engine | `rquickjs-sys-0.12.2/quickjs/quickjs.c` | 1397, 29893, 31060-31190 |

---

## 11. Artifacts and repo state

- Written: `JS_RUNTIME_DYNAMIC_IMPORT.md` (408 lines, untracked).
- Edited: `JS_RUNTIME_COMPARISON.md` (3 added lines in §3.3 pointing at the new doc).
- Tracked before this session: `JS_RUNTIME_COMPARISON.md` (697 lines, now 700), `JS_RUNTIME_EVENT_LOOPS.md`
  (397 lines, unchanged). Both landed in commit `fd74024` ("docs: write down js runtime architecture
  document"), i.e. they are the output of an earlier session and should be treated as authoritative
  prior art rather than re-derived.
- Scratch dir `.tmp_qjs/` created under the repo during extraction was removed; tree is otherwise
  untouched.
- No commits were made. `git status` at hand-off:
  `M JS_RUNTIME_COMPARISON.md`, `?? JS_RUNTIME_DYNAMIC_IMPORT.md`, `?? JS_RUNTIME_ANALYSIS_DATA.md`.

---

## 12. Working notes for the next session

- The user writes Chinese, but the repo docs are English; keep new docs English and reply in Chinese.
- Preferred evidence style: exact `file:line`, tables, an explicit "verified by reading vs. measured"
  split, and no claims that cannot be traced to a line. The comparison docs use
  `---`-separated numbered sections and a provenance/caveats section at the end.
- Reads outside the workspace are allowed; writes are not. `/tmp` does not persist between commands.
- Keep this file updated when findings change — it is the only thing that survives a session reset.
