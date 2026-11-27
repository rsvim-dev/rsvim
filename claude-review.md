# rsvim 架构 Review：JS Runtime / Async / Event Loop

> - 审阅对象：`rsvim` @ `643dad9`（分支 `arch2`，"refactor(js): move js/mod.rs to js.rs"）
> - 审阅日期：2026-11-27
> - 参考实现：
>   - Deno `fe7ed012`（`libs/core`，即 deno_core 0.413.0）
>   - Node.js `bc2e01dc`
>   - LLRT `0a10758`（rquickjs 0.12.2）
>   - Dune `5958b64`，外加其事件循环 crate crabuv `2fda099`
> - 参考文档：
>   - `rfc/Investigation/` 下的 `js-engine.md`、`js-runtime-event-loop.md`、`js-runtime-modules.md`
>   - `rfc/2-EventLoop.md`、`rfc/3-JavaScriptRuntime.md`、`rfc/3-JavaScriptRuntime/2-ImportModules.md`
> - 方法：
>   - 通读 `rsvim_core/src/{evloop.rs, chan/, js/}`，以及 `src/00__web.ts`、`rsvim_cli`。
>   - 跑了 `cargo test -p rsvim_core --lib`，结果是 **708 passed / 14 failed**。
>   - 对关键结论做了针对性复现，并对照 quickjs-ng 源码（rquickjs-sys 0.14.0 vendored）核实。
> - 文中 `file:line` 若无前缀，均指 `rsvim_core/src/` 下的文件。

---

## 0. 结论摘要

当前 `arch2` 分支的设计是**宿主拥有事件循环，JS 引擎只是一个同步对象**：

- QuickJS 被 `JsHost` 封装为同步对象。
- 所有进入 JS 的路径都走 `JsHost::enter`。
- 异步 op 采用"Promise + OpId 表 + 只传 `Send` 数据的 tokio task + 通道回送"。
- 计时器是宿主侧的一张表。

这个骨架是对的。在几个方面，它比四个参考实现更清晰：

- microtask checkpoint、rejection 上报、重绘标记集中在一处。
- 引擎内没有隐藏的调度器。
- 退出时区分"可丢弃 / 必须等待"的后台任务。

主要问题不在骨架，而在骨架的**边缘契约**：

1. **模块加载**：用户配置里的相对导入 `import "./x.js"` 当前直接失败。这是已复现的 bug，11 个用例失败。JSON 导入还存在模板字符串注入。
2. **输入与 JS 的时序**：`:Ex` 命令被异步投递，同一批按键中排在其后的键会先于命令执行。这违反 RFC-2 的"同步一致"原则。
3. **退出语义**：退出时只等待"在途的 tokio task"，不运行它们的 JS 续体，多步写文件会被截断。子进程 `wait()` 可以让 `:q` 永久挂起。
4. **失控脚本没有逃生通道**：`exec_timeout` 默认关闭；raw mode 下 Ctrl-C 不是信号，且只有主线程读输入。`while(true){}` 或无限 promise 链只能靠 `kill -9`，未保存的 buffer 会丢失。
5. **文档漂移**：RFC-2/RFC-3 仍描述 V8 时代的机制（`Notify` 唤醒、退避计时器、退出时报告 stalled TLA），与实现不符。

### 问题一览

| # | 级别 | 问题 | 位置 | 状态 |
|---|---|---|---|---|
| 4.1 | **P0** | 相对 import 解析失败（把文件路径当目录传给 oxc_resolver） | `js/fs_loader.rs:101-117` | 已复现，11 个测试失败 |
| 4.2 | **P0** | JSON 模块用模板字符串拼接：`${}` 会执行代码，转义被破坏 | `js/fs_loader.rs:27-30` | 代码确认 |
| 4.3 | **P0** | `:Ex` 命令与同批后续按键乱序，buf/win 上下文过期 | `state/fsm/cmdline_ex.rs:110`、`evloop.rs:1135` | 代码确认 |
| 4.4 | **P0** | 退出只等在途 op，不跑 JS 续体，多步写入被截断 | `evloop.rs:897-901` | 代码确认 |
| 4.5 | **P0** | 子进程：`wait()` 使退出挂死；孤儿进程；`wait` 期间一直持有 Mutex | `js/binding/process_async.rs:27-60`、`js/process_command.rs:69` | 代码确认 |
| 4.6 | P1 | 失控 JS 或无限 microtask 无法中断 | `evloop.rs:449`、`js/host.rs:182` | 设计缺口 |
| 4.7 | P1 | 每次 `:js` 生成新模块且永不释放；模块不可重载 | `js/command/mod.rs:72`、quickjs.c `js_free_modules` | 源码确认 |
| 4.8 | P1 | 顶层 await 挂起时静默，与 RFC-2 描述不符 | `js/host.rs:257-281` | 代码确认 |
| 4.9 | P1 | dirty 粒度过粗且无帧率控制，1ms 的 interval 会导致每毫秒重绘一次 | `js/host.rs:239-247`、`evloop.rs:1043` | 代码确认 |
| 4.10 | P1 | 资源没有 GC 回收，op 无法取消 | `js/resource.rs`、`js/ops.rs` | 设计缺口 |
| 4.11 | P1 | `import.meta` 未填充 | `js/fs_loader.rs` | 测试失败 |
| 4.12 | P1 | 错误路径绕过 `close_trackers` 和 `shutdown` | `evloop.rs:979-991`、`rsvim_cli/src/bin/rsvim.rs:84-99` | 代码确认 |
| 4.13–4.19 | P2 | GC/内存参数、字节码预编译、rejection 细节、计时器细节、`encodeInto`、测试基建、headless 存活策略 | 见正文 | — |
| 5 | P1 | RFC 和代码注释与实现漂移 | `rfc/2-EventLoop.md` 等 | — |

---

## 1. 当前架构速览

