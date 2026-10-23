# JS Runtime Module Management

## dune vs. LLRT vs. txiki.js vs. Deno vs. Node.js

> 从 high-level / architecture 角度，分析五种 JavaScript runtime 如何解析、加载、链接和执行 module。
>
> 本文关注的是“module 子系统由谁拥有、状态机放在哪一层、builtin 以什么形式交付”，而不是逐个 API 做功能对照。
>
> 分析日期：2026-10-24。

---

## 0. 分析范围与源码快照

分析对象：

| Runtime | Repo | HEAD | Date | Engine | Host language |
|---|---|---|---|---|---|
| **dune** | `aalykiot/dune` | `5958b64` (v0.11.4) | 2026-10-15 | V8 (`rusty_v8` 150.2.0) | Rust |
| **LLRT** | `awslabs/llrt` | `d2ae1d9` | 2026-10-20 | QuickJS-NG via `rquickjs` 0.14 | Rust |
| **txiki.js** | `saghul/txiki.js` | `fc2b5f3b` | 2026-10-21 | QuickJS-NG | C |
| **Deno** | `denoland/deno` | `d7205acbed` | 2026-10-22 | V8 via `deno_core` | Rust |
| **Node.js** | `nodejs/node` | `a232998e0` | 2026-10-22 | V8 | C++ / JavaScript |

相关文档：

| Document | 本文使用的内容 |
|---|---|
| `JS_RUNTIME_COMPARISON.md` | ES module 特性、resolver 模型、builtin API、startup/distribution |
| `JS_RUNTIME_DYNAMIC_IMPORT.md` | V8 dynamic import contract、dune/Deno/Node 的 dynamic import 状态机 |
| `JS_RUNTIME_EVENT_LOOPS.md` | loop ownership、module work 与 event loop 的关系 |
| `JS_RUNTIME_EVENT_LOOP_ANALYSIS.md` | resolve/load/link/evaluate 分层、TLA、module work 与 liveness |
| `JS_RUNTIME_ANALYSIS_DATA.md` | 已验证的源码结论、V8 callback contract、各 runtime 的关键文件索引 |

说明：

- 旧文档的源码快照与当前本地 HEAD 略有差异；本文以当前源码为准。
- `JS_RUNTIME_DYNAMIC_IMPORT.md` 使用了一个 Deno fork 作为样本，其 `libs/core` 对应 `deno_core`；本文直接以当前 Deno 源码复核架构结论。
- 本文没有构建或 benchmark 这些 runtime；结论来自源码阅读和既有文档的交叉验证。

---

## 1. 一个统一的 module 生命周期

不管 runtime 用什么 engine，一个 module 基本都走同一条流水线：

```text
specifier
  -> resolve        解析为 canonical URL / path / builtin id
  -> load           获取源码：文件、HTTP、native module、bytecode、snapshot
  -> compile        解析 JS / TS / JSON / WASM / CJS，生成 engine module
  -> link/instantiate
                    建立依赖图，绑定 import/export
  -> evaluate       执行模块；可能遇到 top-level await
  -> namespace/cache
```

真正决定架构差异的是下面几个问题：

1. **谁拥有 module graph？**
   - host/embedder
   - engine/runtime core
   - JavaScript 层

2. **resolution 和 load 的策略放在哪？**
   - 只支持 import map / URL / file
   - 支持 Node 的 `node_modules` 和 `package.json#exports`
   - 支持 npm / JSR / `node:` / permissions

3. **builtin 以什么形式存在？**
   - JS ESM 源文件
   - native module
   - 预编译 bytecode
   - snapshot / lazy extension

4. **link/evaluate 的语义由谁负责？**
   - V8 host 自己 `instantiate_module` / `evaluate`
   - QuickJS engine 自己递归
   - `deno_core` 的 `ModuleMap`
   - Node 的 JS `ModuleJob`

5. **dynamic import / TLA 的 Promise 由谁创建和结算？**
   - host 创建 resolver
   - engine 创建 Promise
   - JS loader 创建 `ModuleJob`
   - C++ 再包一层 Promise

---

## 2. 总表

### 2.1 Graph ownership、resolution 与 execution

| Runtime | Graph owner | Resolution | Source acquisition | Link / evaluate |
|---|---|---|---|---|
| **dune** | Rust host | import map → core / fs / URL | crabuv worker；fs / `ureq` / `CORE_MODULES` | host `instantiate_module` + `evaluate` |
| **LLRT** | QuickJS / rquickjs | Node-style package resolver + `node:` 去前缀 | native `ModuleDef` / `.lrt` bytecode / fs | QuickJS engine |
| **txiki.js** | QuickJS | import map + `tjs:*` + URL + file | C module / bytecode / HTTP / fs | QuickJS engine |
| **Deno** | `deno_core` runtime + CLI graph | import map + npm / JSR / `node:` + permissions | `deno_graph` / emit cache / file / HTTP / extension | `deno_core` `ModuleMap` |
| **Node.js** | JS ESM loader + CJS loader | package `exports` / `imports` + `node_modules` + `node:` | `readFileSync` / builtin map / hooks | JS `ModuleJob` + V8 `ModuleWrap` |

