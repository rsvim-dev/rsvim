# JS 运行时分析——数据与会话交接

本文是一份自包含记录，记载了 2026-10-20/21 一次 DeepSeek agent 会话中所做的分析：
问了什么、读了什么、核验了什么、得出了什么结论，以及在没有原始对话的情况下如何重新核验。

**新会话请先读这一份。** 它刻意与三份架构文档保持冗余——
本文件记录的是*输入、证据与注意事项*；那几份文档记录的是*叙述*。

配套文档（均在本仓库中）：

| 文件 | 行数 | 内容 |
|---|---|---|
| `JS_RUNTIME_COMPARISON.md` | 700 | rsvim vs. dune vs. LLRT vs. txiki.js，整体架构 |
| `JS_RUNTIME_EVENT_LOOPS.md` | 397 | 事件循环底座深入剖析，同样覆盖这四者 |
| `JS_RUNTIME_DYNAMIC_IMPORT.md` | 408 | 动态 `import()` 追踪：rsvim、dune、Ende、Node.js |
| `JS_RUNTIME_ANALYSIS_DATA.md` | 本文件 | 数据、证据索引、会话状态、后续步骤 |

交接时的状态（HEAD `fd74024`）：`JS_RUNTIME_COMPARISON.md` 已跟踪，有 3 行改动；
`JS_RUNTIME_EVENT_LOOPS.md` 已跟踪且未改动；`JS_RUNTIME_DYNAMIC_IMPORT.md` 与本文件
均为新增、未跟踪。六个代码树中都没有构建、跑基准测试或执行任何东西。

---

## 0. 续接提示词（粘贴到新会话中）

> 我在 `/home/linrongbin/github/rsvim-dev/rsvim` 维护一份关于 JS 运行时模块加载的源码级分析。
> 先读 `JS_RUNTIME_ANALYSIS_DATA.md`——它列出了六个本地仓库的锁定 commit、带 `file:line` 证据的
> 已核验发现、未提交的文档状态，以及未决问题。然后读 `JS_RUNTIME_DYNAMIC_IMPORT.md`，了解
> rsvim/dune/Ende/Node.js 的追踪；读 `JS_RUNTIME_EVENT_LOOPS.md`，了解各事件循环的底座。从 §8
> （未决问题）继续；引用任何行号前请重新核验，因为代码树可能已经变动。

---

## 1. 输入：六个本地代码树

| 仓库 | 路径 | HEAD | 日期 | 在本分析中的角色 |
|---|---|---|---|---|
| **rsvim** | `~/github/rsvim-dev/rsvim` | `fd74024` (0.1.3-alpha.2) | 2026-10-20 | 研究对象；文档也存放于此 |
| **dune** | `~/github/aalykiot/dune` | `5958b64` (v0.11.4) | 2026-10-15 | rsvim 模块加载器移植所依据的设计 |
| **Ende** | `~/github/theende/ende` | `edbd53a78` | 2026-10-20 | Deno fork；`libs/core` 即 `deno_core` |
| **Node.js** | `~/github/nodejs/node` | `14ed05f86bd` (v27.0.0, main) | 2026-10-21 | JS 层 ESM 加载器 + V8 内部实现 |
| **txiki.js** | `~/github/saghul/txiki.js` | `8a9b6044` | 2026-10-17 | QuickJS-NG：动态 import 来自引擎 |
| **LLRT** | `~/github/awslabs/llrt` | `d9fd873` | 2026-10-19 | 通过 `rquickjs` 0.14 使用 QuickJS-NG |

值得记住的代码树特殊之处：

- **txiki.js**：所有 `deps/*` 子模块都**未检出**（`git submodule status` 对位于 `10b99693` 的
  `deps/quickjs` 显示 `-` 前缀），因此拿不到 QuickJS-ng 源码。调用点和测试仍然证明动态 import 可用。