```text
 ┌──────────────────────── main thread（rt.block_on，rsvim.rs:84）────────────────────────┐
 │                                                                                         │
 │  Rsvim::tick (evloop.rs:1025)                                                           │
 │   1. settle：tick_event_loop → pop_ready()                                              │
 │        优先级 master_rx > jsrt_rx > 到期 timer；单轮上限 256 个事件或 8ms (194/197)       │
 │   2. render_if_dirty()                                                                  │
 │   3. select!（非 biased）{                                                              │
 │        jsrt_rx.recv()     ← ExCommandReq（FSM）/ OpDone（tokio task）                    │
 │        master_rx.recv()   ← ExitReq / SyntaxEditReq / SyntaxEditResp                    │
 │        io.next()          ← crossterm EventStream，命中后再 drain 最多 64 个              │
 │        cancellation_token                                                               │
 │        poll_timer()       ← Timers：BTreeMap<(deadline, seq)> + 单个 Sleep               │
 │      }                                                                                  │
 │                                                                                         │
 │  所有 JS 入口 ──► JsHost::enter(f) (js/host.rs:239)                                      │
 │        设置 deadline → f(ctx) → drain_jobs（跑空 job 队列）                              │
 │        → 清除 deadline → flush_rejections → dirty = true                                │
 └─────────────────────────────────────────────────────────────────────────────────────────┘
        ▲ Completion{ id, Box<dyn Any+Send> }（jsrt_tx）
        │
 ┌──────┴──────────── tokio worker / blocking pool ────────────┐
 │ spawn_op (js/ops.rs:198)：future 只能捕获 Send 数据          │
 │ OpKind::Blocked → blocked_tracker（退出时等待）               │
 │ OpKind::Detached → detached_tracker（退出时只 close）         │
 └──────────────────────────────────────────────────────────────┘
```

关键不变量（代码注释写得很清楚）：

- **I1 单一入口**：所有执行 JS 的路径都走 `JsHost::enter`，包括模块求值、`:Ex`、timer、op 完成。microtask checkpoint、rejection 上报、dirty 标记只在这一处实现。
- **I2 引擎内无调度器**：没有使用 rquickjs 的 `AsyncRuntime` / `ctx.spawn`。JS 侧所有"稍后执行"的工作，都以事件的形式出现在宿主循环中。
- **I3 跨线程只传纯数据**：`Persistent`、`Ctx` 不出主线程。需要跨 await 保活的 JS 值放在 `aux` 中，由主线程持有（例如 `fs.read` 的 ArrayBuffer）。
- **I4 计时器是表不是 future**：`clearTimeout` 直接从表中删除，回调立即释放；同一 deadline 按创建顺序（FIFO）触发。

---

## 2. 与四个参考运行时的横向对比

| 维度 | **rsvim** | **Deno** | **Node** | **LLRT** | **Dune** |
|---|---|---|---|---|---|
| 引擎 / 底座 | QuickJS-ng（rquickjs 0.14 同步 `Runtime`）+ tokio multi-thread | V8 + tokio current_thread（`runtime/tokio_util.rs:29`） | V8 + libuv | QuickJS-ng（rquickjs 0.12 `AsyncRuntime`）+ tokio multi-thread | V8 + crabuv（mio + rayon） |
| 循环驱动 | 宿主 `select!` + settle 预算 | `poll_event_loop_inner` 共 6 个 phase（`jsruntime.rs:2425-2643`） | `SpinEventLoopInternal` + `uv_run` phase（`embed_helpers.cc:23-78`、`core.c:427-492`） | `vm.idle()`：跑空 job，再轮询一次 spawner（`runtime/async.rs:301-348`） | `run_event_loop`：tick 加 `run_pending_futures`（`runtime.rs:442-476`） |
| 异步 op 桥 | OpId → `PendingOp{resolve, reject, aux, reply}`；`Send` task 完成后经 mpsc 回送，循环中 settle | JS 分配 PromiseId；future 存在 JS 线程上的 `FuturesUnordered`；每 tick 通过 `__eventLoopTick` **批量**回送（`01_core.js:459`、`jsruntime.rs:3551`） | `ReqWrap` → libuv 线程池 → 循环线程 `MakeCallback` | `Promise::wrap_future` + `ctx.spawn`：future 持有 `Ctx`，在 JS 线程原地 settle（`promise.rs:48-79`） | rayon worker 完成 → 回调推入 `JsFuture` → `run_pending_futures` 中 settle（`file.rs:126-159`） |
| microtask checkpoint | **每个入口一次**（每个回调 / 每个完成） | op：**每批一次**；timer / immediate：每个回调之间一次（`02_timers.js:285-289`） | **每个顶层回调一次**（`InternalCallbackScope::Close`，`callback.cc:138-213`） | 每个调度 pass 一次；timer 每个回调之后一次（`llrt_timers/src/lib.rs:384`） | **每个 future 一次**（`runtime.rs:489-501`） |
| 计时器 | Rust `BTreeMap<(deadline, seq)>` + 单个 `Sleep`；最小 1ms；interval 为固定频率并跳过错过的 tick；无 unref | JS 侧按 duration 分链表 + 优先队列；Rust 侧单个 `UserTimer`；有 ref/unref | 同左（Deno 照搬 Node）；有 ref/unref | 无序 `Vec` + 4ms 下限；同 deadline **不保证 FIFO**（`lib.rs:361`） | `BTreeMap<Instant, Vec<Id>>`；无 unref |
| 未处理 rejection | tracker 记录；每次 `enter` 末尾 flush；"handled later"会撤销；**只报告，不退出** | 默认终止（`terminate_execution`）→ 返回 `Err` | 默认 `throw` → 退出码 1 | **未安装 tracker，静默** | `exit(1)`，支持 `process.on` 钩子 |
| 模块 | QuickJS 自管模块图；同步 resolver（oxc）+ loader；TLA 挂起视为 `Ok` | `RecursiveModuleLoad` 异步加载；检测 stalled TLA | 异步 `ModuleJob`；TLA 未决则退出码 13 | 同步 resolver/loader 链 + 内嵌字节码；入口包成 `import()` | 自建 `ModuleMap`，异步加载；忽略 TLA promise |
| 失控脚本 | 可选 interrupt handler 超时，**默认关闭** | 无内置；embedder 从其他线程调用 `IsolateHandle::terminate_execution` | `vm` timeout 的 `Watchdog` 线程 + SIGINT watchdog，结束后抛出**可捕获**错误（`node_contextify.cc:1287-1315`） | 无 | 无 |
| 关闭 | `Drop` 先清 `Persistent`；`close_trackers` 等待 blocked task；最后 `process::exit` | 先 `pending_ops.shutdown`，再 realm，再 `op_state`，最后 isolate（`jsruntime.rs:250-335`） | 先 `FreeEnvironment`，再 `RunCleanup`，最后 `DisposeIsolate` | 先 `opaque.clear()`，再 `JS_FreeRuntime` | `process::exit` |
| 存活判定 | 不适用，由用户退出 | ref'd op / timer / 模块 / tick / rejection 等 | `uv_loop_alive` + `beforeExit` | spawner 为 `Empty` | 资源、请求、线程池均为空 |

