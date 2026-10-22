# 动态 `import()`：rsvim vs. dune vs. Ende vs. Node.js

本文在源码层面追踪一个 V8 嵌入方回调 —— `HostImportModuleDynamicallyCallback` —— 穿过四个都实现了它的运行时的路径，并给出对同一个问题的四种不同回答：*一旦 V8 把一个说明符交到你手上、并期望收回一个 promise，那么填满这个 promise 的状态机归谁所有？*

本文是 `JS_RUNTIME_COMPARISON.md` 与 `JS_RUNTIME_EVENT_LOOPS.md` 的模块加载姊妹篇；三者共享同一套源码级、只读的方法论。

分析的快照（本地工作树，只读；**没有构建或执行任何东西**）：

| | 仓库 | HEAD | 引擎 | 状态机所在位置 |
|---|---|---|---|---|
| **rsvim** | `rsvim-dev/rsvim` | `fd74024` (0.1.3-alpha.2) | V8 150.4.0 | 嵌入方，手写（`rsvim_core/src/js/`） |
| **dune** | `aalykiot/dune` | `5958b64` (v0.11.4) | V8 150.2.0 | 嵌入方，手写（`src/`） |
| **Ende** | `theende/ende` | `edbd53a78` | V8 via `ende_core` | 引擎层（`libs/core/`），嵌入在一个 Deno fork 中 |
| **Node.js** | `nodejs/node` | `14ed05f86bd` (v27.0.0-pre) | V8 via `deps/v8` | JS 层（`lib/internal/modules/esm/`） |

rsvim 与 dune 是同一套设计，其中一个由另一个移植而来。Ende 与 Node.js 则是逃离那套设计的两个出口：把状态机放进引擎（Ende），或者放进 JavaScript（Node）。

---

## 0. 每个运行时一句话

- **dune** —— 模块加载*不过是又一种 libuv 资源*。`handle.spawn` 在 rayon 工作线程上抓取源码，完成回调重新进入循环，一棵手写的 `EsModule` 节点树则一个 tick 一个 tick 地把 "ready" 向上传播。
- **rsvim** —— 同样的树、同样的传播，但 spawn 通道换成了 tokio 任务加两个 mpsc 队列加一次 `postcard` 往返，因为在编辑器里，每个异步操作本来就要走这条总线。
- **Ende** —— 模块加载是*一串由 `deno_core` 拥有的模块图事件*。嵌入方只实现 `resolve`/`load`；`RecursiveModuleLoad` 遍历模块图，`ModuleMap::poll_progress` 驱动它，而 import promise 何时敲定由 V8 自己的求值 promise 决定。
- **Node.js** —— 模块加载是*JavaScript 策略*。C++ 只是一个同步蹦床，负责调用一个 JS 异步函数；ESM 加载器、作业缓存、钩子以及 CJS 桥接全都在 JS 里。自 PR #62530（2026-10）起，主线程路径端到端同步，因此真正异步的只剩顶层 await 和加载器钩子工作线程。

---

## 1. V8 究竟给了你什么

V8 的契约刻意做得很薄，本文中的一切都由它推出：

- 嵌入方安装一次 `Isolate::SetHostImportModuleDynamicallyCallback`（rsvim `js.rs:149`，dune `src/runtime.rs:151`，Ende `libs/core/runtime/setup.rs:293`，Node `src/module_wrap.cc:1219`）。
- 该回调是**同步的**，必须返回 `MaybeLocal<Promise>`。V8 的动态 import 运行时调用会把该 promise 原样作为 `import()` 表达式的值返回（`deps/v8/src/runtime/runtime-module.cc:28`），V8 既不创建、也不跟踪、更不敲定它。
- V8 会传入 `host_defined_options`（编译时附加的嵌入方标记）、引用方名称、说明符、导入属性，以及在当前 V8 中还会传入导入阶段。当一个模块可能被编译不止一次时，该标记是识别引用方的稳定方式（Node 的 `vm` 依赖它）；dune 与 rsvim 则改用引用方名称字符串。
- 在实例化阶段，V8 要求嵌入方把每个静态请求解析为一个 `v8::Module`（`module_resolve_cb`，见 rsvim `js/hook.rs:22`、dune `src/hooks.rs:18`），因此模块图完全是嵌入方自己的事。

以下后果在四个实现中都可见：