- **Ende**：`Cargo.toml` 依赖 `deno_ast`、`deno_core_icudata`、`deno_doc`、`deno_error`；
  `libs/core/**` 的头注释写着 "Copyright 2018-2026 the Deno authors"。就把它当作 Deno fork 对待。
- **LLRT**：锁定 `rquickjs` 0.14.0（`Cargo.lock:3129`），它随源码附带（vendored）QuickJS-NG 0.16.2；
  该 vendored 源码**不在**本地 crates 缓存中。缓存里是 `rquickjs-sys` 0.12.2，其 vendored
  QuickJS-NG 被用作引擎行为的替身。
- **Node.js**：`src/node_version.h` 显示 27.0.0。PR #62530（commit `c89ec290f0f`，2026-10-06，
  "module: synchronously load most ES modules"）已在本树的 `main` 上；它可能尚未进入
  已发布的 Node 版本。

---

## 2. 环境事实（吃了苦头才重新记住）

- 沙箱为 **workspace-write**，工作区在 `~/github/rsvim-dev/rsvim`：工作区外写入会被拒绝，读取则允许。
  六个代码树都读过，均未修改。
- **每次 `bash` 调用都会拿到一个全新的 `/tmp`。** 在一次调用里解压到 `/tmp` 的目录，下一次就没了。
  要么在单条命令里解压并使用，要么解压到工作区内（用完删掉——
  `.tmp_qjs/` 临时目录就是这么创建并删除的）。
- 网络：`https://static.crates.io/` 可达（直接 `curl -I` 返回 403，对桶根路径来说是正常的）。
  `npm install` 这类写入会以 `EROFS` 失败，所以 `markdownlint`/`prettier` 都跑不了；
  格式是手工检查的（围栏、行尾空白、表格形状）。
- Rust 工具链存在（`~/.cargo/bin/cargo`、`rustc`），rsvim 在 `target/` 下也有预编译产物，但本次分析
  **没有编译或运行任何东西**（V8 构建很贵）。
- `~/.cargo/git/checkouts/crabuv-111f09bf8eabb1d0/2fda099` 存放着 dune 的事件循环 crate；
  读的就是那个修订版（`crabuv` 在 dune 的 `Cargo.lock` 中锁定）。

---

## 3. 会话日志

1. **第 1 轮——主张核查。** 用户称 txiki.js 和 LLRT "尚未实现异步
   `import()`，只有静态 import"。核验（§4.6）表明两者**都**支持 `import()`；它们缺少的是
   *宿主侧*实现，因为这件事由 QuickJS-NG 完成。回复还没送达，用户就继续往下推进了。
2. **第 2 轮——四方对比。** 被要求从架构层面比较 rsvim、Ende 与 Node.js 的 `import()`。
   把三者都追踪到 promise 敲定；产出了一份聊天层面的对比。
   Node.js 部分委派给了一个子 agent，随后又对照代码树做了抽查。
3. **第 3 轮——dune。** 用户透露 rsvim 的实现基于 dune。阅读 dune 后确认这是一次近乎逐字照搬的
   移植（结构体、函数、注释）；差异在于异步底座。
4. **第 4 轮——撰写。** 写出 `JS_RUNTIME_DYNAMIC_IMPORT.md`（408 行），每条引用都对照代码树
   重新核验（初稿中有若干行号偏了 2-20，均已更正），并在 `JS_RUNTIME_COMPARISON.md` §3.3
   加了一条 3 行交叉引用。

---

## 4. 发现（已核验）

下面每条论断都是从锁定 commit 的源码中读出来的。"已核验"指的是*读过*，不是*执行过*。

### 4.1 V8 契约（四者看起来相似的原因）

- 嵌入方会安装一次 `SetHostImportModuleDynamicallyCallback`：
  rsvim `rsvim_core/src/js.rs:149`，dune `src/runtime.rs:151`，Ende
  `libs/core/runtime/setup.rs:293`，Node `src/module_wrap.cc:1219`。