几点观察：

- **op 桥**：rsvim 的形态最接近 Dune（完成队列 + 主线程 settle），也与 Deno 的 PromiseId 关联相似。它与 LLRT（future 持有 `Ctx`、原地 settle）是两种相反的设计。investigation 文档 §9 第 2 条指出"两种都可行，取决于引擎 API 与生命周期设计"。rsvim 选择完成队列，换来了 **I2**：循环能看到每一次 JS 运行，从而能精确标脏、计数、清理。对一个需要"JS 跑完就重绘"的编辑器来说，这是正确的取舍。
- **checkpoint 粒度**：rsvim 是"每个回调一次"，与 Node、Dune 一致，比 Deno 的 op 批处理更细。investigation 实验"两个 timer，第一个调度 `.then`"中，各实现都输出 `timer-1, then-1, timer-2`，rsvim 也满足（`host.rs` 模块文档）。代价是：每个 op 完成都要一次 `enter`，而 Deno 每 tick 一次 JS 调用就能处理约 341 个 op。对编辑器的负载量级而言可以忽略。
- **rejection 策略**：四个参考实现中，三个直接退出，LLRT 静默。rsvim 选择"报告但不退出"，是唯一适合编辑器的策略：插件出错不能杀死编辑器，更不能丢失 buffer。

---

## 3. 做得好的地方（建议保持）

1. **`JsHost::enter` 单一入口**（`js/host.rs:239-247`）。这相当于 Node `InternalCallbackScope` 的思想，但更彻底。checkpoint、rejection、watchdog deadline、dirty 都在这一处，调用点无法遗漏。模块文档把 4 条规则写得很清楚。
2. **op 完成队列**（`js/ops.rs`）：
   - `spawn_op` 的 future 被类型约束为 `Send + 'static`，无法捕获 JS 值。
   - `reply` 闭包在主线程上构造返回值。
   - `aux` 机制在 await 之后按 buffer 的**当时**长度写入，避免了 resize 后越界（`fs_async.rs` 中的 `read`）。
   
   这比 LLRT 在 future 中直接持有 `Ctx` 更容易推理，也不依赖 rquickjs 的私有调度器。
3. **计时器表**（`js/timers.rs`）：
   - 用 `(deadline, seq)` 作键，保证 FIFO。注释说明了 `DelayQueue` 同槽逆序的坑，LLRT 就没有处理这一点（`sort_unstable_by_key`）。
   - interval 在回调**之前**重新 arm，所以回调内的 `clearInterval` 删除的是正确的条目。
   - `poll_due` 返回 `Ready(None)` 时禁用 select 分支，不会空转。
4. **公平性**：
   - `select!` 刻意不用 `biased`（`evloop.rs:1048-1052`）。
   - settle 阶段有 256 个事件 / 8ms 的预算。
   - 输入批处理用的是循环自身的 waker（`evloop.rs:1124-1134` 的注释解释了 crossterm `EventStream` 的 waker 陷阱）。
   
   这正对应 investigation 文档 §9 第 7 条"每一层都要检查公平性"。
5. **退出分级**：`OpKind::Blocked` 与 `Detached` 的区分（`js/ops.rs:47`），以及 `fs_async.rs` 头部关于"按对外部世界的影响而不是实现方式来分类"的论述，都是对的。它比 Dune / LLRT 的直接 `process::exit` 更负责任。4.4 指出了它还没覆盖的部分。
6. **`Persistent` 先于 runtime 释放**（`js/host.rs:414-418`）：用 `Drop` 绑定所有权，而不是绑定代码路径。这与 LLRT（`opaque.clear()` 先于 `JS_FreeRuntime`）和 Deno（先 shutdown ops，再销毁 realm）的做法一致。
7. **rejection 的 handled-later 处理**：QuickJS 会对"晚绑定的 handler"回调两次。tracker 只做记录，drain 之后再 flush。效果等价于 Node 在 `processTicksAndRejections` 末尾处理 `pendingUnhandledRejections`（`lib/internal/process/promises.js:369-416`）。
8. **tokio multi-thread**：JS 长时间运行时，后台 op 仍能推进。`JsHost` 是 `!Send`，被钉在 `block_on` 的主线程上，不需要 `LocalSet`。

---

## 4. 问题与建议

### P0：正确性与数据安全

#### 4.1 相对 import 解析失败（已复现）

**现象**：

```text
$ cargo test -p rsvim_core --lib js::module_map_tests::test_static_import::xdg_config_dir1
right: Some("Uncaught ReferenceError: Error resolving module './util.js' from '/tmp/.tmp1CfLRt/rsvim/rsvim.js'")
```

全量测试中 `js::module_map_tests` 有 11 个用例失败，包括 `xdg_config_dir1-5`、`home_dir1/2`、`*_failed1/2`、`test_import_meta::*`。

**原因**：

- QuickJS 回调 resolver 时，传入的 `base` 是**导入方模块的名字**，即文件路径 `/…/rsvim.js`。
- `fs_loader.rs:113` 直接把它交给 `oxc_resolver::Resolver::resolve(directory, specifier)`。
- oxc_resolver 11.21 的文档要求 `directory` 是"an absolute path to a directory"（`oxc_resolver-11.21.0/src/lib.rs:204-217`）。
- 结果，`./util.js` 被解析成 `/…/rsvim.js/util.js`。

`fs_loader_tests.rs` 中的单元测试全部通过，因为它们传入的 `base` 本身就是目录（如 `fs_loader_tests.rs:70-71`）。**测试验证的是一个与引擎契约不同的 API**：