1. 该 promise 由宿主分配，所以 "async import" 的异步程度完全取决于宿主让它有多异步。
2. 解析*可以*在回调内部同步完成（四个实现都这么做），这正是坏说明符的拒绝不需要绕一圈事件循环的原因。
3. 解析之后的一切 —— 抓取、编译、递归、就绪、敲定 —— 都是宿主必须自己搭起来的状态机。

---

## 2. 四个实现

### 2.1 dune —— 参考设计

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
  → EsModuleFuture::run                                   src/modules.rs:294
      compile_module → ModuleMap::insert → for each ModuleRequest
          if unseen: handle.spawn(load_import(child))     src/modules.rs:412
  → a later tick: fast_forward propagates Ready up the tree (src/modules.rs:182)
  → instantiate_module + evaluate → resolve the namespace (src/runtime.rs:569-608)
```

有意思的是底层。`handle.spawn` 属于 dune 自己的循环（`crabuv`），它把工作闭包放到 **rayon** 线程池上执行（`crabuv/src/thread_pool.rs:33`），并回到循环线程上调用完成回调（`crabuv/src/event_loop.rs:357`）。由于回调运行在 V8 所在的线程，抓取到的源码以一个普通 Rust 值的形式传递 —— `ModuleSource = String`（`src/modules.rs:79`），装箱成 `Box<dyn Any>` 再 downcast —— 无需序列化。

dune 随后把同一套模式套用到 `fs`、`dns` 和 `net`（`src/file.rs`、`src/dns.rs`）：模块加载并不特殊。模块特有的部分只是这些簿记：

- **模块图**：一棵手写的树（`EsModule { path, status, dependencies, exception, is_dynamic_import }`，`src/modules.rs:172`）外加一个 `seen: HashMap<path, ModuleStatus>`。
- **就绪**：`fast_forward`（`src/modules.rs:182`）自底向上遍历，当所有子节点都 `Ready` 时翻转 `Resolving → Ready`；`Duplicate` 节点则查询 `seen`。
- **合并**：对同一根的并发 import 通过 `same_origin`（一个由待决 resolver 组成的 `LinkedList`）共享一张图（`src/modules.rs:227`）。
- **敲定**：在 `instantiate_module` + `evaluate` 之后，命名空间被兑现进该图的 resolver 以及每一个 `same_origin` resolver（`src/runtime.rs:569-608`）。
- **错误**：加载/编译失败会被字符串化后写进树节点（`exception.replace(e.to_string())`，`src/modules.rs:283`）。静态 import 失败会打印并 `exit(1)`（`src/modules.rs:288-289`）。

源码由解析时选定的加载器抓取：核心模块、文件系统或 URL（`load_import`，`src/modules.rs:462`）；URL 加载器是基于阻塞 `ureq` 加 SHA-1 磁盘缓存（`src/loaders.rs:198-213`）。导入映射（import map）是真实存在的，并基于排序后的前缀（`src/modules.rs:484`）。

### 2.2 rsvim —— 跑在消息总线上的 dune 设计

rsvim 的模块子系统几乎是 dune 的逐字移植，连字段名、注释文本和分支顺序都一致：

| dune | rsvim |
|---|---|
| `src/modules.rs:82` `ModuleMap { main, by_path, by_id, seen, pending }` | `js/module/module_map.rs:147` |
| `src/modules.rs:152/160` `ImportKind`, `ModuleStatus` | `js/module/module_map.rs:49`, `js/module.rs:64` |
| `src/modules.rs:172/182` `EsModule`, `fast_forward` | `js/module/es_module.rs:21/86` |
| `src/modules.rs:227` `ModuleGraph { kind, root_rc, same_origin }` | `js/module/module_map.rs:58`（相同，只是用 `Vec` 而非 `LinkedList`） |
| `src/modules.rs:271` `EsModuleFuture` | `js/module/es_module.rs:162` |
| `src/hooks.rs:148-243` 动态 import 回调 | `js/hook.rs:181-295` |
| `src/runtime.rs:427/479/507` tick、futures、fast-forward | `js.rs:610/648/776` |

结构性变化在于*抓取到的字节来自哪里*；其余的差异属于底层与周边环境：

```
host_import_module_dynamically_cb                  rsvim_core/src/js/hook.rs:181
  ... same resolve / cache-hit / same_origin logic ...
  pending::create_import_loader                    rsvim_core/src/js/pending.rs:45
      pending_import_loaders[task_id] = cb
      send_to_master(MasterMessage::LoadImportReq)