- 该回调是同步的，必须返回 `MaybeLocal<Promise>`；V8 把它**原样**作为 `import()` 的值返回
  （`deps/v8/src/runtime/runtime-module.cc:28`，经由
  `deps/v8/src/execution/isolate.cc:7314`）。V8 从不分配或敲定这个 promise。
- `host_defined_options` 是编译期的嵌入方标记：Node 用它还原引用方模块符号
  （`src/module_wrap.cc:1155`）；`deno_core` 用它处理 `vm` 的动态 import 回调
  （`libs/core/runtime/bindings.rs:1195-1216`）；dune 和 rsvim 忽略它（`_host_defined_options`）。
- 静态请求在实例化时通过第二个回调解析：
  rsvim `js/hook.rs:22`，dune `src/hooks.rs:18`。

### 4.2 dune——参考设计

- 流程：`src/hooks.rs:148` → resolver `:160` → `resolve_import` `:168` → 已加载快速路径
  `:187` → `same_origin` 合并 `:200` → `ModuleGraph::dynamic_import` `:216` →
  `handle.spawn(task, Some(task_cb))` `:241`。
- `handle.spawn` 来自 crabuv：工作在 **rayon** 线程池上运行（`crabuv/src/thread_pool.rs:33`），
  完成回调在**循环线程**上运行（`crabuv/src/event_loop.rs:357`）；取回的
  `ModuleSource = String`（`src/modules.rs:79`）以 `Box<dyn Any>` 跨线程传递——不做序列化。
- 记账结构：`ModuleMap { main, by_path, by_id, seen, pending }` `src/modules.rs:82`；
  `EsModule` `:172`；`fast_forward` `:182`；`ModuleGraph { kind, root_rc, same_origin }` `:227`；
  `EsModuleFuture` `:271`；子模块 spawn `:412`。
- tick 顺序：`src/runtime.rs:427-432` = nextTick → `fast_forward_imports`（`:507`）→
  `event_loop.run(RunMode::Once)` → `run_pending_futures`（`:479`）。
- 敲定：先 `instantiate_module` + `evaluate`，再把命名空间兑现给图上的 resolver 以及
  每个 `same_origin` resolver（`src/runtime.rs:569-608`）。
- TLA 不会被等待：`let _ = module.evaluate(...)`（`src/runtime.rs:579`）；错误分支由
  `is_root_module` 保护（`:580`），所以动态根永远不会拒绝；错误是字符串
  （`src/modules.rs:283`），静态 import 失败则 `exit(1)`（`:288-289`）。
- 加载器：`src/loaders.rs`——fs、核心模块，以及一个阻塞式 `ureq` + SHA-1 缓存的 URL 加载器
  （`:198-213`）；import map 是真实实现（`src/modules.rs:484`）；`skip_cache` 用于
  `--reload`/测试模式（`src/modules.rs:344`）。

### 4.3 rsvim——跑在消息总线上的 dune 设计

移植证据（dune → rsvim）：`modules.rs:82` → `js/module/module_map.rs:147`；
`modules.rs:152/160` → `module_map.rs:49` / `js/module.rs:64`；
`modules.rs:172/182` → `js/module/es_module.rs:21/86`；`modules.rs:227` → `module_map.rs:58`；
`modules.rs:271` → `es_module.rs:162`；`hooks.rs:148-243` → `js/hook.rs:181-295`；
`runtime.rs:427/479/507` → `js.rs:610/648/776`。注释也照抄了（命名空间对象那条、
nextTick 检查点那条）。

差异：

- 加载要经过编辑器的消息总线：`pending::create_import_loader`
  （`js/pending.rs:45`）→ `MasterMessage::LoadImportReq` → `EventLoop` 派生一个 tokio 任务
  （`evloop.rs:791`）→ `async_load_import`（`js/module.rs:205`）→ `postcard` 字节 →
  `JsMessage::LoadImportResp` → `forward_js_message` 调用 `tick_event_loop`（`evloop.rs:1321`）→
  `run_pending_futures`（`js.rs:648`）→ `EsModuleFuture::run`（`es_module.rs:185`）。