- `module.rs:18` 中的 `ModuleLoader` trait 只被测试使用。
- 生产路径走的是 `fs_loader.rs:164` 的 rquickjs `JsResolver` impl。

**建议**：

1. 对真实文件路径的 `base`，改用 `resolver.resolve_file(base, specifier)`（oxc 专门为"相对于文件解析"提供的 API，`lib.rs:231`），或者先取 `Path::parent()`。
2. 对合成名字的 `base`，需要单独定义解析基准。例如 `<command-js:N>`、`00__web.js` 这类名字，可以以 cwd 或 `config_home` 为基准，并写入文档。
3. 删除或下沉 `ModuleLoader` trait，让测试直接通过 `JsHost::execute_module` 走引擎路径。

参考：`js-runtime-modules.md` §2.2 指出，QuickJS 的 normalizer 收到的是 `(base_name, name)`。txiki、LLRT 的 normalizer 都必须自己从 base 计算目录。

#### 4.2 JSON 模块用模板字符串拼接

`fs_loader.rs:27-30`：

```rust
fn wrap_json(source: &str) -> String {
  format!("export default JSON.parse(`{source}`);")
}
```

- **`${…}` 会被求值**：一个内容为 `{"a": "${globalThis.x = 1}"}` 的 JSON 文件，被 import 时就会执行代码。插件可能从 `node_modules` 导入第三方 JSON 数据，因此这是一个"数据即代码"的注入面。
- **转义被破坏**：JSON 中的 `\n`、`\"`、`\\` 先被模板字面量解释，之后 `JSON.parse` 会因控制字符报 SyntaxError，或得到错误的值。内容含反引号时直接是语法错误。

**建议**：先用 `serde_json::from_str::<serde_json::Value>` 校验，再生成 `export default <source>;`（合法 JSON 就是合法的 JS 表达式）。另一种写法是 `JSON.parse(<serde_json::to_string(source)>)`。还可以考虑要求 `with { type: "json" }` 导入属性，与 Node/Deno 对齐。注释里的 "using v8's built in objects" 也需要更新。

#### 4.3 `:Ex` 命令与后续按键乱序

**路径**：

- `state/fsm/cmdline_ex.rs:96-120`：按下 `<CR>` 后，把 `ExCommandReq` 投递到 `jsrt_tx`，然后**立即**返回 `State::Normal`。
- `evloop.rs:1135-1159`：`drain_input` 继续处理同一批中剩余的最多 63 个按键。
- 命令要到**下一次** `tick` 的 settle 阶段才执行。

**后果**：

- 粘贴，或将来映射展开出的 `:cmd<CR>dd`，会先执行 `dd`，再执行 `cmd`。
- `ExCommandReq` 中的 `current_buf_id`、`current_win_id` 在入队时就已捕获。如果后续按键切换了窗口，命令拿到的就是过期上下文。
- 将来如果命令改变模式或布局（打开窗口、`:normal` 一类），后续按键已经作用在错误的状态上。

这与 `rfc/2-EventLoop.md` 的核心原则直接冲突："process should always be synchronous… input => calculation => output"。Vim/Neovim 的 ex 命令和映射在输入处理中**同步**执行，延迟执行必须显式声明（`vim.schedule()`）。

**建议**：

- **最小修复**：在 `drain_input` 中，一旦某个按键让 FSM 投递了 JS 工作，就结束本批，让下一轮 settle 先执行命令。判断方式可以是 `!self.jsrt_rx.is_empty()`，或者让 `on_input` 返回一个标志。
- **推荐修复**：让 FSM 返回 `Effect`（如 `Effect::RunEx(req)`），由循环在处理下一个按键**之前**同步执行 `command.run(&mut self.js)`。通道只保留给真正异步的工作。这也为将来"JS 回调实现的按键映射"打好了时序基础。

#### 4.4 退出语义：等的是"在途 op"，不是"JS 承诺链"

`evloop.rs:897-901` 的 `close_trackers` 只等待 `blocked_tracker` 中的 tokio task。循环 break 之后，没有人再消费 `jsrt_rx`。因此：

```js
const f = await fs.open(p, { write: true, truncate: true }); // Blocked
await f.write(header);   // 如果用户此时 :q，这一步之后的续体不会运行
await f.write(body);     // 永远不会发出
```

用户在 `open` 完成之后退出，会得到一个**被截断的空文件**。这正是 `Blocked` 分类想防止的情况。现有测试 `write_survives_immediate_exit`（`evloop_tests.rs:377`）只覆盖了"单个未 await 的 write"。

对照：

- Node 和 Deno 在自然退出前会一直运行事件循环，直到没有 ref'd op 为止，所以续体都会执行。显式的 `process.exit()` 则明确放弃一切，这是有文档说明的语义。
- investigation 文档 §9 第 3 条把这种情况称为 "completion handoff liveness"：结果已经就绪但尚未交给 JS，这段时间也必须被某个 registry 或不变量覆盖。

**建议**：增加一个显式的 **quit drain 阶段**。

1. 收到 `:q`（优雅退出）后，停止读取输入。timer 是否继续可以按策略决定，建议继续。
2. 继续消费 `jsrt_rx` 中的完成事件并执行 microtask，直到 `OpTable` 中 **blocked op 计数归零**。这需要在 `PendingOp` 上记录 `kind`。续体发起的新 blocked op 会自然被计入。
3. 设置超时（例如 3s）。超时后在命令行提示"仍有 N 个写入未完成，再按一次强制退出"。
4. 明确区分 `process.exit()`（立即退出，与 Node 一致）和 `:q`（优雅退出），并写入 RFC。

#### 4.5 子进程：退出挂死、孤儿进程、`wait` 期间持锁

**相关代码**：

- `js/process_command.rs:69` 使用 `std::process::Command`，`Child` 被放进全局 `ResourceTable`。
- `js/binding/process_async.rs:27-60`：`wait()` 是 `OpKind::Blocked`，在 blocking pool 中执行 `lock!(resource).wait()`，**整个等待期间都持有该 Child 的 Mutex**。
- JS 侧没有 `kill()`。

**后果**：