### 2.2 Builtin、CJS 与 TLA

| Runtime | Builtin model | Builtin dependency | CJS / npm | TLA |
|---|---|---|---|---|
| **dune** | 19 个 JS ESM 模块，`include_str!` | 真实 ESM import；host 递归加载 | 无 CJS、无 npm | 不等待 evaluation promise |
| **LLRT** | native `ModuleDef` + JS bytecode | native 共享 crate/global；JS builtin 可 import native | 部分 CJS，Node 兼容 | 由 QuickJS engine 处理 |
| **txiki.js** | C core + JS/bytecode bundles | core 构建期 bundle；stdlib 运行期 import `tjs:internal/*` | 无 CJS、无 npm | 由 QuickJS engine 处理 |
| **Deno** | ESM extensions + snapshot / lazy / synthetic | extension 之间的 ESM import；`synthetic_esm` | 完整 Node compat / npm | 正确跟踪 evaluation promise |
| **Node.js** | `BuiltinModule` map，源码内嵌 | builtin 之间通过 internal require / ESM facade | 原生 CJS + ESM bridge | 支持；async graph 走异步 evaluate |

---

## 3. dune：embedder 自己实现整个 module 状态机

### 3.1 定位

dune 是 V8 + Rust + 自己的 libuv-shaped event loop（`crabuv`）。V8 只提供两个回调：

- `module_resolve_cb`：静态 import 时，embedder 必须返回一个 `v8::Module`；
- `host_import_module_dynamically_cb`：`import()` 时，embedder 必须返回一个 Promise。

dune 选择把整个 module 状态机放在 Rust host 里。

### 3.2 数据结构

核心结构在 `src/modules.rs`：

```rust
ModuleMap { main, by_path, by_id, seen, pending }
EsModule { path, status, dependencies, exception, is_dynamic_import }
ModuleGraph { kind, root_rc, same_origin }
EsModuleFuture
```

- `ModuleMap` 是全局注册表；
- `EsModule` 是一棵手写依赖树；
- `ModuleStatus` 有 `Fetching / Resolving / Duplicate / Ready`；
- `fast_forward()` 自底向上把 `Resolving → Ready`；
- `same_origin` 用来合并并发 import 同一 root 的 resolver。

### 3.3 静态加载

```text
fetch_module_tree()
  -> compile_module
  -> ModuleMap.insert
  -> for each ModuleRequest:
       resolve_import()
       if unseen: fetch_module_tree(child)
```

`fetch_module_tree()` 在编译当前模块后立即把它插入 `ModuleMap`，再递归加载依赖。这样 `module_resolve_cb` 在 `instantiate_module` 时可以在 map 里找到所有子模块。

### 3.4 动态 import

```text
import("x")
  -> host_import_module_dynamically_cb
  -> PromiseResolver::new
  -> resolve_import (sync)
  -> 已加载：直接 resolve namespace
  -> 正在加载：加入 same_origin
  -> 否则：ModuleGraph::dynamic_import
  -> crabuv handle.spawn 加载源码
  -> 回调进入 loop
  -> EsModuleFuture 编译 / 递归
  -> fast_forward_imports
  -> instantiate_module + evaluate
  -> resolve namespace
```

dune 把 module loading 当作“另一种 libuv resource”：源码在 rayon 线程读取，completion 回到 loop 线程，继续在 V8 上编译和递归。它和 `fs`、`dns`、`net` 共用同一套 async 基础设施。

### 3.5 Resolver

`resolve_import()` 按 specifier 选择 loader：

- `CoreModuleLoader`：`CORE_MODULES` 中的 19 个 builtin；
- `UrlModuleLoader`：`http(s)://`，`ureq` + SHA-1 磁盘缓存；
- `FsModuleLoader`：文件路径；
- import map：WICG 子集，支持 trailing slash 和 extensionless specifier。

没有 `node_modules`，没有 CJS。bare specifier 只有命中 `CORE_MODULES`，或被 import map 改写后才能解析。

### 3.6 Builtin

dune 的 builtin 是普通的 JS ESM 源文件：

```rust
CORE_MODULES: HashMap<&str, &str> = [
  "console", "events", "process", "timers", "assert", "util",
  "fs", "perf_hooks", "colors", "dns", "net", "test",
  "stream", "http", "sqlite",
  "@web/abort", "@web/text_encoding", "@web/clone", "@web/fetch",
]
```

入口 `src/js/main.js` 直接 import：

```js
import process from 'process';
import timers from 'timers';
import fetch from '@web/fetch';
import structuredClone from '@web/clone';
import { Console, prompt, wrapConsole } from 'console';
import { AbortController, AbortSignal } from '@web/abort';
import { TextEncoder, TextDecoder } from '@web/text_encoding';
```

这些 import 是真实的 ESM 依赖：