→ EventLoop::process_master_message                   rsvim_core/src/evloop.rs:791
      detached_tracker.spawn(async { async_load_import(...) })
      async_load_import                             rsvim_core/src/js/module.rs:205
      send(JsMessage::LoadImportResp { postcard::to_allocvec(source) })
→ forward_js_message                                rsvim_core/src/evloop.rs:1321
      jsrt_tx.send(msg); js_runtime.tick_event_loop()
→ run_pending_futures                               rsvim_core/src/js.rs:648
      LoadImportResp → loader_cb → pending_futures.push(EsModuleFuture)
→ EsModuleFuture::run                               rsvim_core/src/js/es_module.rs:185
      compile → insert → for each request: create_import_loader(child)
```

其余的一切 —— `fast_forward`（`js.rs:776`）、`seen`/`Duplicate` 状态格、`same_origin`、敲定循环（`js.rs:836-905`）—— 都是 dune 的代码。剩下的差异是底层与周边环境：

- **两条通道，而非一条。** 工作线程任务在 `jsrt_forwarder_tx` 上发送；select 循环把消息复制到 `jsrt_tx` 并 tick 运行时（`evloop.rs:1321`）。源码注释称通道 2 "simply for trigger the `tokio::select!` loop"（`evloop.rs:274-276`）。
- **dune 没有序列化，rsvim 有。** dune 的完成回调运行在循环线程上，直接移动一个 `String`；rsvim 的任务跨线程传递字节，要付出 `postcard` 编解码的代价，因为消息枚举是通用传输格式。
- **加载器换了主人。** dune 分派给 `FsModuleLoader`/`UrlModuleLoader`/`CoreModuleLoader`；rsvim 则有 `oxc_resolver`，加上一个带 swc 转译的文件系统加载器和一个 `node_modules` 包解析器，还有一个明确标注为 "just a mock-up" 的 `import_map.rs`（`parse_from_json` 返回空、`lookup` 返回 `None`，`js/module/import_map.rs:35-41`），以及一个被从加载器模块里注释掉的 `url_loader.rs`（`js/loader.rs:5`）。
- **非致命错误。** dune 在静态 import 失败时打印并退出，rsvim 则把 `report_js_error` 送到编辑器的命令行（`js/err.rs:164` → `send_cmdline_message`）。
- **失效的旋钮。** dune 的 `skip_cache`（`--reload` / 测试模式，`src/modules.rs:344`）在 rsvim 中被注释掉了（`js/module/es_module.rs:241-244`），连同它所隐含的缓存破除逻辑。

### 2.3 Ende —— 引擎拥有状态机

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

`RecursiveModuleLoad`（`libs/core/modules/recursive_load.rs:76`）是一个 `Stream`：它解析根模块，持有一组 `loader.load()` future 组成的 `FuturesUnordered`，每到达一个模块就把它注册进 `ModuleMap`，并递归进入该模块的请求（`Poll` 位于 `recursive_load.rs:741`）。这里没有第二棵树，也没有状态枚举 —— 就绪就是 "流返回了 `None`"。

加载器是一个嵌入方 trait（`libs/core/modules/loaders.rs:61`），其 `load` 返回 `ModuleLoadResponse::Sync` 或 `::Async`（`loaders.rs:34`）。Ende 在 `cli/module_loader.rs:1070` 中实现它（其中 `:576` 的 `load_inner` 负责抓取/转译路径），因此嵌入方从来看不到模块图。

有两个细节把这个设计与 dune/rsvim 区分开：

- **TLA 是一等公民。** 求值 promise 被存了下来（`DynImportModEvaluate`，`dynamic.rs:30`），通过 `promise.state()` 轮询，用户 resolver 只在 `Fulfilled` 时敲定；若某模块已 `Evaluated` 但仍在跑 TLA，此时到来的 import 会排入 `pending_tla_waiters`（`dynamic.rs:89-97`）。
- **V8 看到的 promise 是派生出来的。** `promise.catch(map_err)`（`bindings.rs:1317`）的存在，是为了让解析错误能带着 `import()` 调用点的栈重新抛出。

### 2.4 Node.js —— JS 层拥有状态机

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
          new ModuleJobSync                          loader.js:620-623   (main thread)
      moduleJob.run()                                module_job.js:643
          instantiateSync → evaluateSync             module_job.js:633/697
          or await module.evaluate() if hasAsyncGraph (TLA)   module_job.js:671
      module.getNamespace()                          loader.js:699
```