- 两条通道（`jsrt_forwarder_tx` → select 循环 → `jsrt_tx`）只是为了唤醒 `tokio::select!`
  （`evloop.rs:274-276` 的注释也承认这点）。
- 加载器：`oxc_resolver` + fs 加载器 + swc 转译；import map 只是个桩
  （`js/module/import_map.rs:35-41`）；URL 加载器在模块树中被注释掉（`js/loader.rs:5`）。
- 错误是非致命的（`report_js_error` → 命令行，`js/err.rs:164`），与 dune 的 `exit(1)` 不同。
- `skip_cache` 逻辑被注释掉了（`js/module/es_module.rs:241-244`）。
- 继承下来的缺口：`js.rs:865` 丢弃了 V8 的求值 promise（TLA）；`js.rs:872` 用 `is_root_module`
  保护错误处理，所以动态根永远不会拒绝；实例化失败时在 `js.rs:853-863` 处 `continue`，
  让 promise 保持 pending；错误是 `Option<String>`，在 `js.rs:799-802` 处重建。

### 4.4 Ende——引擎掌管状态机

- `libs/core/runtime/bindings.rs:1163` 的回调 → `ModuleMap::load_dynamic_import`
  （`libs/core/modules/map/dynamic.rs:46`）；快速路径：已求值/TLA 等待者 `:64`，
  懒加载 ESM `:109`，合成 ESM `:133`；其余情况走 `RecursiveModuleLoad::new_dynamic_import`
  （`recursive_load.rs:131`）、`dynamic_import_map[load_id]`、`preparing_dynamic_imports`、
  `notify_new_dynamic_import`（`jsruntime.rs:2955`）；返回 `promise.catch(...)`（`bindings.rs:1317`）。
- 推进：`poll_event_loop`（`jsruntime.rs:2385`）→ `poll_progress`（`dynamic.rs:435`）→
  `drain_prepare` `:483` / `drain_loads` `:511` / `evaluate_dyn_imports` `:300`（轮询
  `promise.state()`）；`dynamic_import_module_evaluate` `:203` 保存 V8 的求值 promise；
  兑现/拒绝 `:394/:373`；TLA 等待者 `:89-97`。
- `RecursiveModuleLoad`（`recursive_load.rs:76`）是一个 `Stream`，对 `loader.load()` future 组成的
  `FuturesUnordered` 进行轮询（`poll_next` 位于 `:741`）；`ModuleLoader` trait 在
  `libs/core/modules/loaders.rs:61`，带 `ModuleLoadResponse::Sync|Async`（`:34`）；Ende 的加载器
  `cli/module_loader.rs:1070`，`load_inner` `:576`。
- 结果：TLA 正确，支持各阶段（`defer`/`source`），异常以 `v8::Global<Value>` 保留，
  跨多次 import 共享同一个模块实例，但每个 load id 各有一个 resolver。

### 4.5 Node.js——JS 层掌管状态机

- `src/module_wrap.cc:1126` 从 host-defined options 中读取引用方符号（`:1155`），
  同步调用 JS 回调（`:1175`），然后**重新包装**结果到一个新的 `Promise::Resolver`
  （`:1184-1191`）；注册在 `:1219`。JS 侧：`lib/internal/modules/esm/utils.js:266`，
  注册在 `:312`。
- `ModuleLoader.import`（`loader.js:682`）→ `getOrCreateModuleJob`（`:647`）→
  `#getOrCreateModuleJobAfterResolve`（`:568`）→ `loadCache.get(url, type)`（`:588`）→
  `loadAndTranslate`（`:538`）→ `#loadSync`（`:869`）→ `defaultLoadSync` → **`readFileSync`**
  （`load.js:38`）→ 转译器 → `ModuleJobSync` 在 `loader.js:620-623` 处被选中
  （类定义在 `module_job.js:595`）。