1. 配置中 `await p.wait()` 一个长时间运行的子进程（如 watcher 或 `sleep 100`）后，`:q` 会一直等到子进程结束。`close_trackers` 没有超时，`spawn_blocking` 也无法取消。
2. 没有被 wait 的子进程在编辑器退出后成为孤儿进程，因为 std 的 `Child` drop 时不会 kill。
3. 只要将来有任何同步 binding（如 `kill`、`pid`）去锁同一个 rid，**主线程就会阻塞到子进程退出**。

对照：

- Neovim 退出时会停止所有 job。
- Node 的 child_process 有 `kill`，并支持 ref/unref。
- Deno 的 `ChildProcess` 是 resource，runtime drop 时统一关闭（`jsruntime.rs:250-335`）。

**建议**：

- 改用 `tokio::process::Command` 加 `kill_on_drop(true)`，`wait()` 改为不持锁的异步等待。
- 提供 `kill()`。
- 退出时对子进程先发 SIGTERM，宽限期后再 SIGKILL。
- 同时给 `close_trackers` 加超时（见 4.4）。

### P1：架构风险

#### 4.6 失控脚本没有逃生通道

**现状**：

- `JsRuntimeOptions::exec_timeout` 默认是 `None`（`evloop.rs:449`、`js/runtime.rs:46-57`），因此 interrupt handler **根本没有安装**（`js/host.rs:182-188`）。
- `drain_jobs`（`js/host.rs:364-377`）会无限跑完 job 队列。

**后果**：

- 配置或插件中的 `while(true){}`，或者一条无限的 promise 链（investigation 实验"无限 promise 链 + 10ms timer"，四个 runtime 都没有跑到 timer），都会让编辑器永久冻结。
- raw mode 下 Ctrl-C 只是一个按键，而读取输入的唯一线程就是正在跑 JS 的主线程。
- 用户只能 `kill -9`，未保存的 buffer 全部丢失。

代码注释说这是 "evloop-plan-v2.md decided to accept" 的取舍。但这份文档不在 rfc 仓库中（见 §5）。对编辑器而言，这个代价比对 CLI runtime 要大得多。

**对照**：

- **Node**：`Watchdog` 在独立线程上运行 uv_loop 和定时器，到期后调用 `TerminateExecution()`。`SigintWatchdogHelper` 用信号量把 SIGINT 转给 watchdog 线程（`node_watchdog.cc:51-109, 715-774`）。执行结束后先 `CancelTerminateExecution`，再抛出**可捕获**的 `ERR_SCRIPT_EXECUTION_INTERRUPTED`（`node_contextify.cc:1287-1315`）。
- **Deno**：没有内置机制，embedder 从其他线程调用 `IsolateHandle::terminate_execution`。测试 runner 有 watchdog 线程（`cli/tools/test/mod.rs:1092-1180`）。
- **LLRT、Dune**：都没有。
- **Vim**：长操作中通过 `ui_breakcheck()` / `line_breakcheck()` 轮询输入，检测 CTRL-C 后设置 `got_int`。

**建议**（Vim breakcheck 与 Node SIGINT watchdog 的结合）：

1. **总是安装 interrupt handler**。QuickJS 每 `JS_INTERRUPT_COUNTER_INIT = 10000` 次计数才调用一次（quickjs.c:556），handler 中只做一次原子读，开销可以忽略。
2. 把终端读取移到**独立的输入线程**，用阻塞的 `crossterm::event::read` 读取，再通过 channel 交给主循环。输入线程识别到 Ctrl-C（必要时再加 Ctrl-\）时，设置 `AtomicBool INTERRUPT`，同时照常转发事件。
3. handler 返回 `INTERRUPT || deadline 已过`。被中断的回调抛出不可捕获异常，由 `enter` 统一报告 "Interrupted"，然后清除标志。
4. 无限 promise 链中，每个 job 都很短。但 `interrupt_counter` 是 runtime 级的，跨 job 累计，所以在约一万步内必然命中某个 job 并打断这条链。
5. `exec_timeout` 可以保留为可选的软上限（例如给 config 求值设一个较宽松的默认值）。

#### 4.7 每次 `:js` 生成新模块且永不释放；模块不可重载

- `js/command/mod.rs:72` 为每次 `:js` 生成唯一的模块名 `<command-js:{task_id}>`，然后走 `Module::evaluate`。
- QuickJS 会把每个模块挂到 `ctx->loaded_modules`。按模块名查找是线性的（`js-runtime-modules.md` §2.2）。
- QuickJS 只会释放 **未 resolve** 的模块（`js_free_modules(ctx, JS_FREE_MODULE_NOT_RESOLVED)`，quickjs.c:2966-2980、31834、38338）。其余模块要等到 `JS_FreeContext` 时才释放（quickjs.c:3078）。

**结果**：

- 每执行一次 `:js`，就**永久泄漏**一个 `JSModuleDef`，连同它的字节码和 namespace。查找成本也随之线性增长。
- 同一机制意味着：求值失败的模块会以 `EVALUATED + eval_exception` 的状态被缓存。用户修好 `rsvim.js` 后无法在会话内重新 source，类似 `:source %` 的工作流做不到。

**建议**：

- `:js` 改为 **script 求值**：用 `ctx.eval` 加 `EvalOptions { global: true, promise: true, .. }`，`promise: true` 允许顶层 await（rquickjs-core 0.14 `context/ctx.rs:37`；LLRT 入口也这样用，见 `vm.rs:158-179`）。需要模块时用 `import()`。
- 模块重载是 ESM 语义本身的限制，应在 RFC 中写明。如果确实需要 `:source`，可以设计"重建 Context"或"带版本后缀的模块名"，并接受其内存代价。

#### 4.8 顶层 await 挂起时静默

`js/host.rs:257-281` 把 `rquickjs::Error::WouldBlock` 视为 `Ok(())`，并且**不保留**这个求值 promise。然而 `rfc/2-EventLoop.md` 的伪代码第 19-20 行和正文第 6 条都写着："a top-level `await` that never settles … the module is named once on the way out"。`evloop_tests.rs:412` 附近的注释也承认，这个诊断在迁移到 QuickJS 后丢失了。

对照（investigation §7.1 实验）：

| 运行时 | TLA 永不 settle 时的行为 |
|---|---|
| Node | 退出码 13，提示 "unsettled top-level await"（`src/env.cc:1577-1610`） |
| Deno | stalled TLA 检测，报错（`jsruntime.rs:2727-2828`） |
| Dune、LLRT | 静默 |