```text
main
 ├─ process -> events, util
 ├─ timers -> assert
 ├─ @web/fetch -> http -> net -> dns
 │                    -> assert, events
 ├─ console -> perf_hooks, colors
 ├─ @web/abort -> events
 ├─ @web/text_encoding
 └─ @web/clone

test -> fs, perf_hooks, colors
stream -> assert, events
```

因此 dune 的 builtin 不是“独立小模块的集合”，而是一张真实的 builtin 依赖图。`fetch_module_tree()` 是它能工作的前提。

### 3.7 架构特征与代价

优点：

- 高度可控，module loading 与 fs/dns/net 使用同一套 async 机制；
- 不依赖 engine 的 module 生命周期，容易插入自定义错误、日志和缓存；
- 与 dune 自己的 event loop / thread pool 深度整合。

代价：

- TLA 不被等待：`let _ = module.evaluate(...)`，evaluation promise 被丢弃；
- 动态 import 在 evaluate 后立即 resolve namespace，TLA 可能仍在 pending；
- import attributes、phases、evaluation error、code cache 等语义都要自己补；
- 没有 CJS，npm 生态基本不可用。

---

## 4. LLRT：engine 拥有图，host 提供 resolver / loader

### 4.1 定位

LLRT 是 QuickJS-NG + `rquickjs`，面向 AWS Lambda 冷启动，目标是“Node 子集 + 尽可能小的启动成本”。

QuickJS 本身有 module loader 机制，所以 LLRT 不手写依赖树。它提供一条 loader 链，QuickJS 负责递归、link、evaluate。

### 4.2 Loader 链

LLRT 的 resolver / loader 大致是：

```text
ModuleResolver     native module 名称：fs、path、events、buffer...
EmbeddedResolver   预编译 bytecode cache：bundle/js/*.lrt
PackageResolver    Node 的 LOAD_AS_FILE / LOAD_AS_DIRECTORY / LOAD_NODE_MODULES
file resolver      普通文件
```

`node:` 前缀在 resolver 里被剥掉：

```rust
let name = name.trim_start_matches("node:").trim_end_matches("/");
```

所以 `import fs from "node:fs"` 最终解析到 native `"fs"` module。

### 4.3 Native module 与 JS builtin

LLRT 的 builtin 分两类。

**（1）native ModuleDef**

绝大多数 builtin 是 Rust `ModuleDef`：

```rust
impl ModuleDef for FsModule {
    fn declare(declare: &Declarations) -> Result<()> { ... }
    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> { ... }
}
```

它们直接向 QuickJS 注册 `declare` / `evaluate`，不经过 JS 源文件。例如 `fs`、`path`、`events`、`net`、`crypto`。

**（2）JS builtin + bytecode**

少数 builtin 是 JS，例如 `stream`、`@llrt/test`：

- 先用 esbuild 打成 `bundle/js/*.js`；
- `llrt_core/build.rs` 遍历 `bundle/js`，用 QuickJS 编译成 bytecode；
- bytecode 按模块名存入 `BYTECODE_CACHE`，首次 import 时解压加载。

所以 LLRT 的 builtin 是 **native + bytecode 的混合体**。

### 4.4 Builtin 之间的依赖

LLRT 的 native module 之间通常不是 ESM import，而是共享 Rust crate 和 global primordials。

例如 `node:fs`：

- `llrt_fs` 使用 `llrt_buffer::Buffer`；
- `Buffer::into_js()` 通过 `BufferPrimordials` 获取全局 `Buffer` 构造器；
- `buffer::init` 作为 global attachment 在用户代码前执行；
- `fs` 本身不会 `import "node:buffer"`，也不依赖 `node:path` module；它使用的是 `llrt_path` crate。

所以 `fs` 对 `buffer` 的依赖是 **初始化/全局对象依赖**，不是 ESM 模块依赖。

但 LLRT 确实有 builtin import builtin 的例子。`node:stream` 是 JS builtin：

```ts
import { EventEmitter as EE } from "node:events";
import { inherits } from "node:util";
```

`node:events`、`node:util` 是 native module；`stream` 是 JS bytecode module。bytecode 里保留 import 表，QuickJS 在运行时继续通过 loader 解析它们。

### 4.5 CJS 兼容

LLRT 也实现了 CJS 兼容（另外两个是 Node 和 Deno）。它用两个 magic prefix：

- `__cjs:`：这个路径来自 CJS import；
- `__cjsm:`：强制按 CJS 处理。

`PackageLoader` 会把 CJS 模块合成为 ESM：

```js
const value = require("...");
export default value.default || value;
// 再从 Object.keys(module.exports) 推导 named exports
```

`require(esm)` 则走 `Module::import` + 嵌套 pump。循环依赖由 `RequireState { cache, exports, progress }` 处理。

### 4.6 架构特征

- QuickJS engine 拥有 module graph；
- host 只实现 resolver / loader；
- native module 让 API 很快，但依赖体现在 Rust crate 和全局初始化上；
- JS builtin 走 bytecode，启动时不必解析源码；
- CJS 兼容是为 AWS SDK 生态服务的，不是通用 Node 复刻。