三个属性让 Node 成为异类：

- **双重 promise。** C++ 回调新建一个 `Promise::Resolver`，并用 JS 异步函数的 promise 去兑现它（`module_wrap.cc:1184-1191`）；因此 `import()` 的值不是加载器的 promise，而是包着它的一个包装。对于带 TLA 的图还有第三个 promise，即 V8 的求值 promise，加载器会 await 它。
- **主线程同步。** 自 PR #62530（`c89ec290f0f`，"module: synchronously load most ES modules"）起，`resolveSync`/`#loadSync` 就地执行：`defaultLoadSync` 以 `readFileSync` 收尾（`load.js:38`），链接由 `ModuleJobBase.syncLink` 完成（`module_job.js:312`），求值由 `ModuleWrap::EvaluateSync` 完成（`module_wrap.cc:915`）。异步只保留给 `hasAsyncGraph`（TLA）和加载器钩子工作线程。
- **策略在 JS 里。** 以 (url, attribute type) 为键的 `loadCache` 就是合并机制（`loader.js:588/634`）；`module.registerHooks()` 在同一线程内运行同步钩子，而 `module.register()` 把它的异步钩子留在一个专用工作线程上（`hooks.js:528-538`），主线程用 `AtomicsWait` + `receiveMessageSync` 驱动该线程（`hooks.js:542-548`、`624-640`）。CJS 通过合成一个带静态导出列表的命名空间模块来桥接（`createCJSModuleWrap`，`translators.js:208`）。`require(esm)` 完全不经过该回调：它通过 `importSyncForRequire` 复用 `ModuleJobSync`（`loader.js:270`），遇到异步图则抛 `ERR_REQUIRE_ASYNC_MODULE`（`module_job.js:365`）。

---

## 3. 并排对比

### 3.1 特性矩阵

| | dune | rsvim | Ende | Node.js |
|---|---|---|---|---|
| 说明符解析 | 同步，在回调内：导入映射 → core/fs/url | 同步，在回调内：`oxc_resolver`，导入映射只是个桩 | 同步，在回调内：`ModuleMap::resolve` → loaders | 主线程同步：`resolveSync` → `defaultResolve` |
| 源码抓取 | rayon 任务，阻塞 fs / HTTP | tokio 任务，异步 fs + swc | 被轮询的 future：`ModuleLoader::load`（file/`npm:`/`jsr:`） | 主线程，`readFileSync` |
| 子模块加载 | 并发，每个一次 spawn | 并发，每个一条消息 | 并发，`FuturesUnordered` | 串行 |
| 去重 | `seen` + `Duplicate` + `same_origin` | 相同 | `visited` + 共享 `ModuleMap` | 每个 (url, type) 一个 `loadCache` |
| 顶层 await | 否（求值 promise 被丢弃） | 否（相同） | 是（`pending_dyn_mod_evaluations`） | 是（`hasAsyncGraph`） |
| 导入属性 | 忽略（`_` 参数） | 忽略 | 解析 | 解析 |
| 导入阶段 | 否 | 否 | `defer`、`source` | `defer`、`source` |
| CJS | 否 | 否 | 否 | 是（合成命名空间） |
| 错误值 | 树节点上的 `String` | 树节点上的 `String` | `v8::Global<Value>` | 带 `code` 的 JS 对象 |

### 3.2 机制矩阵