**建议**：

- 用 `Persistent<Promise>` 保存 config 入口模块的求值 promise。
- promise settle 时，发出内部的 "config loaded" 事件。将来可以用于实现 `VimEnter` 一类的 autocmd，让插件的初始化顺序有明确定义。
- 超过 N 秒仍未 settle，或在退出时仍未 settle，至少报出模块名："rsvim.js is still awaiting at top level"。QuickJS 没有 V8 的 `GetStalledTopLevelAwaitMessages`，但"哪个入口还在等"已经足够有用。

#### 4.9 dirty 粒度过粗，且没有帧率控制

- `JsHost::enter` **无条件**设置 `dirty = true`（`js/host.rs:246`）。
- 主循环每一轮都会执行 `render_if_dirty`（`evloop.rs:1043`）。

所以一个 `setInterval(f, 1)` 即使 `f` 不碰编辑器状态，也会每毫秒执行一次完整的 `tree.draw` 加 canvas shade diff。大量 op 完成时同理。

**建议**（二选一，或者组合使用）：

1. 由会修改编辑器状态的 binding 主动标脏（例如 `kernel.mark_dirty()`，用于 `cmd.echo`、buf/opt 的写操作等），`enter` 不再兜底。
2. **帧节奏控制**：输入触发的重绘立即进行，保证按键延迟最低；只由 JS 或 timer 触发的 dirty，合并到每 8–16ms 最多重绘一次，可以在 `select!` 中加一个 `next_frame_at` 分支实现。浏览器的 rAF 和 Neovim 的 `ui_flush` 批量重绘都是这个思路。

#### 4.10 资源生命周期：没有 GC 回收，op 无法取消

**资源回收**：

- `ResourceTable`（`js/resource.rs`）是一个全局 `Arc<Mutex<FoldMap>>`，条目只能通过显式 close 移除。
- JS 侧的 `File`、流式 `TextDecoder`、`ReadDir`、子进程句柄对象被 GC 后，**底层 fd 和 decoder 一直泄漏到进程退出**。

**取消**：

- `OpTable` 没有 abort 能力。
- `TaskTracker::close()` 不会取消任务。
- 没有 `AbortSignal`。

对照：

- **Deno**：`Rc<dyn Resource>` 表。`op_close` 调用 `Resource::close()`，后者可以通过 `CancelHandle` 让包了 `.or_cancel()` 的 future 立即以 `Err(Canceled)` 结束（`async_cancel.rs:26-48, 182-229`）。
- **Dune**：`wrap_gc_dropped` 在 JS 对象被 GC 时释放 Rust 句柄（`bindings.rs:216-245`）。
- **Node**：`RunCleanup` 统一 cancel 所有 `ReqWrap`，再 close 所有 `HandleWrap`（`env.cc:1463-1487`）。

**建议**：

- 句柄对象改用 rquickjs `Class`（实现 `Trace` 和 `Drop`）来持有 rid，或者至少用 `FinalizationRegistry` 兜底 close。
- 在 `PendingOp` 上挂一个子 `CancellationToken`：退出时取消 `Detached` op，将来再暴露为 `AbortSignal`。
- 资源类型改为 tokio 异步类型，让取消可以通过 `select!` 实现（同时解决 4.5 中的持锁问题）。

#### 4.11 `import.meta` 未填充

`test_import_meta::property1` 失败（期望 5 条消息，实际 1 条）。测试期望有 `url`、`filename`、`dirname`、`main`，但 loader 从未设置。

rquickjs 0.14 提供了 `Module::meta()`（`rquickjs-core-0.14.0/src/value/module.rs:491`，即 `JS_GetImportMeta`），并且对 `Declared` 模块也可以调用。在 `FsModuleLoader::load` 中 `Module::declare` 之后设置这几个字段即可。`main` 只对 config 入口模块为 `true`。

#### 4.12 错误路径绕过 `close_trackers` 和 `shutdown`

- `evloop.rs:979-991`：`self.tick(..).await?` 遇到渲染 IO 错误时直接返回，**跳过了 `close_trackers`**。这与紧随其后的注释"Every way out funnels through here"相矛盾。
- `rsvim_cli/src/bin/rsvim.rs:94`：`event_loop.run().await?` 出错时跳过 `shutdown()`，终端停留在 raw mode 和备用屏幕。目前只有 panic hook 会恢复终端。

**建议**：

- `run_inner` 改成"先拿到结果，再无条件执行 `close_trackers`，最后返回结果"。
- `main` 中用 guard 保证 `shutdown` 一定执行。

另外，`std::process::exit(exit_code)`（`rsvim.rs:99`）在 `block_on` 内部调用，所以生产路径上 `JsHost::drop` 从不执行，精心设计的释放顺序只在测试里被覆盖。将来如果依赖 `Drop` 做落盘（swap 文件、日志 flush），需要注意这一点。

### P2：改进项

#### 4.13 引擎参数：内存、栈、GC

`Runtime::new()`（`js/host.rs:134`）全部使用默认值：

- 不限内存。
- 栈大小为 1MB（`quickjs.h:442`）。
- GC 阈值为 256KB（quickjs.c:2336）。

对照：LLRT 设置了 512KiB 栈和 20MB GC 阈值（`llrt_core/src/vm.rs:60-69, 115-116`），以降低 GC 频率。

**建议**：

- 调高 GC 阈值（例如 8–32MB），减少在按键处理中途触发的 cycle GC。
- **利用编辑器天然的空闲时间**：`select!` 空闲超过约 1s 时执行一次 `runtime.run_gc()`。服务端 runtime 没有这种空闲窗口，编辑器有。
- 设置一个宽松且可配置的 `set_memory_limit`（例如 1–2GB）。这样泄漏的插件会得到一个异常，而不是让编辑器被 OOM kill 并丢失 buffer。

#### 4.14 内置模块字节码预编译

目前约 20KB 的 `js/*.min.js` 每次启动都要重新 parse。可以对照的做法：