---

## 5. txiki.js：一个 QuickJS loader，构建期 bundle + 运行期 `tjs:*`

### 5.1 定位

txiki.js 是 C + QuickJS-NG + libuv，目标是“Web 平台 API + 系统能力”，明确不做 Node 兼容。

它只注册一个 module loader：

```c
JS_SetModuleLoaderFunc2(
    rt,
    tjs_module_normalizer,
    tjs_module_loader,
    tjs_module_attr_checker,
    qrt
);
```

### 5.2 Normalizer：specifier 策略

`tjs_module_normalizer()` 负责：

- `tjs:internal/*` 只允许被 `tjs:` 模块 import，用户文件不能直接 import；
- 裸 specifier 先过 import map；
- 相对路径做平台路径归一化；
- unmatched bare specifier 原样传下去；
- 没有 `node_modules`。

`tjs:internal/*` 的限制是显式的：

```c
if (name starts with "tjs:internal/" && base_name not starts with "tjs:")
    throw ReferenceError("module is not exposed to user code")
```

### 5.3 Loader：模块形态

`tjs_module_loader()` 按 module name 分派：

- `tjs:internal/core`：C 构造的 default module，导出 C 层 `core` 对象；
- `tjs:internal/worker`：同理；
- 其他 `tjs:internal/*` 和 `tjs:*`：`tjs__load_builtin()` 从 `builtins[]` 取预编译 bytecode；
- `http(s)://`：运行时通过 libwebsockets 下载并编译；
- 文件系统模块：读取文件，按 `type: json | text | bytes` 或 `.json` 后缀处理；
- 普通 JS：`JS_Eval(..., JS_EVAL_TYPE_MODULE | JS_EVAL_FLAG_COMPILE_ONLY)`。

### 5.4 Builtin 与构建流水线

txiki 的 builtin 交付是分层的：

| 层 | 构建方式 | 加载时机 |
|---|---|---|
| `tjs:internal/core` | C `core` 对象 | 启动时创建 |
| core bundle | esbuild `core/index.js` → `tjs:internal/bootstrap` | 启动时 eager |
| polyfills bundle | esbuild `polyfills/index.js` | 启动时 eager |
| stdlib `tjs:*` | 每个文件单独 `qjsc` 成 bytecode | 首次 import 时 lazy |

`src/js/core/index.js` 导入大量 core 内部文件：

```js
import core from 'tjs:internal/core';
import internalPathHolder from 'tjs:internal/path';
import engine from './engine.js';
import env from './env.js';
import { open, makeDir, makeTempFile, remove, symlink, writeFile } from './fs.js';
import { serve } from './httpserver.js';
import { parseImportMap } from './import-map.js';
import { lookup } from './lookup.js';
import pathModule from './path.js';
import { spawn } from './process.js';
import { addSignalListener, removeSignalListener } from './signal.js';
import './direct-sockets/index.js';
import { connect, listen } from './sockets.js';
import { getStdin, getStdout, getStderr } from './stdio.js';
import system from './system.js';
import './sourcemap.js';
```

esbuild 用 `--bundle --external:tjs:*` 把这些 local import 打成一个 core bundle；`qjsc` 再把它编译成 `tjs:internal/bootstrap`。所以在运行时，core 内部已经没有这些 local import，只剩下 `tjs:*` 外部依赖。

stdlib 则是独立 bytecode builtin，例如：

```js
// tjs:path
import pathModule from 'tjs:internal/path';

// tjs:hashing
import core from 'tjs:internal/core';

// tjs:sqlite
import core from 'tjs:internal/core';

// tjs:wasi
import core from 'tjs:internal/core';
```

这些是真实的运行期 builtin 依赖。`tjs:path` 依赖 core 先通过 `Object.assign` 填充 `tjs:internal/path` 这个 holder。

### 5.5 架构特征

- QuickJS engine 拥有 module graph；
- host 只有一个 loader 函数，策略很薄；
- core/polyfills 在构建期 bundle，启动时 eager；
- stdlib 是 lazy bytecode；
- 没有 CJS、没有 npm，import map/URL 是主要扩展手段；
- “Web 优先”的模块策略，与 Node 生态刻意保持距离。

---

## 6. Deno：两层 module graph，engine 拥有运行时，CLI 拥有解析/构建

### 6.1 定位

Deno 是五者中最复杂的：它要同时支持 Web API、npm、JSR、Node compat、TypeScript、permissions 和 snapshot。

可以分成两层：

```text
deno_graph（CLI 静态图：解析、依赖分析、转译、缓存；类型检查是可选阶段）
        ↓
deno_core ModuleMap（运行时图：ModuleId、handle、link、evaluate）
```

### 6.2 `deno_core`：运行时 module graph

`deno_core` 提供 `ModuleLoader` trait：

```rust
fn resolve(&self, specifier, referrer, kind) -> Result<ModuleSpecifier>;
fn load(&self, specifier, referrer, options) -> ModuleLoadResponse;

enum ModuleLoadResponse {
    Sync(Result<ModuleSource, ModuleLoaderError>),
    Async(Pin<Box<ModuleSourceFuture>>),
}
```