- 链接/求值：`syncLink`（`module_job.js:312`）、`instantiateSync`（`:633`）、`run`（`:643`）、
  `evaluateSync`（`:697`）；只有 `hasAsyncGraph`（TLA）才走异步（`:671`）。
- 自 PR #62530（`c89ec290f0f`，2026-10-06）起，主线程路径端到端同步；异步只保留给 TLA
  和 loader hook 工作线程。
- Hooks：`module.registerHooks()` 在线程内同步执行；`module.register()` 的异步 hook 跑在
  专用工作线程上（`hooks.js:528-538`），主线程通过 `AtomicsWait`/`receiveMessageSync` 阻塞
  （`:542-548`、`:624-640`）。
- CJS：合成命名空间模块（`translators.js:208`）。`require(esm)` 绕过 import 回调：
  `importSyncForRequire`（`loader.js:270`）、`ERR_REQUIRE_ASYNC_MODULE`
  （`module_job.js:365`）。

### 4.6 QuickJS 运行时（第 1 轮）

- **txiki.js** 支持 `import()`：`JS_SetModuleLoaderFunc2` 位于 `src/vm.c:550`；测试中的使用见
  `tests/test-import-map.js:13/24`、`tests/test-import-unknown-builtin-dynamic.js:6`、
  `tests/test-import-http.js:8`、`tests/test-worker-ffi-classes.js:43`。
- **LLRT** 支持 `import()`：它的入口点字面上就是
  `import("<file>").catch(...)`（`llrt_core/src/vm.rs:170-179`）；测试见
  `tests/unit/import.test.ts:5/25/31/39/57`；加载器实现是 `llrt_modules/src/module/loader.rs:45`
  （`impl Loader for ModuleLoader`）。
- 引擎侧：QuickJS-NG 有 `js_dynamic_import`（`quickjs/quickjs.c:1397`），它创建一个 promise 并把
  `js_dynamic_import_job` 入队（`:31060`，`JS_EnqueueJob` 位于 `:31182`）；加载器通过
  `JS_SetModuleLoaderFunc2` 安装（`:29893`）。
- 引擎的证据来源：`~/.cargo/registry/cache/index.crates.io-*/rquickjs-sys-0.12.2.crate`
  （在单条命令中解压；`/tmp` 在多次调用之间不保留）。

### 4.7 简要对比（细节见 `JS_RUNTIME_DYNAMIC_IMPORT.md` §3）

| | dune | rsvim | Ende | Node.js |
|---|---|---|---|---|
| 状态机 | 嵌入方树 | 嵌入方树（移植） | deno_core stream | JS 加载器 |
| V8 看到的 promise | resolver 的 | resolver 的 | `promise.catch(...)` | C++ 重新包装 |
| 获取 | rayon，阻塞 | tokio，异步 | 被轮询的 future | `readFileSync` |
| TLA | 否 | 否 | 是 | 是 |
| 子模块加载并发 | 是 | 是 | 是 | 否 |
| 错误值 | 字符串 | 字符串 | V8 异常 | JS 错误 + code |

---

## 5. 结论（推导所得，未引自任何来源）

1. 四种实现共享同一种扇出形态（解析 → 获取 → 编译 → 发现 → 重复）；
   区别只在于"就绪"的传播放在哪里：一棵树（dune/rsvim）、一条流（Ende），还是一条
   promise 链（Node）。
2. rsvim 的模块加载器就是 dune 的代码；唯一实质变化是异步底座
   （crabuv/rayon/typed-callback → tokio/mpsc/postcard/`task_id`）。`same_origin`、`seen`、
   `Duplicate`、`fast_forward` 这套机制没有变。
3. Ende 是"框架派"的答案：`RecursiveModuleLoad` + `poll_progress` 把 dune/rsvim 手工重写的
   模块语义（TLA、阶段、错误保真、代码缓存）吸收进来。
4. Node 是"策略放在 JS 里"的答案，而自 #62530 起，它又成了"同步加载"的答案：`import()` 只是
   同步 I/O 之上的一层 promise 边界，只有 TLA 和加载器钩子 worker 才真正异步。