| | dune | rsvim | Ende | Node.js |
|---|---|---|---|---|
| 谁分配 promise | 回调 | 回调 | 回调 | 回调，随后由 C++ 重新包装 |
| `import()` 返回什么 | resolver 的 promise | resolver 的 promise | `promise.catch(...)` | 一个由 JS 函数喂给的新 resolver promise |
| 模块图表示 | `ModuleMap` + `EsModule` 树 | 相同 | `ModuleMap` + `RecursiveModuleLoad` 流 | V8 `ModuleWrap` + JS `ModuleJob` |
| 进展由什么驱动 | 循环 tick | 循环 tick，每条转发消息一次 | `poll_event_loop` 轮询 | 微任务（以及同步工作） |
| 由什么敲定 | `fast_forward_imports` → 兑现命名空间 | 相同 | `evaluate_dyn_imports` → 命名空间 | `ModuleLoader.import` 的返回值 |
| 引用方身份 | 引用方名称 | 引用方名称 | 引用方名称（`vm` 用宿主定义的选项） | 宿主定义的选项（模块 symbol） |
| 重入安全 | 借用纪律 | 借用纪律 | 明确注释 "no borrows held while resolving" | JS 作业缓存 |

### 3.3 轮次核算

对于一个不含 TLA 的全新文件：

| | promise 敲定前的循环轮数 | 备注 |
|---|---|---|
| dune | 每层模块图 ≥ 1 | 每一层只有在父级编译完成后才会被发现 |
| rsvim | 每层模块图 ≥ 1 个 tick，外加每条消息 1 个 | `fast_forward_imports` 在队列被排空之前运行 |
| Ende | 每层模块图 ≥ 1 次 poll | 所有就绪工作在一个 `while has_evaluated` 循环里排空 |
| Node.js | 0 轮 | 加载 + 链接 + 实例化 + 求值都在原地完成；只剩微任务 |

其他可观察的差异：

- **坏说明符**：dune/rsvim/Ende 在回调内部拒绝（返回一个已被拒绝的 promise）；Node 从异步加载器函数里拒绝。
- **顶层抛出**：dune/rsvim 仍然兑现该 import（见 §5）；Ende/Node 拒绝。
- **重复的并发 import**：dune/rsvim 把一个 resolver 挂到 `same_origin` 上；Ende 每次 import 用一个 resolver，但共用一个模块实例；Node 返回缓存的 `ModuleJob`，于是第二个 import await 的是第一个的链。

---

## 4. 同一个问题的四种答案

四个实现共享同一种扇出形状 —— 解析根、抓取、编译、发现请求、重复 —— 差别只在于*就绪传播住在哪里*：

- **一棵带四状态格的手写树**（dune、rsvim）。每个节点知道自己的子节点，并在子节点就绪时翻转为 `Ready`；tick 循环遍历待决的图并敲定已完成者。这是从零写起来最容易的东西，也正是 dune 所做的。
- **每个图一条流**（Ende）。没有树：`RecursiveModuleLoad` 就是遍历本身，`ModuleMap` 是唯一的注册表。完成就是 "流结束了"，没有需要轮询的就绪状态。
- **JS 作业的 promise 链**（Node）。`ModuleJob` 把自己记忆化进 `loadCache`；`linked` 与 `instantiated` 都是 promise，因此环与重复由 promise 身份处理，而模块图是 V8 自己的。

代价来自这个选择，而不是来自 V8：

- dune/rsvim 换来的是**统一性与控制力** —— 模块不过是又一种异步资源，代码不依赖任何框架，同一条 `spawn`/消息路径同时服务 fs、dns、定时器和 import。它们付出的代价是**手工重新实现模块语义**：TLA、求值错误、阶段、属性、代码缓存以及 "已在加载" 快速路径，全都得自己补。就 rsvim 而言，这次移植还继承了 dune 的真实 bug，而不只是它的设计。
- Ende 换来的是**语义**（TLA 等待、阶段、惰性 ESM 与合成模块、错误保真、基于 `.catch` 的栈增强），代价是一层庞大的运行时层，要跟踪源码格式、阶段和代码缓存，外加一个必须按模块暴露同步或异步加载的嵌入方 trait。
- Node 换来的是**最大的策略灵活性**（钩子在 JS 里、按 scheme 的加载器、CJS 桥接），代价是 C++→JS 的重入调用、每次 import 两到三个 promise，以及 —— 自 #62530 起 —— 只要钩子不是异步的，就在主线程上做阻塞文件 I/O。

一个有用的判断线索：那两个必须让*其他代码*决定模块字节从哪来的宿主（Ende 之于 Deno 的加载器，Node 之于用户钩子）都把状态机移出了嵌入方的热路径；而两个拥有整个应用的宿主（dune，以及 rsvim，一个编辑器的脚本引擎）把它留在了原地。

---

## 5. rsvim 从 dune 继承了什么