运行时用 `RecursiveModuleLoad` 这个 `Stream` 递归加载依赖：

```text
RecursiveModuleLoad
  -> resolve root
  -> FuturesUnordered<loader.load()>
  -> compile_module
  -> ModuleMap 注册
  -> register_and_recurse()
  -> 对每个 ModuleRequest 继续加载
```

`ModuleMap` 维护：

- `ModuleId` / module handle；
- `pending_dynamic_imports`；
- `pending_dyn_mod_evaluations`；
- synthetic ESM / lazy ESM / wasm module。

动态 import 由 `ModuleMap` 管理，TLA 的 evaluation promise 会被保存并 poll。`EventLoopPendingState` 把：

- pending dynamic imports；
- pending dynamic module evaluations；
- pending main module evaluation

都计入 liveness。所以 Deno 的 `import()` 会等 evaluation 完成再交付 namespace。

### 6.3 CLI：`deno_graph` + `CliModuleLoader`

Deno CLI 在运行时之前先构建 `deno_graph`：

- 解析 import map、npm、JSR、`node:`、文件/HTTP；
- 做依赖分析、转译；`deno check` 场景下再做类型检查；
- 写 emit cache / code cache。

`CliModuleLoader` 的职责：

- `resolve()`：import map / npm / JSR / node resolver / permissions；
- `prepare_load()`：触发 graph 构建或更新；
- `load()`：从 graph / emit cache 取已转译 source；
- 支持 `module.registerHooks()` 风格 resolve/load hooks；
- 对同步加载提供 `try_load_inner_sync()` 快路径。

注意：Deno 的 graph 构建和 `deno_core` 的递归加载不是同一张图。`deno_graph` 是 CLI 层的静态/构建图，`ModuleMap` 是运行时图。

### 6.4 Builtin：Extension 是 ESM，不是 native module

Deno 的 builtin 主要是 `Extension`：

```rust
Extension {
    esm: [...],
    lazy_loaded_esm: [...],
    synthetic_esm: [...],
    esm_entry_point: Some("ext:..."),
    ...
}
```

四种交付方式：

1. **`esm`**：随 extension 一起加载/注册为 side module，不自动 evaluate；由 `esm_entry_point` 指定的模块在启动时 evaluate；
2. **lazy ESM**：`lazy_loaded_esm`，注册但不 evaluate，首次 import 时才 deserialize；
3. **synthetic ESM**：`synthetic_esm`，把已有脚本的 exports 包装成 ESM facade；
4. **snapshot**：extension 源码和 entry point 可以进入 V8 snapshot。

Deno 的 public builtin 通常不是纯 JS：底层是 Rust `#[op2]`，ESM extension 只是 wrapper；但在 module 层面，它们以 ESM 形式注册、解析和加载。

`ext:` 模块有访问限制：只允许 `ext:` / `node:` 内部模块 import。Node compat 的 polyfills 大量是 `lazy_loaded_esm`，例如：

- `node:module`
- `node:process`
- `node:stream`
- `node:net`
- `node:tty`

这样非 Node 程序不会为它们付出启动成本。部分 `node:*` 走 `synthetic_esm`，例如 `node:url`、`node:util`、`node:worker_threads`、`node:zlib`。

### 6.5 CJS / npm

Deno 的 CJS 支持在 `ext/node`：

- `NodeRequireLoader`：`require` 的 loader 抽象；
- `CjsTracker`：判断一个模块是否可能是 CJS；
- `deno_node` ops：`require`、`createRequire`、`node:module`；
- ESM loader 和 CJS 之间通过 synthetic wrapper / node polyfill 桥接。

npm/JSR 的解析、下载、安装、缓存都在 CLI 层完成；运行时只看到最终的 module specifier 和 source。

### 6.6 架构特征

- engine/core 层拥有运行时图；
- CLI 层拥有解析、构建、类型检查、转译和缓存；
- builtin 是 ESM extension，支持 eager/lazy/synthetic/snapshot；
- Node compat 是建立在 ESM extension 和 ops 之上的兼容层；
- 解析/加载是 permission-aware 的（dynamic import 可以检查权限）。

---

## 7. Node.js：JS 层的 ESM loader + 独立的 CJS loader

### 7.1 定位

Node 的独特之处在于：CJS 是原生起点，ESM 是后加并通过 bridge 兼容。它不是一套 loader，而是两套。

### 7.2 CJS：`Module._load`

CJS 是 Node 的原始模块系统：

```text
require(x)
  -> Module._load
  -> Module._resolveFilename
  -> Module._findPath
        -> node_modules 逐级查找
        -> package.json#main
        -> extension / directory index
  -> Module._cache[filename]
  -> new Module(...)
  -> compile wrapper
  -> execute
```

关键点：

- `Module._cache` 以 filename 为 key；
- 循环依赖通过“先放入 cache，再执行”处理；
- `require()` 是同步的；
- `node_modules` 解析算法是 Node 生态的核心。