- LLRT 在 `build.rs` 中执行 `Module::declare` 加 `module.write`，再用 zstd 字典压缩并放进 phf 表；运行时调用 `Module::load`（`llrt_core/build.rs:115-197`、`embedded/loader.rs:30-56`）。
- txiki 使用 `tjsc`。

rsvim 可以在 `build.rs` 中做同样的事。字节码在构建时生成并嵌入二进制，属于可信来源（`js-engine.md` §11 第 8 条）。收益可能只有毫秒级，**先测量再决定**。注意字节码与 quickjs-ng 版本绑定，升级 rquickjs 时需要重新生成。

#### 4.15 未处理 rejection 的细节

- `flush_rejections`（`js/host.rs:379-396`）在 deadline 清除之后、drain 之外执行。把 reason 转成 `JsError` 时会读取 `name`、`message`、`stack`，用户定义的 getter 可以在不受 watchdog 保护的情况下运行，它入队的 job 要等到下一次 `enter` 才会执行。
  - 建议：把 flush 移到 deadline 生效期内，并在 flush 后再 drain 一次。
- 没有给插件的钩子：Node 有 `process.on('unhandledRejection')`，Dune 也支持；Deno 有 `globalThis` 上的 `unhandledrejection` 事件。将来可以考虑提供 `globalThis.addEventListener("unhandledrejection")`。

#### 4.16 计时器语义细节

- **interval 语义**：rsvim 是固定频率并跳过错过的 tick（`js/timers.rs:123-137`）；Node 从回调开始时刻重新计时。两者都合理，但应写进 API 文档。
- **`setTimeout` 包装器泄漏**：`src/00__web.ts:486-489` 中 `callback(...args); activeTimers.delete(id);`。回调抛异常时 `delete` 被跳过，`activeTimers` 的条目会泄漏。应改用 `try/finally`。
- **两套 id**：JS 侧的 `nextTimerId` 和 Rust 侧的 `TimerId` 是两套 id，加上 `activeTimers` 映射，属于冗余。Rust 侧的 id 已经是从 1 开始的单调 i32，可以直接返回给 JS。
- **settle 阶段的优先级**：`pop_ready` 固定按 master、jsrt、timer 的顺序取事件，一串 op 完成会推迟已到期的 timer。Deno、Node 都是 timer phase 在前。这不是正确性问题，但值得在 RFC 中写明。
- **unref**：目前不支持。将来做 headless 模式时会需要（见 4.19）。

#### 4.17 `TextEncoder.encodeInto` 的实现错误（测试 `encode2`、`encode3` 失败）

- `src/00__web.ts:69-72` 传的是 `dest.buffer`，**忽略了 `byteOffset` 和 `byteLength`**。对 `subarray` 或带偏移的视图，会从底层 buffer 的 0 偏移处开始写，覆盖其他视图的数据。
- `js/binding/global.rs:56` 把 `read` 设成了字节数。规范要求的是已消费的 UTF-16 code unit 数。
- `copy_into_array_buffer` 按字节截断（`converter.rs:56-69`），可能把多字节 UTF-8 序列切成两半。

**建议**：传入 TypedArray，按字符边界计算能放下的最长前缀，`read` 返回该前缀的 UTF-16 长度。

#### 4.18 测试基建

- `tests/js.rs:129-150` 中的 `run_until` 有自己的 `select!` 循环，复制了生产循环的分发逻辑，两者可能悄悄分叉。4.1 就是同类问题：测试走的路径与生产不同。
  - 建议：抽出一个 `JsHost` 级别的 `poll_next(&mut self, rx, cx) -> Ready`，生产循环和测试共用。
- `evloop_tests::timer_callback_marks_the_editor_dirty` 单独运行能通过，全量并行时会超时失败。这是 flaky 测试，疑似并行用例共享 `PATH_CONFIG` 或配置目录等全局状态。

#### 4.19 前瞻：headless 模式与存活策略

`evloop/writer.rs:88` 已经有 `HeadlessWriter`，但还没有"JS 工作做完就退出"的策略。将来一旦做 `rsvim --headless script.js` 或插件测试 runner，就会面对 investigation 文档 §8、§9 讨论的全部问题：

- 只统计 ref'd 的 op 和 timer。
- 引入 unref。
- 定义 TLA 未 settle 时的策略。
- 区分"等待"与"可运行"。

`OpTable` 已经有 `pending_len()` 和 `timers_len()`，只需要再加一个 `refed` 标志。建议现在就在 RFC 中把这个策略定下来，避免以后又靠打补丁补上。

更远的方向：重型插件（如 LSP 客户端的 JSON 处理）会占用主线程。QuickJS 是"一个线程一个 runtime"，txiki、Node 的 Worker 都是"独立 runtime 加消息传递"。届时可以把 `JsHost` 放到独立线程，通过 serde 做 structured clone。当前不必实现，但 `JsKernel` 中的 `Arc<Mutex<…>>` 共享状态的设计，要为这一步留出空间。

---

## 5. 文档与实现的偏差

| 位置 | 文档说法 | 实际 |
|---|---|---|
| `rfc/2-EventLoop.md`（最后更新于 2025-09-04） | V8 isolate；通过 `Notify` 唤醒；用 1ms→16ms 的退避计时器兜底 V8 后台任务；"each completion run to quiescence, then a tick of the js runtime"；退出时 "report rejections, stalled TLA" | QuickJS；没有 `Notify`，没有退避计时器；没有单独的 JS tick；退出时不报告 TLA（4.8） |
| `rfc/3-JavaScriptRuntime.md` | "Rsvim choose the same way (V8)" | 已切换到 QuickJS-ng。需要补充选型理由（体积、启动速度、无 JIT 的性能边界，见 `js-engine.md` §2、§11） |
| `rfc/3-JavaScriptRuntime/2-ImportModules.md` | 照搬 Dune 的 `ModuleMap` / `fetch_module_tree` | 模块图由 QuickJS 自管，宿主只提供 resolver 和 loader |
| `Investigation/js-runtime-modules.md` §11 | "No rsvim checkout was available"，并按 V8 的 `ModuleMap` 给出建议 | 已过时，可以按 4.1、4.7、4.11 更新 |
| `js/runtime.rs:52-53` | 引用 `evloop-plan-v2.md`、`evloop-quickjs-plan-claude.md` §8.6 | 这两份文档不在 `rfc` 仓库，也不在 `rsvim` 仓库中。如果是决策依据，应当入库 |
| `js/fs_loader.rs:27` | "using v8's built in objects" | 与 V8 无关 |
| `js/core_modules.rs:17-19` | 链接到 `crate::js::loader::core_loader::CoreModuleLoader` | 该路径不存在，实际是 `js::loader::CoreLoader` |
| `js/binding.rs:66`、`00__web.ts` 中 `queueMicrotask` 的注释 | 提到 v8 backend 和 `create_new_context` | 已不存在 |
| `js/command/mod.rs:39` | `CommandFuture` | 已经不是 future，建议改名为 `ExCommand` |
| `evloop.rs:103-104, 306` | `EventLoop` 作为 `Rsvim` 的别名保留 | 可以在一次性重命名后删除 |