这些是代码层面的观察，不是测试结果 —— `module_map_tests.rs` 里的动态 import 测试只覆盖成功路径（`failed1` 演练的是 `require` 引用错误，`resolve_failed1` 演练的是 `import.meta.resolve`）。

1. **顶层 await 没有被 await。** `let _ = module.evaluate(tc_scope)`（`js.rs:865`，dune `runtime.rs:579`）丢弃了求值 promise，随后立刻兑现命名空间（`js.rs:895-905`）。`await import("./tla.js")` 会在被 await 的模块恢复之前就恢复，而 TLA 拒绝会表现为未处理的 promise 拒绝，而不是 import 拒绝。修复方式：把 promise 保存在图节点上，每个 tick 轮询 `promise.state()` —— 也就是 Ende 中 `dynamic_import_module_evaluate` / `evaluate_dyn_imports` 所用的形状（`libs/core/modules/map/dynamic.rs:203/300`）。rsvim 的 `pending_futures`（`JsFuture`）是安放这种轮询器的自然位置，所以这是一个局部改动。

2. **动态 import 从不在实例化/求值错误上拒绝。** 错误分支被 `is_root_module = !graph.root_rc().borrow().is_dynamic_import()`（`js.rs:872`）挡着，所以实例化失败的动态根会走 `report_js_error` 然后 `continue`（`js.rs:853-863`），让 promise 永远待决；而在求值时抛出异常的动态根会一路穿过去并*兑现*为命名空间。dune 有一模一样的守卫（`runtime.rs:580`）。修复方式：无论哪种状态，都拒绝该图的 resolver，并把条目从 `seen` 中删掉。

3. **错误跨异步边界后变成了字符串。** `EsModule::exception` 是 `Option<String>`，拒绝时被重建为 `v8::Exception::error(String)`（`js.rs:799-802`），于是加载器给出的 `err.code` / `err.stack` 都丢了。由于 V8 句柄无法跨线程，可移植的修复方式是发送结构化错误（`code`、`message`、`stack`）并在 JS 线程上重建它，就像 `report_js_error` 已经对 `JsError` 做的那样。

4. **每个模块一条消息、一次 `postcard` 往返。** dune 的完成回调运行在循环线程上，直接移动 `String`；rsvim 序列化它只是因为其通道负载是统一的。这是成本选择，不是约束：`String` 是 `Send`，负载本可以原样传递，而按图批量加载子模块能省掉每个模块一条消息。

5. **没能挺过移植的 dune 特性。** `skip_cache`（dune `modules.rs:344`）被注释掉了（`es_module.rs:241-244`），导入映射是个占位实现（`import_map.rs:35-41`），URL 加载器被从模块树里注释掉了（`js/loader.rs:5`）。编辑器并不需要其中任何一个，但如果真要比较行为，它们正是 dune 的加载器与 rsvim 的加载器之间的差别。

---

## 6. 来源 / 注意事项

- 四棵树都是按文首列出的 commit 读取的；**没有构建任何运行时，也没有跑任何测试**。时序陈述（"轮"、"原地"）来自控制流推导，而非测量。
- dune 的 `crabuv` 按其 `Cargo.lock` 锁定的修订版（`2fda099`）读取；它使用的线程池是 rayon（`crabuv/src/thread_pool.rs:33`），完成回调在循环线程上投递（`crabuv/src/event_loop.rs:357`）。
- 针对 Node.js 的 V8 侧引用指向 node 随源码附带（vendored）的 `deps/v8` 树，因此描述的是 Node 附带的 V8 修订版，而不是 rsvim 的 `rusty_v8` 150.4.0。V8 契约本身在两者之间是稳定的。
- Ende 是一个 Deno fork；`libs/core` 就是 `deno_core`。那里的行号在上游变动很快，而 Node 加载器里 `TODO(joyeecheung)` 风格的注释表明，同步加载设计仍在整合之中。
- 基于 QuickJS 的运行时（LLRT、txiki.js）从引擎那里获得动态 import（QuickJS-NG 中的 `js_dynamic_import` + 一个作业队列），自己不为它写任何宿主代码；它们已在 `JS_RUNTIME_COMPARISON.md` §3 中覆盖，并在此有意排除在范围之外 —— 在这里，每个运行时都得满足 V8 回调。