### 7.3 ESM：JS 层的 `ModuleLoader`

ESM 的核心在 `lib/internal/modules/esm/`：

```text
import("x")
  -> V8 host callback
  -> C++ module_wrap.cc
  -> JS importModuleDynamicallyCallback
  -> ModuleLoader.import
  -> getOrCreateModuleJob
  -> loadCache.get(url, type)
  -> defaultResolve
  -> defaultLoad
  -> ModuleJob / ModuleJobSync
  -> V8 ModuleWrap
```

关键结构：

- `ModuleLoader`：ESM loader 主类；
- `loadCache`：key 是 `(url, importAttributes.type)`；
- `ModuleJob`：异步 job；`ModuleJobSync`：主线程同步 job；
- `defaultResolve()`：package `exports` / `imports`、conditions、`node_modules`；
- `defaultLoad()`：读源码、判断 format；
- `ModuleWrap`：V8 module handle；
- `module_wrap.cc`：只是 V8 callback 的 trampoline，把 JS 返回的 Promise 再包一层。

当前 Node 主线程的 ESM 路径已经很同步：

```text
defaultResolve
  -> readFileSync
  -> ModuleJobSync.syncLink
  -> instantiateSync
  -> evaluateSync
```

只有两类情况真正异步：

1. 模块图包含 top-level await（`hasAsyncGraph`）；
2. 使用 `module.register()` 的异步 loader hooks（运行在 loader hook worker）。

`module.registerHooks()` 则是同步 in-thread hooks。

### 7.4 Builtin：`BuiltinModule` map + ESM facade

Node 的 builtin JS 源码通过 `js2c` 编入二进制。`lib/internal/bootstrap/realm.js` 中的 `BuiltinModule` 维护：

```text
id -> BuiltinModule { exports, loaded, loading, module, exportKeys }
```

`node:` 前缀和裸 `fs` 都通过 `BuiltinModule.normalizeRequirableId()` 归一化。

CJS builtin 有 CJS exports；当 ESM 代码 `import fs from "node:fs"` 时：

```text
BuiltinModule.getESMFacade()
  -> 创建 ModuleWrap
  -> syncExports()
  -> 生成 named exports + default
```

因此 builtin 能同时被 `require()` 和 `import`。

### 7.5 CJS/ESM bridge

Node 的桥接有三条路：

1. **`import` 一个 CJS 文件**：`createCJSModuleWrap()` 合成 ESM namespace；
2. **`require` 一个 ESM 文件**：`importSyncForRequire()` 复用 `ModuleJobSync`，有 TLA 就抛 `ERR_REQUIRE_ASYNC_MODULE`；
3. **loader hooks**：同时服务 CJS 和 ESM。

### 7.6 架构特征

- module policy 在 JavaScript 里；
- C++ 只做 V8 的薄适配；
- 真正的 resolve/load/job/cache/hooks 都在 JS；
- 两套缓存、两套循环依赖处理；
- CJS/ESM 语义边界的兼容成本长期存在。

---

## 8. 横向对比：五条架构主线

### 8.1 V8 把 module graph 的所有权留给了 embedder

V8 只提供：

- 静态 import 的 `module_resolve_cb`；
- 动态 import 的 host callback + 一个必须由 host 创建/结算的 Promise。

所以 V8 runtime 典型地有三种选择：

| 选择 | Runtime | 结果 |
|---|---|---|
| host 自己写图 | dune | 最大控制，最大语义负担 |
| 把图放进 engine/core 层 | Deno | 语义完整，代码量大 |
| 把图放进 JS | Node | 策略灵活，C++/JS 边界复杂 |

### 8.2 QuickJS 把图的所有权留给了 engine

LLRT 和 txiki 都只实现 resolver/loader，QuickJS 自己负责：

- 递归依赖；
- link/instantiate；
- evaluate；
- dynamic import Promise；
- module cache。

因此它们的 host 代码明显更薄：

- LLRT 的重点在 native module + CJS 兼容；
- txiki 的重点在 C API + build pipeline。

### 8.3 Builtin 交付的四种形态

| 形态 | 代表 | 优点 | 代价 |
|---|---|---|---|
| JS 源码内嵌，运行时编译 | dune | 简单、可读 | 启动慢；依赖需要 ESM 图 |
| native `ModuleDef` + global primordials | LLRT | 快；无需 ESM 解析 | 依赖隐式；需要初始化顺序 |
| 预编译 bytecode | LLRT、txiki | 构建期编译；启动快 | 构建复杂；调试稍难 |
| ESM source / snapshot / lazy facade | Deno (Extension), Node (`BuiltinModule` + ESM facade) | 统一模块模型；支持懒加载/Node facade | 架构复杂；需要 snapshot/source 支持 |

### 8.4 Builtin 之间的依赖