5. 对 rsvim 而言，实际后果是它的缺陷属于*继承来的结构性*问题（TLA、动态根拒绝、字符串错误），
   而不是偶发 bug——修复它们只是 `js.rs`/`es_module.rs` 中的局部改动，而不是重新设计。

---

## 6. rsvim 的缺口与修复草案（摘自 `JS_RUNTIME_DYNAMIC_IMPORT.md` §5）

1. **TLA**：保留 `module.evaluate()` 的 promise，每个 tick 轮询 `promise.state()`，在
   `Fulfilled` 时敲定——照搬 `dynamic.rs:203/300` 的做法；`pending_futures`（`JsFuture`）
   就是存放它的地方。
2. **动态根拒绝**：对动态图去掉 `is_root_module` 保护；实例化失败时拒绝该图的各个 resolver，
   而不是 `continue`（`js.rs:853-863`），并删掉 `seen` 条目，好让重试时重新获取。
3. **错误保真**：传输结构化的 `{code, message, stack}` 而不是 `String`
   （V8 句柄不能跨线程），再用现有的 `JsError` 机制重建。
4. **传输成本**：`postcard` 是选择，不是约束（`String` 是 `Send`）；按图批量发送子模块加载请求，
   可以每个模块省下一条消息。
5. **dune 里已死的开关**：`skip_cache`、真正的 import map、URL 加载器——编辑器都不需要，
   但它们解释了 dune/rsvim 加载器的差异。

---

## 7. 更正与被证伪的前提（请保留；这些很容易被重新引入）

- **"txiki.js 和 LLRT 没有异步 `import()`。"** 在 JS 层面是错的（§4.6）。只有在狭义上成立：
  两者都没写宿主侧的动态 import 代码——那是 QuickJS-NG 做的。
- **Node 符号名。** `node::loader::ImportModuleDynamically`、`src/node_module_loader.*`、
  `setupModuleLoader`、`ModuleLoader#getModuleJob`、`#getModuleJobForImport` 以及一个
  `ModuleMap` 类在 `14ed05f86bd` 处**都不存在**。真正的名字是
  `ModuleWrap::ImportModuleDynamically`、`getOrCreateModuleJob`、
  `#getOrCreateModuleJobAfterResolve`、`LoadCache`/`ResolveCache`
  （`lib/internal/modules/esm/module_map.js`）、`ModuleJobSync`/`ModuleJob`。
- **写作过程中行号发生了漂移。** 初稿引用是近似值；后来全部重新锁定。
  更正示例：dune `hooks.rs:162→160`、`:174→168`、`:190→187`、`:207→200`、`:221→216`；
  Ende `dynamic.rs:50→46`、`:317→300`、`:419→435`、`:472→483`、`:518→511`；
  rsvim 的 `ModuleStatus` 在 `js/module.rs:64`（不是 `es_module.rs`）。引用前请重新核验。
- **Node 的同步加载是新事物。** 主线程上的 `ModuleJobSync` 与 `readFileSync` 是这个 commit 处
  `main` 的状态；更早的 Node 版本所有路径都用异步 `ModuleJob`。

---

## 8. 未决问题 / 建议的后续步骤

1. **给 rsvim 补测试**：覆盖对抛出异常的模块、无法解析的说明符、缺失的具名导出以及 TLA 模块的
   `import()`。现有测试只覆盖成功路径（`module_map_tests.rs`；`failed1` 测的是 `require`，
   `resolve_failed1` 测的是 `import.meta.resolve`）。这会把 §6.1/6.2 从预测变成观测。
2. **实现 TLA 轮询器**（§6.1）——改动最小、语义收益最大。
3. 决定是否提交这些文档：`JS_RUNTIME_DYNAMIC_IMPORT.md` 未跟踪，
   `JS_RUNTIME_COMPARISON.md` 有 3 行改动。