---

## 6. 测试现状

在 `cargo test -p rsvim_core --lib` 中，**708 passed / 14 failed**：

| 分组 | 失败数 | 归因 |
|---|---|---|
| `js::module_map_tests::test_static_import::*` | 9 | 4.1 相对解析；`*_failed*` 另外受错误消息格式变化影响（期望 `"Uncaught Error: Module "`，实际是 QuickJS 的 `ReferenceError: Error resolving module`） |
| `js::module_map_tests::test_import_meta::*` | 2 | 4.11（`resolve_failed1` 也依赖 4.1） |
| `js::text_encoder_tests::test_encode2/3` | 2 | 4.17 |
| `evloop_tests::timer_callback_marks_the_editor_dirty` | 1 | 并行运行时 flaky，单独运行通过（4.18） |

这说明 V8 到 QuickJS 的迁移在模块和 Web API 层面还没有收尾。建议把"全量测试为绿"作为 `arch2` 合并到 `main` 的门槛。

---

## 7. 建议路线图

**第一阶段：止血（合并 `arch2` 之前）**

1. 4.1：resolver 改用 `resolve_file`，测试改为走引擎路径。
2. 4.2：用 serde_json 校验后再生成 JSON 模块。
3. 4.11、4.17：补 `import.meta`，修 `encodeInto`。全量测试转绿。
4. 4.3（最小修复）：遇到 JS 工作时结束输入批。
5. 4.12：错误路径上也执行 `close_trackers` 和 `shutdown`。
6. §5：同步 RFC-2、RFC-3、2-ImportModules，并把 plan 文档入库。

**第二阶段：退出与中断契约**

7. 4.4：quit drain 阶段，包括 blocked op 计数、超时和提示。区分 `process.exit` 与 `:q`。
8. 4.5：改用 `tokio::process`，加 `kill_on_drop` 和 `kill()`，`wait` 不再持锁。
9. 4.6：引入独立输入线程，加 Ctrl-C breakcheck，并总是安装 interrupt handler。
10. 4.8：保存入口模块的 promise，提供 TLA 诊断和 "config loaded" 事件。

**第三阶段：性能与长期健壮性**

11. 4.9：细粒度 dirty 加帧节奏控制。
12. 4.7：`:js` 改为 script 求值。
13. 4.10：句柄 GC 回收、per-op 取消、`AbortSignal`。
14. 4.13、4.14：GC、内存参数和空闲 GC；字节码预编译（先测量）。
15. 4.3（推荐修复）：FSM 改为返回 `Effect`，为 JS 按键映射打基础。
16. 4.19：在 RFC 中定下 headless 存活策略（ref/unref、TLA）。

---

## 附录：主要参考源码索引

**rsvim**（`rsvim_core/src/`）：

- `js/host.rs`：`enter` 239、`execute_module` 260、`complete` 292、`run_timer` 342、`drain_jobs` 364、`flush_rejections` 379、`Drop` 414
- `js/ops.rs`：`OpKind` 47、`spawn_op` 198、`blocking` 252
- `js/timers.rs`：`insert` 60、`pop_due` 109、`poll_due` 151
- `evloop.rs`：`tick_event_loop` 221、`pop_ready` 243、`close_trackers` 897、`tick` 1025、`select!` 1053、`drain_input` 1135
- `js/fs_loader.rs`：`wrap_json` 28、`resolve_impl` 101
- `state/fsm/cmdline_ex.rs`：110
- `js/binding/process_async.rs`：27

**Deno**（`~/github/denoland/deno/libs/core/`）：

- `runtime/jsruntime.rs`：phase 文档 2425-2435、liveness 3127-3137、`dispatch_event_loop_tick` 3551-3619、stalled TLA 2727-2828、cleanup 250-335
- `01_core.js`：`__eventLoopTick` 459-466
- `runtime/setup.rs`：284（`MicrotasksPolicy::Explicit`）
- `runtime/op_driver/futures_unordered_driver.rs`：128-210
- `async_cancel.rs`

**Node**（`~/github/nodejs/node/`）：

- `src/api/embed_helpers.cc`：23-78
- `deps/uv/src/unix/core.c`：427-492
- `src/api/callback.cc`：138-213
- `lib/internal/process/task_queues.js`：72-109
- `lib/internal/process/promises.js`：158-416
- `lib/internal/timers.js`：189-646
- `src/node_watchdog.cc`：51-109、715-774
- `src/node_contextify.cc`：1287-1315
- `src/env.cc`：1463-1487、1577-1610

**LLRT**（`~/github/awslabs/llrt/`）：

- `llrt_core/src/vm.rs`：60-69、105-135、158-197
- `modules/llrt_timers/src/lib.rs`：270-398
- `llrt_core/build.rs`：115-197
- rquickjs-core：`runtime/async.rs` 301-348、`value/promise.rs` 48-79

**Dune**（`~/github/aalykiot/dune/`）：

- `src/runtime.rs`：148、442-504、522-616
- `src/hooks.rs`：110-243
- `src/bindings.rs`：216-245
- crabuv `src/event_loop.rs`：169-284、657-661

**QuickJS-ng**（rquickjs-sys 0.14.0 vendored）：

- `quickjs.c`：556（`JS_INTERRUPT_COUNTER_INIT`）、2336（GC 阈值）、2966-2980（`js_free_modules`）、3078
- `quickjs.h`：442（`JS_DEFAULT_STACK_SIZE`）