| Runtime | 依赖形态 | 例子 |
|---|---|---|
| dune | 真实 ESM import | `main -> @web/fetch -> http -> net -> dns`；`test -> fs` |
| LLRT | native 共享 crate/global + JS builtin import native | `fs -> llrt_buffer`/global `Buffer`；`stream -> node:events`、`node:util` |
| txiki | core 构建期 bundle；stdlib 运行期 import internal | `tjs:path -> tjs:internal/path`；`tjs:hashing -> tjs:internal/core` |
| Deno | extension ESM import；lazy/synthetic | `node:stream` 等 lazy polyfill；`node:url` 等 synthetic ESM |
| Node | builtin map + internal require + ESM facade | CJS builtin 之间 `require`；ESM `node:fs` 通过 facade |

一个关键区别：

- **dune/Deno** 把 builtin 当作真正的 ESM 模块图的一部分；
- **Node** 通过 `BuiltinModule` + ESM facade 把 CJS builtin 接入 ESM 图；
- **LLRT** 的 native builtin 不是 ESM 节点，但 JS builtin 可以是；
- **txiki** 的 core 内部依赖被 esbuild 吃掉，stdlib 才保留运行期依赖。

### 8.5 CJS / npm 是最大的分界线

| Runtime | CJS | npm | 策略 |
|---|---|---|---|
| Node.js | 原生 | 原生 | CJS 优先，ESM 后加并桥接 |
| Deno | Node compat | 原生 | ESM 优先，npm/Node 作为兼容层 |
| LLRT | 部分 | Node resolver + bundling | Lambda 场景，CJS 为 AWS SDK 服务 |
| dune | 无 | 无 | import map / URL / 预打包 |
| txiki.js | 无 | 无 | Web/URL/import map，刻意不做 Node |

### 8.6 Module cache / dedupe

| Runtime | 缓存 key | 并发 import 处理 |
|---|---|---|
| dune | path；`ModuleMap.by_path` + `seen` + `Duplicate` | `same_origin` 合并 resolver |
| LLRT | QuickJS module cache；ESM `ModuleCache` by path；CJS `RequireState` | engine 自己处理 |
| txiki | QuickJS module cache；builtin 按 name | engine 自己处理 |
| Deno | `ModuleSpecifier` + requested type；`ModuleId` | `RecursiveModuleLoad` + `visited` + dynamic import map |
| Node | ESM `loadCache(url, type)`；CJS `Module._cache[filename]`；builtin map | `ModuleJob` promise identity |

### 8.7 Dynamic import / TLA 的交付语义

| Runtime | Dynamic import 状态机 | TLA |
|---|---|---|
| dune | host resolver + `ModuleGraph` + `fast_forward` | 不等待；namespace 可能先于 TLA 完成 |
| LLRT | QuickJS engine + loader | engine 处理；宿主 liveness 另说 |
| txiki | QuickJS engine + loader | engine 处理；宿主 liveness 另说 |
| Deno | `ModuleMap` + `RecursiveModuleLoad` + evaluation promise | 等待；`pending_dyn_mod_evaluations` |
| Node | JS `ModuleLoader` + `ModuleJob` + V8 `ModuleWrap` | 等待 async graph；`ModuleJobSync` 处理同步图 |

### 8.8 启动与懒加载

| Runtime | 构建期 | 启动期 | 懒加载 |
|---|---|---|---|
| dune | 几乎不做 | 按 import 图解析 builtin，`main.js` 拉起大部分 | 未 import 的 builtin |
| LLRT | 编译 JS builtin 为 bytecode | 注册 native module / 解压 bytecode | 首次 import |
| txiki | esbuild + qjsc | core/polyfills eager | stdlib `tjs:*` |
| Deno | snapshot + `deno_graph` + emit/code cache | extension entry point | `lazy_loaded_esm` |
| Node | js2c + startup snapshot | builtin map / CJS bootstrap | CJS require / 动态 ESM |

---

## 9. 一张“谁负责什么”的总结表

| 阶段 | dune | LLRT | txiki | Deno | Node |
|---|---|---|---|---|---|
| resolve | host Rust | QuickJS loader + Node resolver | QuickJS normalizer + import map | `deno_core` loader + CLI resolver | JS `defaultResolve` |
| load | crabuv worker | native / bytecode / fs | C / bytecode / HTTP / fs | `deno_graph` + emit cache | `readFileSync` / hooks |
| compile | host V8 | QuickJS | QuickJS | CLI transpile + `deno_core` compile | translators + V8 |
| graph | host `ModuleMap` tree | engine | engine | CLI graph + `deno_core` `ModuleMap` | JS `ModuleJob` |
| link | host | engine | engine | `deno_core` | JS + V8 |
| evaluate | host | engine | engine | `deno_core` | JS + V8 |
| builtin | JS ESM | native + bytecode | C + bytecode | ESM extension | `BuiltinModule` map |
| CJS | 无 | 部分 | 无 | Node compat | 原生 |

---

## 10. 对 rsvim 的映射

rsvim 当前是 **V8 + host-owned module graph**，设计上最接近 dune：