4. 可选：把 QuickJS 的说明并入 `JS_RUNTIME_COMPARISON.md` §3.1：LLRT/txiki 的动态 import 由
   引擎提供，而非宿主实现。目前的 ✅✅ 行是正确的，但解释不足。
5. 启动耗时/内存/单次 import 延迟都没有数字；所有关于"轮次（turn）"的陈述都是控制流推理。
   任何基准测试都属于新工作。

---

## 9. 复现步骤

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

## 10. 关键文件索引（想看什么读哪里）

| 问题 | 文件 | 关注行 |
|---|---|---|
| dune 回调 + 图 | `dune/src/hooks.rs`、`dune/src/modules.rs` | 148-243 / 79-283, 344-412 |
| dune tick + 敲定 | `dune/src/runtime.rs` | 427-432, 479, 507, 566-608 |
| rsvim 回调 | `rsvim_core/src/js/hook.rs` | 181-295 |
| rsvim 图/编译 | `rsvim_core/src/js/module/es_module.rs`、`module_map.rs` | 86, 162-322 / 49-160 |
| rsvim tick/敲定 | `rsvim_core/src/js.rs` | 610, 648, 713, 776, 836-905 |
| rsvim 消息总线 | `rsvim_core/src/evloop.rs`、`js/pending.rs`、`js/module.rs` | 791, 1321 / 45 / 205 |
| Ende 绑定 | `ende/libs/core/runtime/bindings.rs` | 1163-1327 |
| Ende 动态 import 映射 | `ende/libs/core/modules/map/dynamic.rs` | 30-520 |
| Ende 流 | `ende/libs/core/modules/recursive_load.rs` | 76-180, 741-780 |
| Ende 加载器实现 | `ende/cli/module_loader.rs` | 576, 1048-1110 |
| Node C++ 桥接层 | `node/src/module_wrap.cc` | 1126-1226 |
| Node JS 加载器 | `node/lib/internal/modules/esm/loader.js` | 538-705, 869-893 |
| Node 模块作业 | `node/lib/internal/modules/esm/module_job.js` | 312-351, 595-700 |
| Node hooks 工作线程 | `node/lib/internal/modules/esm/hooks.js` | 528-548, 590-640 |
| QuickJS 引擎 | `rquickjs-sys-0.12.2/quickjs/quickjs.c` | 1397, 29893, 31060-31190 |

---

## 11. 产出物与仓库状态

- 新写：`JS_RUNTIME_DYNAMIC_IMPORT.md`（408 行，未跟踪）。
- 已编辑：`JS_RUNTIME_COMPARISON.md`（§3.3 中新增 3 行，指向新文档）。
- 本次会话前已跟踪：`JS_RUNTIME_COMPARISON.md`（697 行，现为 700）、`JS_RUNTIME_EVENT_LOOPS.md`
  （397 行，未变）。两者都在 commit `fd74024`（"docs: write down js runtime architecture
  document"）中落地，即它们是更早一次会话的产物，应视为权威的既有工作，而不是重新推导。
- 解压时在仓库下创建的临时目录 `.tmp_qjs/` 已删除；除此之外代码树未被改动。
- 未做任何提交。交接时的 `git status`：
  `M JS_RUNTIME_COMPARISON.md`、`?? JS_RUNTIME_DYNAMIC_IMPORT.md`、`?? JS_RUNTIME_ANALYSIS_DATA.md`。

---

## 12. 给下一次会话的工作笔记

- 用户用中文写作，但仓库文档是英文；新文档保持英文，回复用中文。
- 偏好的证据风格：精确的 `file:line`、表格、明确区分"靠阅读核验"与"实测"，以及不给出无法追溯到
  某一行的论断。对比文档采用以 `---` 分隔的编号小节，并在末尾附来源/注意事项一节。
- 允许读取工作区外内容；写入不允许。`/tmp` 在命令之间不保留。
- 发现变化时请更新本文件——它是会话重置后唯一幸存的东西。