- `ModuleMap` / `EsModule` / `ModuleGraph` / `fast_forward` 都是 dune 的移植；
- 静态 builtin 初始化在 `JsRuntime::new` 中通过 `init_builtin_modules` 完成；
- 当前两个 builtin bundle（`00__web.min.js`、`01__rsvim.min.js`）没有静态 import，因此 `init_builtin_modules` 使用 `fetch_module` 就够了。

但如果以后要把 builtin 拆成互相 import 的模块，当前路径不够：

- `fetch_module` 不会把 referrer 放进 `module_map`；
- `module_resolve_cb` 会去 `module_map` 查找 referrer，找不到会 panic；
- 需要选择：
  1. **dune/Deno 路线**：把 builtin 注册进 `CORE_MODULES`，改用 `fetch_module_tree`；
  2. **txiki/LLRT 路线**：构建期 bundle/预编译成无 import 的模块；
  3. **LLRT native 路线**：把部分 builtin 做成 native module + global primordials。

---

## 11. 证据索引

> 行号会随上游提交漂移；优先按函数/结构名搜索。

### dune

| 内容 | 位置 |
|---|---|
| `CORE_MODULES` | `src/modules.rs` |
| `ModuleMap` / `EsModule` / `ModuleGraph` | `src/modules.rs` |
| `fast_forward` | `src/modules.rs` |
| `fetch_module_tree` | `src/modules.rs` |
| dynamic import callback | `src/hooks.rs` |
| `fast_forward_imports` | `src/runtime.rs` |
| builtin 入口 | `src/js/main.js` |

### LLRT

| 内容 | 位置 |
|---|---|
| module builder / native module 注册 | `llrt_modules/src/module_builder.rs` |
| module resolver | `llrt_modules/src/module/resolver.rs` |
| package resolver | `llrt_modules/src/package/resolver.rs` |
| package loader / CJS | `llrt_modules/src/package/loader.rs` |
| bytecode cache 生成 | `llrt_core/build.rs` |
| JS builtin 源 | `llrt_core/src/modules/js/stream.ts` |
| `node:` 去前缀 | `llrt_modules/src/module/resolver.rs` |

### txiki.js

| 内容 | 位置 |
|---|---|
| loader 注册 | `src/vm.c` |
| normalizer / loader | `src/modules.c` |
| builtin 表 | `src/builtins.c` |
| core bundle 构建 | `Makefile` |
| core 入口 | `src/js/core/index.js` |
| stdlib 依赖 | `src/js/stdlib/path.js`、`hashing.js`、`sqlite.js`、`wasi.js` |
| internal path holder | `src/js/internal/path.js` |

### Deno

| 内容 | 位置 |
|---|---|
| `ModuleLoader` trait | `libs/core/modules/loaders.rs` |
| `RecursiveModuleLoad` | `libs/core/modules/recursive_load.rs` |
| `ModuleMap` | `libs/core/modules/map.rs` |
| dynamic import / TLA | `libs/core/modules/map/dynamic.rs` |
| extension 加载 | `libs/core/extension_set.rs` |
| extension entry point | `libs/core/runtime/jsruntime.rs` |
| CLI module loader | `cli/module_loader.rs` |
| Node polyfills | `ext/node/lib.rs` |
| Node synthetic ESM | `ext/node/lib.rs` |

### Node.js

| 内容 | 位置 |
|---|---|
| CJS loader | `lib/internal/modules/cjs/loader.js` |
| ESM loader | `lib/internal/modules/esm/loader.js` |
| resolve | `lib/internal/modules/esm/resolve.js` |
| load | `lib/internal/modules/esm/load.js` |
| module job | `lib/internal/modules/esm/module_job.js` |
| CJS/ESM bridge | `lib/internal/modules/esm/translators.js` |
| builtin map | `lib/internal/bootstrap/realm.js` |
| loader hooks | `lib/internal/modules/esm/hooks.js` |
| V8 trampoline | `src/module_wrap.cc` |

---

## 12. 结论

把这五个 runtime 放在一条光谱上：

```text
host/embedder-owned        engine/core-owned                    JS-owned

dune                       Deno (deno_core)                     Node
                           LLRT / txiki (QuickJS)
```

- **dune**：V8 的“手工 module graph”参考实现，host 拥有全部状态机。
- **LLRT**：QuickJS engine 拥有图，host 提供 native module + bytecode + Node resolver 的兼容层。
- **txiki.js**：QuickJS engine 拥有图，host 提供单 loader + 构建期 bundle + Web 风格解析。
- **Deno**：`deno_core` 拥有运行时图，CLI 用 `deno_graph` 拥有解析/构建策略，builtin 是 ESM extension。
- **Node.js**：JS 拥有 ESM 状态机，CJS 是独立同步 loader，两者通过 synthetic namespace 和 `require(esm)` 桥接。

选择哪一种，本质上是在回答三个问题：

1. **你愿意自己维护 module graph 吗？**
2. **你需要 Node/npm/CJS 生态吗？**
3. **你希望 builtin 是 ESM 源文件、native module，还是预编译 bytecode？**

这三个问题的答案，基本决定了 runtime 的 module 架构。
