# JavaScript 运行时架构：rsvim vs. dune vs. LLRT vs. txiki.js

对四个可嵌入 JS 运行时的源码级比较，围绕一个真正决定运行时形态的问题来组织：**谁拥有时间？** 事件循环、模块加载器和异步桥是同一个问题在三个角度下的呈现，因此文档从这里开始，再向外扩展到模块、API 面、启动与分发。

分析的快照（本地工作树，只读）：

| | 仓库 | HEAD | 语言 | 引擎 | 循环宿主 |
|---|---|---|---|---|---|
| **rsvim** | `rsvim-dev/rsvim` | `ef9a345` (0.1.3-alpha.2) | Rust | V8 15.0.245.2（`rusty_v8` crate 150.4.0） | **tokio**（编辑器的 `select!`） |
| **dune** | `aalykiot/dune` | `5958b64` (v0.11.4) | Rust | V8（`rusty_v8` crate 150.2.0） | **crabuv** —— 一个架在 `mio` + `rayon` 上的 libuv 形状的循环 |
| **LLRT** | `awslabs/llrt` | `d9fd873` (v0.9.0-beta-147) | Rust | 经由 `rquickjs` 0.14 使用 QuickJS-NG | **rquickjs** 的 `AsyncRuntime`，跑在 tokio 上 |
| **txiki.js** | `saghul/txiki.js` | `8a9b6044` (v26.6.0-136) | C | QuickJS-NG | **libuv** |

其中两个是 V8 + Deno 风格的 Rust 骨架；两个是 QuickJS-NG。两个驱动自己的循环；两个借用别人的。这个 2×2 就是下文所有内容的地图。

---

## 0. 每者一句话定位

- **rsvim** —— 一个*嵌入编辑器的 V8 + tokio 脚本层*。不存在运行时循环；编辑器的事件循环就是运行时的循环，JS 由它推进。公开接口面是 `Rsvim.*`，而不是 `globalThis.fetch`。
- **dune** —— 一个*独立的、Node 形状的 V8 + libuv 风格运行时*。它拥有循环，但是手写的循环：`crabuv` 在 `mio` 与 `rayon` 之上复刻了 libuv 的 API 与语义。
- **LLRT** —— 一个*Node 形状的 QuickJS + tokio 运行时*，为 Lambda 冷启动优化。它不拥有*主*循环：`rquickjs` 调度 JS future，而 LLRT 提供 API，外加两个手写的泵（定时器、`require`）和 Lambda 调用循环。
- **txiki.js** —— 一个*Web 形状的 QuickJS-NG + libuv 运行时*。它拥有真正的 libuv 循环，手写作业队列桥，并把 worker 当作独立的运行时 + 循环对。

各自最具诊断价值的一点：

| | 谁拥有事件循环？ |
|---|---|
| rsvim | **宿主编辑器拥有**——在输入、通道与取消之上的 `tokio::select!` |
| dune | **dune 拥有**——`JsRuntime::run_event_loop` 驱动 `crabuv` 的 `RunMode::Once` |
| LLRT | **rquickjs 拥有**——`vm.idle().await`；LLRT *没有*自己的循环代码 |
| txiki.js | **txiki 拥有**——`uv_run` 外加一个 `uv_prepare`/`uv_check`/`uv_idle` 作业队列桥 |

---

## 1. 架构速览

| 维度 | rsvim | dune | LLRT | txiki.js |
|---|---|---|---|---|
| 引擎绑定 | `v8` crate，裸 `v8::` API | `v8` crate，裸 `v8::` API | `rquickjs`（`ModuleDef`，无 fork） | QuickJS-NG C API，直接调用 |
| 宿主 / 平台 | tokio | `crabuv`（mio + rayon） | tokio（经由 `AsyncRuntime`） | libuv |
| 循环所有者 | 编辑器循环 | 运行时 | `rquickjs` | 运行时 |
| JS 线程 | 1（与编辑器任务共享） | 1（主线程） | 1（串行化于互斥锁之后） | 1 + N 个 worker（各自运行时 + 各自循环） |
| 微任务 | **显式**，每个 tick 顶部一个检查点 | **显式**，多个检查点 | 由 `rquickjs` 的 `idle()` 管理 | 在 `uv_run` 周围 3 个点排空 |
| 模块系统 | **仅 ESM**（无 CJS，无 `require`） | **仅 ESM**（无 CJS，无 `require`） | **ESM + CJS**（`require`，互操作） | **仅 ESM**（无 CJS，无 `require`） |
| npm / 裸说明符 | 通过 `oxc_resolver` 解析 `node_modules`，仅 `default` 条件 | **无**——仅导入映射（import map） | 在*构建时*解析（esbuild + shims），运行时解析器作为回退 | **无**——仅导入映射（import map） |
| TypeScript | `.ts` 经 swc（`rsvim_cli` 默认 feature） | `.ts` / `.tsx` / `.jsx` 经 swc | 无（自行预编译） | 无 |
| 启动加速器 | 由 CLI crate 构建的 **V8 启动快照** | 无（每次运行重新编译 JS 源码） | `phf` 映射中的预编译字节码 | `qjsc` 字节码；`core`+`polyfills` 急切加载 |
| 可作为库嵌入 | 是（`rsvim_core`） | 否（仅二进制；`main.rs` 拥有运行时） | `llrt_core` 是库，`llrt` 仅二进制 | 是（`add_library(tjs STATIC)`），但没有 `install()` 规则 |
| 运行时核心体积 | `rsvim_core/src/js/` 中约 7.9k LOC 非测试代码（含测试约 18k）；全仓库约 31k 非测试代码 | 约 9.1k LOC 非测试 Rust + 约 6.4k LOC JS | 约 3.8k 核心胶水（`llrt_core/src`）+ 约 2.6k `llrt_modules`；`modules/` ≈ 57.9k，`libs/` ≈ 7.2k，Rust 合计约 73k | 约 35k LOC C（不含生成的 bundle）+ 约 22k LOC JS |
| 发布二进制 | 约 64 MB（`ef9a345` 时为 64,227,600 B） | 此处未测量 | 刻意保持小 | 用文档给出的体积标志构建时很小（`BUILD_WITH_GC_SECTIONS` 默认关闭） |
| 权限模型 | 无 | 无 | 仅网络允许/拒绝列表 | 无 |
| REPL / 调试器 | 无 REPL；inspector 代码被注释掉 | REPL + Chrome DevTools inspector | REPL（非 lambda 构建） | REPL；无 inspector |

有一条对全文都重要的提示：**rsvim 与 dune 近乎同胞兄弟。** 两者携带相同的 Deno 风格骨架——`ModuleMap` / `ModuleGraph` / `EsModule` / `same_origin` / `JsFuture` / `pending_futures` / `ImportMap`——以及相同的 `tick_event_loop` 形状。比较 `rsvim_core/src/js.rs:610-614` 与 `dune/src/runtime.rs:427-432`：两者都执行 *nextTick → imports → futures*，并且都调用 `v8::Isolate::set_microtasks_policy(Explicit)`。有意思的差异在于各自底下接了什么：tokio（rsvim）对上手写的 libuv 克隆（dune）。

---

## 2. 事件循环：两种基底，四个答案

关于本节更深入、独立的论述——包括审计线索——见 `JS_RUNTIME_EVENT_LOOPS.md`。

### 2.1 tokio 与 libuv 不是同一想法的两种实现

- **libuv** 是*一张带回调的句柄表*。每个待处理的东西都是引用计数的句柄（`uv_timer_t`、`uv_tcp_t`、`uv_work_t`、`uv_idle_t`）；`uv_run` 在 poll 阶段阻塞，直到某个 fd、定时器或唤醒触发；当且仅当仍有被引用的句柄（或活跃请求）时，循环才是"活的"。时间是循环的一等设施。宿主调用 `uv_run`，只有在回调跑过或者循环排空之后才重获控制。
- **tokio** 是*带 reactor 的 future 执行器*。时间与 I/O 暴露为 future；进展由轮询任务树驱动。没有可检查的句柄表，没有 `uv_loop_alive`，也没有"第 N 次迭代的回调"这种概念。宿主 await，由执行器决定接下来运行什么。

下文一切都由此推出。与此同时，JS 引擎提供了第三种模型：QuickJS 有一个**作业队列**，只有一个入口（`JS_ExecutePendingJob`、`JS_IsJobPending`）；V8 有一个**微任务队列**，带显式策略下的检查点。两者对 tokio 与 libuv 都不可见。因此每个运行时都必须回答一个其基底无法回答的问题：*"还有 JavaScript 工作剩下吗，它该在什么时候运行？"*

### 2.2 rsvim：宿主拥有循环，JS 是乘客

整个程序就是一个 `tokio::select!`（`rsvim_core/src/evloop.rs:1347-1365`），等待终端输入、master 消息、JS 消息与取消信号。`JsRuntime` 是编辑器 `EventLoop` 结构体的一个**字段**（`evloop.rs:142`），因此 V8 是被 select 分支内联驱动的：

```rust
// rsvim_core/src/evloop.rs:1321 — the only place JS advances on a message
self.jsrt_tx.send(message).unwrap();
self.js_runtime.tick_event_loop();
```

没有独立的 JS 线程，也没有 `uv_run` 的对应物。一个 tick 是 `checkpoint → fast_forward_imports → run_pending_futures`（`js.rs:610-614`）。

异步 JS 是一种 actor 协议。一个原生 op 通过无界通道投递 `MasterMessage`；select 分支在 `detached_tracker` 上 spawn 一个 tokio 任务（`evloop.rs:816-835`）；该任务执行真正的 I/O，并在一条**回环**通道上送回 `JsMessage`，这唤醒 select，被推入 JS 队列并触发一次 tick（`evloop.rs:1321-1327`）。结果以 `postcard` 字节跨越边界（`evloop.rs:828`）。

颇具揭示性的后果是：当 JS 有挂起工作而 tokio reactor 无事可做时，rsvim 必须给自己发消息来争取下一轮。那就是 `TickAgainReq`（`js.rs:627-637`），处理方式是 spawn 一个任务，其整个函数体就是把 `TickAgainResp` 回显回来（`evloop.rs:810-814`）。这是对执行器的一次 `await`，由一次通道往返搭建而成。

定时器不是一种定时器设施：每一个都 spawn 一个 `tokio::time::sleep_until` 任务，到期时投递消息（`evloop.rs:774-789`），而 `setInterval` 通过重新插入回调来重新设定（`js.rs:687-696`）。

有两个运行时层面的事实值得知道：配置模块在 TUI 存在**之前求值一次**（`evloop.rs:541-574`；回显消息被缓冲，初始化之后再冲刷，`evloop.rs:671-689`），并且没有 REPL 或 `eval`。另外，`*Sync` API 在编辑器任务上内联执行（`binding/global_rsvim/fs/open.rs:176-197`），因此一次慢速同步 fs 调用会冻结输入与渲染。

### 2.3 dune：一个 libuv 克隆，由 JS 驱动

`crabuv` 是 aalykiot 自己的 crate（v0.2.0；依赖 `mio`、`rayon`、`notify`、`signal-hook`、`slotmap`）。它**不是 libuv**；它是对 libuv API 面与迭代顺序的忠实重实现：

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

运行时拥有外层循环（`dune/src/runtime.rs:442-465`），其条件正是 JS 工作变得可见的地方：

```rust
while self.event_loop.has_pending_events()
   || self.has_promise_rejections()
   || self.isolate.has_pending_background_tasks()
   || self.has_pending_imports()
   || self.has_next_tick_callbacks() { self.tick_event_loop(); }
```

`has_pending_events()` 是 `uv_loop_alive` 的克隆：资源 + 排队请求 + 在途线程池任务（`event_loop.rs:657-661`）。那四个 JS 谓词是 dune 的发明，因为循环看不见 V8 的微任务队列。

异步工作使用 `uv_queue_work` 的等价物——`state.handle.spawn(work, Some(cb))` 把作业送到一个 **rayon** 池；worker 发送 `Event::ThreadPool` 并调用 `waker.wake()` 打断 poll（`event_loop.rs:454-466`）。完成时*并不*调用 JS：它把一个 `JsFuture` 推入 `pending_futures`（`src/file.rs:145-160`），后者在循环迭代返回之后被排空（`runtime.rs:479-504`）。

而当某个定时器让 JS 回调就绪、循环却可能阻塞时，dune 会故意打断 poll，并用一个标志保护，使其只做一次：

```rust
// dune/src/timers.rs — inside create_timeout's loop callback
state.pending_futures.push(Box::new(future));
if !state.wake_event_queued {
    state.interrupt_handle.interrupt();   // mio::Waker
    state.wake_event_queued = true;
}
```

`setImmediate` 是一个 `CheckHandle`（uv check 阶段），不是定时器。REPL 展示了同样的风格：它用一个空的 `u32::MAX` 间隔定时器让循环保持存活，读取线程在输入一行时打断循环（`src/repl.rs:281-294`）。

### 2.4 LLRT：rquickjs *就是* 循环

`#[tokio::main]` + `vm.idle().await`（`llrt/src/main.rs:115,131`）。整个运行时核心是一个 219 行的文件，而 `idle()` 只是一次委托：

```rust
// llrt_core/src/vm.rs:29-32, 194-197
pub struct Vm { pub runtime: AsyncRuntime, pub ctx: AsyncContext }
pub async fn idle(self) -> ... { self.runtime.idle().await; Ok(()) }
```

`rquickjs` 的 `AsyncRuntime` 在 QuickJS 作业队列旁边维护一个 Rust future 的侵入式调度器；`idle()` 把 `JS_ExecutePendingJob` 与对该调度器的轮询交错执行，全部在同一把 `async_lock::Mutex` 之下。JS future **不是 tokio 任务**——它们住在那个调度器里，只会在 `idle()` / `drive()` 中被轮询（`rquickjs-core-0.14.0/src/runtime/async.rs:301-355`），这就是为什么一个多线程的 tokio 进程仍然会串行化 JS。原生异步函数无需任何接线：`Func::from(Async(read_file))` 变成 `Promise::wrap_future` → `JS_NewPromiseCapability` + `ctx.spawn`。

在 `rquickjs` 没有概念的地方，LLRT 自己写泵：

- **定时器**（`modules/llrt_timers/src/lib.rs:35-41`、`305-398`）：一个带 `deadline` 和 `Notify` 的 `RuntimeTimerState`，由 spawn 出的任务以 `select! { notify, sleep }` 驱动。它把睡眠钳制到 `MIN_SLEEP = 4ms`，在第一个回调之前以及每个回调之后各排空一次作业队列，并通过把 `setImmediate` 的截止时间设为 600 秒之前来让它排在最前。
- **`require`**（`llrt_modules/src/module/require.rs:111-121`）：一个同步 CJS require，实现方式是空转作业队列——并就地触发到期的定时器——直到 `Module::import` 敲定。这是该运行时最奇怪的角落。

因为一切都只有一个队列，两处语义偏离了 Node：`process.nextTick` 就是 `queueMicrotask`（两者都是 `cb.defer`），并且不存在 ref/unref 记账——挂起的定时器让进程存活，因为驱动 future 尚未完成，而 `dgram` 的 `unref()`/`ref()` 是有文档记录的空操作（`modules/llrt_dgram/src/socket.rs:527-536`）。

### 2.5 txiki.js：真正的 libuv，作业队列嫁接其上

`TJS_Run` 启动 `uv_prepare`/`uv_check`，把两者都 **unref**，然后循环：

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

诀窍是 `uv__maybe_idle`（`src/vm.c:923-929`）：如果 QuickJS 有挂起的作业或挂起的 rejection，就启动一个**被引用的** `uv_idle` 句柄；否则停止它。被引用的句柄让 `uv_run` 保持存活，*并且*把 libuv 的 poll 超时强制为 0，因此 JS 工作永远不会被困在阻塞的 poll 之后。`uv_check` 运行作业，然后重新求值。这是四者中对活跃性问题最干净的答案，因为它使用 libuv 自己的引用计数，而不是发明谓词。

微任务在**三个**点排空：`uv__check_cb`、每次经其路由的回调之后的 `tjs_call_handler`（`src/utils.c:205`，41 个 C 调用点），以及 `EventTarget.dispatchEvent`（`src/js/polyfills/event-target.js:657` → `core.drainMicrotasks()`），重入保护 `draining_microtasks` 位于绑定中（`src/vm.c:184-198`）。

原生异步使用 libuv 的线程池：`uv_queue_work`（`src/mod_fs.c:1365`）加上一个 after-work 回调，在 `:1339` 敲定 promise，创建于 `:1372`。每个定时器各是一个 `uv_timer_t`。从不重入 `uv_run` 的代价是 HTTP 导入会阻塞循环：一个 `while (!load_ctx->done) lws_service(...)` 循环，每次尝试有 10 秒硬期限，最多 20 次重定向（`src/lws-utils.c:798-843`），全部发生在一个 JS 求值内部。`tjs.exit(n)` 绕过 `beforeunload` 并立即调用 `exit()`（`src/mod_os.c:37-38`）；而*正常*的循环终止会触发一个可取消的 `beforeunload`，它可以重入循环——除非发生未捕获异常或未处理的 rejection（`src/vm.c:1103-1119`）。

### 2.6 活跃性问题，并排对比

| | "还有 JS 工作剩下吗？" | 什么让进程存活 | 唤醒机制 |
|---|---|---|---|
| rsvim | `tick_event_loop` 中的四个谓词（`js.rs:627-637`） | 编辑器永远运行；只能通过 `Rsvim.rt.exit` 退出 | 通道；JS 需要一轮时 `TickAgainReq` 往返 |
| dune | 四个谓词 + 循环自身的 `has_pending_events`（`runtime.rs:442-465`） | 被引用的 `crabuv` 资源 | 来自 `interrupt_handle` 的 `mio::Waker`；worker 完成时调用 `waker.wake()` |
| LLRT | `rquickjs` 调度器（LLRT 不可见） | `idle()` 中未完成的驱动 future | 定时器驱动中的 tokio `Notify` + `Sleep` |
| txiki.js | 由 `uv__maybe_idle` 查询的 `JS_IsJobPending` | libuv 引用计数；作业挂起期间 `uv_idle` 被引用 | libuv 本身 |

这个模式再清楚不过：**libuv 给你一套把"JS 挂起"表达为句柄状态的词汇，而 tokio 没有。** txiki 在一个函数里直接用它。dune 有同样的词汇，但只是近似于 libuv 的保证，所以它在循环*外面*加上 `has_next_tick_callbacks()` 之类，然后打断循环来响应它们。rsvim 无法打断自己的 select，于是给自己发消息。LLRT 通过借用 `rquickjs` 完全绕开这个问题，然后为此付出两个手写泵的代价。

### 2.7 原生异步 → promise，并排对比

| | 原生 op 运行于 | 结果跨越方式 | Promise 创建于 | 敲定于 |
|---|---|---|---|---|
| rsvim | tokio 工作线程 / 阻塞池 | 经 `mpsc` 的 `postcard` 字节 | 存于 `JsFuture` 的 `v8::PromiseResolver` | 下一个 tick，从 `pending_futures` |
| dune | rayon 线程池 | `Box<dyn Any + Send>`，在循环中 downcast | `JsFuture` 中的 `v8::PromiseResolver` | 循环迭代之后，从 `pending_futures` |
| LLRT | tokio I/O 被卸载；future 本身在 JS 线程上轮询 | 就地（同线程，借用 `Ctx`） | `Promise::wrap_future`（`JS_NewPromiseCapability`） | 在 `idle()` 期间 future 被轮询时 |
| txiki.js | libuv 线程池 / 循环线程 | 就地（循环线程） | `JS_NewPromiseCapability`（fs/dns）或 JS `withResolvers()` | after-work / C 回调 |

### 2.8 微任务检查点与顺序

| | 策略 | 检查点 | 顺序后果 |
|---|---|---|---|
| rsvim | `Explicit`（`js.rs:144-146`） | **一个**，在 tick 顶部（`js.rs:1097-1131`） | 在排空 future 过程中被解决的 promise，其 continuation 要到*下一个* tick 才排空 |
| dune | `Explicit`（`runtime.rs:148`） | `nextTick` 排空后（`runtime.rs:718`）、导入后（`:615`）、每个 future 后（`:500`）、REPL 派发（`:294`） | continuation 在解决它们的同一个 tick 内运行 |
| LLRT | 引擎管理 | 在 `poll_timers` 内部、每个回调前后；在 `idle()` 内部 | JS 回调总在队列已排空的状态下运行 |
| txiki.js | 引擎管理（作业队列） | `uv_check` 后、每个经辅助函数路由的回调后、每次监听器派发后 | 回调级粒度；由 `tests/test-async-task-sequence.js` 钉住 |

rsvim 的顺序值得标注，因为它在两个方向上都不同寻常：检查点在 tick 的*开始*，而 `run_next_tick_callbacks` 只做检查点——真正的 nextTick 队列代码被注释掉了（`js.rs:1097-1118`），这是它继承的 Deno 骨架的残余。

### 2.9 定时器

| | 机制 | 间隔处理 | 说明 |
|---|---|---|---|
| rsvim | 每个定时器一个 tokio `sleep_until` 任务，投递消息 | 每次触发后重新创建，`start_at` 重置 | 会漂移；相对于 I/O 的顺序取决于 tokio 调度器 |
| dune | 循环原生的定时器集合，在 poll 之前运行 | 循环内的 `TimerKind::Interval` | 真正的循环定时器；`setImmediate` 是 check 句柄 |
| LLRT | LLRT 自有的泵，基于 tokio `Sleep` + `Notify` | 在 `poll_timers` 内部重新设定 | 4 ms 下限；`setImmediate` 靠过去的截止时间 |
| txiki.js | 每个定时器一个 `uv_timer_t` | 由循环重新设定 | 真正的循环定时器；不受作业队列影响 |

---

## 3. ES 模块支持

### 3.1 特性矩阵

| | rsvim | dune | LLRT | txiki.js |
|---|---|---|---|---|
| 静态 `import`/`export` | ✅ (V8) | ✅ | ✅ | ✅ (QuickJS) |
| 动态 `import()` | ✅ 自定义宿主钩子 | ✅ 自定义宿主钩子 | ✅ | ✅ 引擎侧（txiki 未写任何相关代码） |
| `import.meta` | ✅ `filename`、`dirname`、`url`、`main`、`resolve()` | ✅ `url`、`main`、`resolve()` | ✅ | ✅ `url`、`main`、`dirname`、`basename`、`path` |
| 顶层 await | ⚠️ 会求值，宿主**不** await（编辑器循环继续运行） | ⚠️ 会求值，宿主**不** await（循环继续运行） | ✅（入口经 `import()` 运行） | ✅（入口 `await core.evalFile(...)`） |
| JSON 模块 | ⚠️ 通过模板字符串重写源码 | ⚠️ 同样的重写 | ✅ 经加载器 | ✅ 导入属性或 `.json` 后缀 |
| WASM 模块 | ⚠️ 加载器按 UTF-8 读取，因此真实二进制会失败 | ⚠️ 同样的缺陷 | ❌（引擎不支持） | ❌ 作为 ESM 类型；✅ WAMR 上的 `WebAssembly` API + `tjs:wasi` |
| 导入属性 | ❌ 被忽略 | ❌ 被忽略 | ⚠️ 仅 JSON，且 JSON *必需* | ✅ `type: "json" \| "text" \| "bytes"` |
| 导入映射（import map） | ❌ **仅桩** | ✅（WICG 子集） | ❌（改为加载器*钩子*） | ✅（WICG 规范） |
| 远程 `http(s)://` 导入 | ❌（加载器被注释掉） | ✅ 带 sha1 键的磁盘缓存 | ❌ | ✅ **是** |
| CommonJS / `require` | ❌ 明确不支持 | ❌ 不支持 | ✅ | ❌ |
| `node_modules` | ✅ 可解析，仅 ESM，`default` 条件 | ❌ | ✅ 始终启用的解析器，建议打包 | ❌ |
| TypeScript | ✅ `.ts` 经 swc | ✅ `.ts`/`.tsx`/`.jsx` 经 swc | ❌ | ❌ |

对上表的直观读法需要两处更正：

- rsvim 的导入映射支持是一个**模拟品**。`ImportMap` 在文档注释里就是这么说的，`parse_from_json()` 返回空映射，`lookup()` 总是返回 `None`，真正的实现被注释掉了（`js/module/import_map.rs:27-41`、`:43-92`）。从来没有任何东西填充那个选项（`js.rs:329-349`）。相比之下，dune 的导入映射是真的，并且处理尾斜杠包与无扩展名说明符（`src/modules.rs:488-538`）。
- 两个 V8 运行时的 JSON 支持是同一行代码，``format!("export default JSON.parse(`{source}`);")``（`rsvim .../fs_loader.rs:26-28`、`dune/src/loaders.rs:49-51`）。任何包含反引号或 `${` 的 JSON 都会被破坏，而两个 wasm 加载器在重新发出字节之前都用 `read_to_string` 读取模块，这会拒绝合法的二进制 `.wasm`。它们是继承来的缺陷，不是各自独立的错误。

### 3.2 解析算法：四种不同模型

**rsvim** 按说明符选择加载器，并把路径计算委托给 `oxc_resolver`：

```rust
// rsvim_core/src/js/loader/fs_loader.rs:107-135
ResolveOptions {
  extensions: vec![".js", ".ts", ".mjs", ".json", ".wasm"],
  modules: vec![config_home(), config_home()/node_modules, "node_modules"],
  ..   // note: extension_alias, but no condition_names
}
```

没有设置 `condition_names`，因此 `package.json#exports` 只按 `default` 条件匹配（`import`/`node` 键无法解析）。CJS 在文档注释里是明确的非目标（`fs_loader.rs:166-177`），`.jsx`/`.tsx` 被注释掉（`:211-216`、`:253-258`），`.ts` 是唯一被转译的扩展名。实际后果是：像 `lodash` 这样的包*能解析*，然后求值失败，因为它是 CJS。

**dune** 在 `resolve_import`/`load_import` 中按三类加载器分派——核心模块、URL、文件系统（`src/modules.rs:428-477`）。裸说明符只有在导入映射重写它们时才能解析；否则 `FsModuleLoader::resolve` 以 `Module not found` 退出（`src/loaders.rs:98-118`）。URL 导入用 `ureq` 抓取，用 SHA-1 哈希并缓存到 `~/.dune/cache`（debug 构建下是 `.cache`），`--reload` 会绕过缓存。导入映射的实现是一张有序前缀表，带 WICG 的尾斜杠规则（`src/modules.rs:488-538`）。

**LLRT** 保留了一个完整的运行时 `PackageResolver`——Node 的 `LOAD_AS_FILE` / `LOAD_AS_DIRECTORY` / `LOAD_NODE_MODULES` 的移植——放在默认解析器元组中（`llrt_core/src/vm.rs:106-112`），因此未打包的 `node_modules` 也能工作。仍然强烈建议打包：`build.mjs` 对 LLRT 自己的 JS 入口点和 AWS SDK 运行 **esbuild**（`platform: "browser"`、`format: "esm"`），内联依赖，并为伸进 Node 内部的包换上 `shims/`；用户代码由用户自己的 esbuild/rollup/webpack 配方打包。QuickJS 的解析器被要求剥掉 `node:` 方案（`llrt_modules/src/module/resolver.rs:43`）。

**txiki.js** 是 Web 纯粹主义者：单个 `JS_SetModuleLoaderFunc2`（`src/vm.c:550`）。`tjs:internal/core` 与 `tjs:internal/worker` 是 C 构建的模块（`tjs:internal/*` 的其余部分是字节码），`tjs:*` 反序列化预编译字节码（`src/builtins.c:78-103`），`http(s)://` 通过 libwebsockets **在运行时下载并编译**，其余一切都是文件，可选地按导入属性或 `.json` 后缀解释为 JS/JSON/text/bytes。裸说明符命中导入映射；未匹配的说明符原样通过（`src/modules.c:356-378`），这意味着按设计没有 `node_modules`。

### 3.3 动态导入的 promise

关于这一机制的、回调层面的四方追踪（dune、rsvim、Ende、Node.js）——谁分配 promise、什么驱动进展、在哪里敲定——见 `JS_RUNTIME_DYNAMIC_IMPORT.md`。

- **rsvim**：`host_import_module_dynamically_cb`（`js/hook.rs:181`）创建一个 `v8::PromiseResolver`，在 `ModuleMap::pending` 中注册一个 `ModuleGraph`，之后在 `fast_forward_imports` 中敲定它（`js.rs:843-906`，解析在 `:903`）。重复的并发导入通过 `graph.same_origin()` 合并。
- **dune**：同样的设计，只差一个文件（`src/hooks.rs:145+`，敲定在 `runtime.rs:600-609`），包括 `same_origin` 合并列表。
- **LLRT**：委托给 `rquickjs` 的加载器；`run_file` 字面上就是 `import("<path>").catch((e) => {console.error(e);process.exit(1)})`（`vm.rs:171-176`）。
- **txiki.js**：QuickJS-ng 在加载器函数之上原生实现 `import()`，因此没有手写的 promise。

### 3.4 CommonJS 互操作——真正的分界线

只有 LLRT 做这件事，因为 AWS SDK v3 会把 CJS 形状的包泄漏进 ESM 图。机制是两个魔法前缀：`__cjs:`（"这条路径来自一次 CJS 导入"）与 `__cjsm:`（"在加载器中强制按 CJS 处理"）（`llrt_modules/src/lib.rs:79-81`）。

- **`import` 一个 CJS 文件** → `PackageLoader::load_cjs_module` *合成模块源码*（`package/loader.rs:20-58`）：`const value = require("..."); export default value.default||value;` 再加上从运行时 `Object.keys(module.exports)` 推导出的具名导出。
- **`require` 一个 ESM 文件** → `Module::import` + §2.4 描述的嵌套泵，然后是默认对象导出提升（`require.rs:130-187`）。
- **循环**通过 `RequireState { cache, exports, progress }` 处理；HEAD 提交（`#1824`）增加了一次同一性检查，使得到达同一个进行中对象的规范化路径直接返回它，而不是复制属性。

rsvim、dune 与 txiki 在结构上拒绝 CJS，并在代码里明说：rsvim 把它列为非目标，而 dune 与 txiki 完全不含 `require` 调用（txiki 有一处注释提到它）。

### 3.5 各自的痛点

- rsvim：**CJS 高墙**加上仅 `default` 的 `exports` 解析器。内置 TypeScript 只支持 `.ts`，而且只有构建 `rsvim` 二进制时才可用（`rsvim_core` 的默认 feature 集不含它）。
- dune：**完全没有 npm 方案**，除非使用导入映射或 URL 导入——这是一笔刻意的交易，换来四者中最干净的解析代码。`.jsx`/`.tsx` 支持是真的，不像 rsvim 那样是注释掉的分支。
- LLRT：**打包实际上是强制的**；运行时解析器是逃生舱口。按设计不支持 TypeScript。
- txiki.js：**没有打包器或导入映射就没有 npm**，但它是两个能直接从 CDN URL `import` 的运行时之一（txiki 通过 libwebsockets 编译它；dune 缓存它）。

---

## 4. 内置运行时 API

### 4.1 rsvim —— 有意最小化，编辑器优先

`globalThis` 只实现 WinterTC 的最小集（`js/runtime/00__web.js:486-493`）：定时器、`queueMicrotask`、`reportError`、`TextEncoder`/`TextDecoder`（由 `encoding_rs` 支持，因此独立于 `icudata` feature）。其余一切都挂在 `Rsvim` 上（`01__rsvim.js:1795-1801`）：`buf`、`cmd`、`fs`、`opt`、`proc`、`rt`、`syn`。

| 分组 | 是否存在 | 说明 |
|---|---|---|
| 定时器 | ✅ | 底层是 tokio sleep |
| 文本 | ✅ | `encoding_rs` |
| **`console`** | ❌ | 用 `Rsvim.cmd.echo()` |
| fetch / URL / Headers / Blob | ❌ | `url_loader` 被注释掉 |
| streams / EventTarget / AbortController | ❌ | |
| crypto / WebCrypto | ❌ | |
| `fs` | ✅ Deno 风格：`open`/`readFile`/`readTextFile`/`readDir`/`stat`/`lstat`/`mkdir`/`symlink`/`link` + `*Sync`；`File` 带 `[Symbol.dispose]` | 不是 Node 形状 |
| `process` / `child_process` | ✅ `Rsvim.proc`（Deno.Command 形状） | `ChildProcess` 带 `[Symbol.asyncDispose]` |
| stdio | ⚠️ 仅 stdout/stderr 可读 | 无可写的 stdin |
| 编辑器 API | ✅ buffer、命令、选项、语法/tree-sitter | 真正的产品 |
| workers | ❌ | |

错误是 UI 事件，不是进程退出：未捕获异常与未处理 rejection 变成命令行消息（`js/err.rs:164-166`、`js.rs:1133-1225`），而 `Rsvim.rt.exit` 是唯一的退出路径。没有 `console`，因为编辑器有命令行。

### 4.2 dune —— "Node，为业余运行时而生"

19 个模块注册在 `CORE_MODULES`（`src/modules.rs:28-52`），并作为 `process.binding(name)` 命名空间加上顶层模块安装：`console`、`events`、`process`、`timers`、`assert`、`util`、`fs`、`perf_hooks`、`colors`、`dns`、`net`、`test`、`stream`、`http`、`sqlite`、`@web/abort`、`@web/text_encoding`、`@web/clone`、`@web/fetch`。全局是 `process`、`console`、`queueMicrotask`、`prompt`、`setTimeout`/`setInterval`/`setImmediate`（+ 对应 clear）、`TextEncoder`/`TextDecoder`、`structuredClone`、`AbortController`/`AbortSignal`、`fetch`、`print`、`reportError`（`src/js/main.js:3-66`、`src/bindings.rs:56-58`）。

| 分组 | 是否存在 | 说明 |
|---|---|---|
| `fs` | ✅ Node 形状：10 个异步 + 10 个 `*Sync` + `createReadStream`/`createWriteStream`/`watch` | `File` 类，`open`/`stat`/`mkdir`/`rm`/`copyFile` |
| `net` | ✅ `createServer`/`createConnection`，`Socket`/`Server` 基于 `EventEmitter` | 含 `TimeoutError` |
| `http` | ✅ **客户端 + 服务端**（`createServer`、`request`、`Server`、`ServerRequest`） | 基于 `httparse` + net 手写 |
| `sqlite` | ✅ 基于 rusqlite（bundled）的 `Database` / `DatabaseSync` | |
| `dns` | ✅ `lookup` + IPv4/IPv6 校验器 | |
| `events` / `stream` / `assert` / `util` / `perf_hooks` | ✅ / ⚠️ | `stream` 只有 `pipeline`/`compose`；`util` 只有一个函数；`assert` 是 692 行的重实现 |
| `console` / `process` / `timers` | ✅ | `console` 包装 V8 的 console；`process` 有 argv/env/cwd/exit/nextTick/binding/kill/uptime/memoryUsage |
| `test` | ✅ 内置 runner（`dune test`） | 受 Deno 启发 |
| `fetch` | ⚠️ `http.request` 之上的包装 | "not fully compatible with WHATWG fetch"（README） |
| URL / Buffer / EventTarget / crypto / child_process / workers / WebSocket | ❌ | 也没有 `path`，没有 `os` |
| Inspector | ✅ 经 axum WebSocket 的 Chrome DevTools 协议 | `--inspect[=addr]` |

`process.binding` 是相对于 rsvim 的承重创新：原生命名空间在一张 `lazy_static` 的初始化器注册表里查找（`src/bindings.rs`、`src/process.rs:229-237`），并在 JS 中缓存（`src/js/process.js:4-21`）。它让 dune 拥有 Node 形状的内部 ABI，却没有 Node 形状的公开 ABI。

### 4.3 LLRT —— "为 Lambda 的 Node 子集"，最宽的 API 面

通过 `base` 默认启用——27 个 feature 条目注册了 **29** 个模块名（`llrt_modules/Cargo.toml:43-71`；其中五个条目是全局安装器，不是模块）。对照 `modules/` 目录树而非 README 矩阵核验：

| Node API | 状态 | 说明 |
|---|---|---|
| `fs`, `fs/promises` | ✅ | 大多数异步调用有 `*Sync` 孪生；`open` 没有 `openSync` |
| `path`, `os`, `url`, `util`, `buffer`, `events`, `zlib` | ✅ | `events` = `EventEmitter` + `EventTarget` |
| `net`, `dns`, `dgram`, `tty` | ✅ | |
| `tls` | ❌ | 没有 `node:tls`；该 crate 只为 `https`/`fetch` 保存客户端配置 |
| `child_process` | ✅ | `spawn` / `execFile` |
| `crypto` | ✅ | 默认 RustCrypto；可选 ring / graviola / openssl |
| `module` | ✅ | `builtinModules`、`createRequire`、`isBuiltin`、`registerHooks` |
| `perf_hooks`, `string_decoder`, `assert` | ✅ / ✅ / ⚠️ | `assert` 是 78 LOC |
| `timers`, `console`, `abort`, `async_hooks` | ✅ | `async_hooks` 需经 `LLRT_ASYNC_HOOKS=1` 选用 |
| `stream`, `stream/promises` | ⚠️ 仅 JS | 约 230 行的适配器，架在随源码附带（vendored）的 `readable-stream` 4.7.0 之上 |
| `stream/web` | ✅ | WHATWG streams，约 14.9k LOC——最大的模块 |
| `http` | ❌ 缺失 | 没有 `http` 模块 |
| `https` | ⚠️ | 已注册，但只导出 `Agent`；承载内部的 hyper 客户端 |
| `http2`, `cluster`, `vm`, `v8`, `wasi`, `inspector`, `readline`, `querystring`, `sqlite`, `worker_threads` | ❌ | 作为模块缺失——HTTP/2 *可以*通过 `fetch` 到达（`LLRT_HTTP_VERSION=2`） |

Web 平台：`fetch`（基于 hyper + rustls 的 Request/Response/Headers/FormData/Blob，后端可选）、WHATWG streams、`URL`、`TextEncoder`、WebCrypto、`EventTarget`/`Event`/`CustomEvent`、`AbortController`、`structuredClone`（含 `transfer`）、`performance`、`Blob`/`File`、压缩 body。缺失：**`WebSocket`**（零命中）、`XMLHttpRequest`、WebAssembly、`EventSource`。`navigator` 是 19 行的桩。`Intl` 是 `jiff` 之上、feature 门控的最小 `DateTimeFormat`/`supportedValuesOf` 子集，不是 ECMA-402；`Temporal` 同样被门控。

LLRT 独有：`llrt:qjs`、`llrt:codec`、`llrt:util`、`llrt:xml`（最后一个存在是为了给 AWS SDK 垫 `fast-xml-parser`）。再加上它的存在理由：`sdk.cfg` + `build.mjs` 预注册 `@aws-sdk/client-*` 入口点，`llrt_core/src/runtime_client.rs` 通过 hyper 讲 Lambda Runtime API。

### 4.4 txiki.js —— "Web，外加一个刀具抽屉"

两层：C 暴露一个由原语组成的 `core` 对象（27 个命名空间初始化器——26 个 `mod_*` 加 WebCrypto——`src/vm.c:241-268`），以 `tjs:internal/core` 安装，再由 `src/js/` 包装成标准形状的全局。`core` 对象是内部 ABI：除非导入方自身是 `tjs:` 模块，否则 `tjs:internal/*` 会被拒绝（`src/modules.c:343-351`）；公开接口面是冻结的 `tjs` 全局（`src/js/core/index.js:231`）。

Web 平台：`fetch`（H1/H2/H3、代理、cookie jar）、`WebSocket` + `WebSocketStream`、`EventSource`、`XMLHttpRequest`、streams + `CompressionStream`、`Blob`/`File`/`FileReader`/`FormData`、`AbortController`、`EventTarget`/`Event`/`CustomEvent`、`URL`/`URLPattern`/`URLSearchParams`、`TextEncoder`/`TextDecoder`（+ stream 变体）、WebCrypto（mbedtls；Ed25519/X25519 用 TweetNaCl）、`structuredClone`、`performance`、`navigator`、`localStorage`（SQLite 支撑）/ `sessionStorage`（内存）、`Worker`、`MessageChannel`、`BroadcastChannel`、`console`、定时器、`WebAssembly`（WAMR **解释器**）、Direct Sockets。

Node 式 / 系统式：地址查询，TCP/UDP/TLS/pipe socket（经 `tjs.lookup` 与 `tjs.connect`/`tjs.listen`，外加 Direct Sockets），带 WS 升级的 HTTP+H2+H3 **服务端**，子进程，信号，stdio，`path`。txiki 独有的 stdlib：`tjs:sqlite`、`tjs:ffi`（libffi）、`tjs:hashing`、`tjs:ipaddr`、`tjs:readline`、`tjs:uuid`、`tjs:getopts`、`tjs:posix-socket`、`tjs:wasi`、`tjs:utils`、`tjs:assert`、`tjs:path`。

缺失：**任何 `node:` 命名空间**、`require`、`Buffer`、`EventEmitter`、`process`。有一个冻结的 `tjs.*` 全局，带类 fs 辅助函数，但不是 Node 形状。也没有公开的 `*Sync` fs API。

构建期 feature 会改变接口面：`BUILD_WITH_WASM=OFF` 移除 `WebAssembly` 全局与 `tjs:wasi`；`BUILD_WITH_SQLITE=OFF` 移除 `tjs:sqlite` 并把 `localStorage` 降级为内存。`tjs.engine.{compile,serialize,deserialize,evalBytecode,gc}`（`src/js/core/engine.js:5-31`）是公开的字节码 API，`tjs compile` 与嵌入的预编译代码都建立在它之上。

### 4.5 覆盖面裁决

| | rsvim | dune | LLRT | txiki.js |
|---|---|---|---|---|
| Node.js 兼容性 | ✗（约 0） | ★★（fs/net/http/events/assert/process） | ★★★☆（刻意的子集） | ✗（按策略，无） |
| Web/WinterTC | ☆（定时器 + 文本） | ★（fetch-lite、文本、abort） | ★★★ | ★★★★ |
| 系统 / 原生 | `fs`、子进程 | fs、TCP、HTTP 服务端、SQLite | `net`、`dns`、`dgram`、`zlib` | socket、TLS、HTTP **服务端**、sqlite、FFI、WASM、Worker |
| 编辑器领域 | ★★★★ | ✗ | ✗ | ✗ |
| 并发 | ✗ | ✗ | ✗（单 isolate） | ★★★★（`Worker` = 线程 + 运行时 + 循环） |
| 工具链 | 快照构建 | REPL、测试 runner、打包器、编译器、inspector | REPL、测试 runner、字节码编译器 | REPL、测试 runner、esbuild 打包器、字节码/独立编译器、应用打包器 |
| 差距 | `console`、`URL`、`fetch`、streams、`EventTarget` | `URL`、`Buffer`、crypto、workers、npm | `WebSocket`、workers、`node:http2` 模块、`vm`、WASM | Node 兼容性（有意为之） |

---

## 5. 宿主函数实际如何到达 JS

这决定了新增一个 API 的成本有多高。

- **rsvim** —— 手写的 `v8::Function` 回调安装进一个隐藏全局 `__InternalRsvimGlobalObject`（`js/binding.rs:58`），参数转换经由 `rsvim_macro::ToV8`/`FromV8` derive。JS 层是有类型的、手写的，并且在构建时**用 terser 压缩**（`dev.py:360-365`）；真正运行的是检入的 `*.min.js`（`js.rs:157-173`），所以只改 `01__rsvim.js` 不会有任何变化。
- **dune** —— 手写的 `v8::Function` 回调安装进 `process.binding(name)` 命名空间对象，再由可读的 Node 风格 JS 模块包装（`src/js/*.js`）。没有宏层：参数校验是手动的 `v8::Local::<T>::try_from`。新增一个 API = 一个 Rust fn + 一条注册表项 + 一个 JS 包装。
- **LLRT** —— `rquickjs` 的 `ModuleDef` trait（不是 `#[rquickjs::module]` 属性宏，后者零使用）。每个模块注册一个名字与 `LoadFn`；异步函数无需特殊处理：`Func::from(Async(read_file))`。
- **txiki.js** —— 普通的 C 函数，用 `TJS_CFUNC_DEF` 放进 `JSCFunctionListEntry` 表，挂到扁平的 `core` 对象上，再由 `qjsc` 编译成字节码的 JS 包装。

一个判断代码库成熟度的有用线索：rsvim 与 dune 都在大致相同的抽象层次上手写 V8 回调，但 rsvim 从 Rust 类型生成绑定，而 dune 在运行时校验。

---

## 6. 启动与分发

| | rsvim | dune | LLRT | txiki.js |
|---|---|---|---|---|
| 机制 | V8 **启动快照**：`build.rs` 实例化一个快照 isolate，求值两个内置模块，`create_blob(FunctionCodeHandling::Keep)` → `RSVIM_SNAPSHOT.BIN` → `include_bytes!`（`rsvim_cli/build.rs:91-102`、`js.rs:157-202`） | 无快照：`include_str!` 全部 19 个核心模块，每次运行首次导入时重新编译 | `build.rs` 把每个内置 JS 编译成 QuickJS 字节码，用 40 KB 训练字典做 zstd 压缩（`--maxdict=40K`），嵌入 `phf` 映射（`llrt_core/build.rs`） | `qjsc` 编译的字节码放在 `src/bundles/c/**`，用 `JS_ReadObject` 加载 |
| 谁构建 | **CLI** crate，以 `rsvim_core` 为 build-dependency，`--predictable --random-seed=42` | 不适用 | 核心 crate | 构建系统 |
| 惰性 | 内置模块在快照内已**求值** | 否：每个核心模块在首次导入时解析 | **惰性**，按导入解压 | `core`（约 72 KB）+ `polyfills`（约 291 KB）**每个运行时都急切加载**；`tjs:*` 惰性 |
| 用户代码 | 加载时解析 | 加载时解析；`dune compile` 打包（swc）+ zstd + 追加 8 字节尾标 `4tr31d1s` | 经 `llrt compile` 生成 `.lrt` 字节码；`--executable`/`pack` → 自包含二进制 | 经 `tjs compile` 生成字节码；12 字节 `tx1k1.js` 尾标 |
| 代价 | 约 64 MB 二进制，启动快 | 二进制小，启动较慢（每次运行要解析约 6.4k 行 JS） | 二进制小，启动快 | 二进制小，但每个运行时都要为急切 polyfills 付费——包括每个 worker |

取舍三角：rsvim 用二进制体积买启动；LLRT 用构建步骤买启动；dune 买简单性，在启动时付费；txiki 买简单性，按运行时付费。

有一个对 rsvim 值得标注的分发细节：因为快照由 `rsvim_cli/build.rs` 产生，单独的 `rsvim_core` 并不是一个带内置模块、可运行的嵌入式运行时——嵌入方继承的是裸 `v8::` API 加上 JS 源码，而不是准备好的堆。

---

## 7. 每种架构优化什么——以及代价是什么

**rsvim：安全性与编辑器一致性优先于吞吐。**
actor/通道边界意味着任何绑定都无法破坏编辑器状态，而且代码库的结构已经为"JS 住在自己线程上"的未来做好了准备。代价：每个异步 op 都序列化自己的结果，循环需要显式的 `TickAgainReq` 推动，微任务是手动的且在 tick 边界设检查点，`*Sync` 调用冻结 UI，JS 接口面仍然很小（缺 `console`；`fetch`/`URL`/streams 都不存在）。就其实际工作而言——在 TUI 出现之前运行一次 `rsvim.{ts,js}` 配置，注册命令并编辑 buffer——这是一笔站得住脚的交易。

**dune：Node 形状的编程模型，没有 Node 引擎。**
相对其体量，它是四者中最"电池齐全"的：Node 风格的 `fs` 带同步孪生、HTTP 服务器、SQLite、测试 runner、打包器、独立编译、REPL 和真正的 Chrome DevTools inspector。代价是循环是一份重实现：poll 超时计算、ref 语义、信号、fs 监听和 TCP 全都住在一个 0.2.0 的 crate 里，而 JS 层面的活跃性谓词加上 waker 中断之所以存在，正是因为循环看不见 V8 的队列。它是一个循环，但还不是运行时级的循环。

**LLRT：接口面优先于循环创新。**
它是唯一有真正 CJS 互操作与 Node 形状 API 的，也是唯一把 npm 当作*构建*问题、同时仍然交付运行时解析器的。它的事件循环整个借自 `rquickjs`，这就是为什么它自己的胶水只有约 3.8k LOC，尽管整棵树约 73k 行 Rust。代价是与该运行时的调度刚性耦合（在接缝处用手写的定时器与 `require` 泵打补丁）、没有 worker，以及一条必须在部署前打包的构建流水线。

**txiki.js：控制循环，标准优先于兼容性。**
亲手拥有 `uv_prepare`/`uv_check`/`uv_idle` 的三步舞，就能精确控制 JS 何时运行，而正因如此，真正的 OS 线程才变得可控：每个 `Worker` 都在新线程上获得自己的 `JSRuntime`、自己的上下文和自己的 `uv_loop_t`，通过堆邮箱 + `uv_async` 上的 `JS_WriteObject2`/`JS_ReadObject2` 结构化克隆交换消息。Web 优先的 API 加上 FFI/SQLite/WAMR 让它成为四者中最有能力的"小"运行时。代价是处处 C 内存管理、没有 CJS，以及没有打包器或导入映射就没有 npm。

关于"可嵌入"的一点注意事项：txiki 在*架构上*是可嵌入的（`add_library(tjs STATIC)` + 一个 48 行的 `tjs.h`，九个 `TJS_*` 入口点），但没有 `install()` 规则、没有导出的 CMake 包、没有 pkg-config 文件、也没有版本化 ABI——而且除非运行时是 worker，`TJS_Run` 总是求值 CLI 的 `run-main` bundle。嵌入方的旋钮相应地很薄：`TJS_NewRuntimeOptions` 只暴露 `mem_limit`/`stack_size`（`tjs.h:33-36`），只有 worker 运行时可以阻塞（`JS_SetCanBlock`，`src/vm.c:527`），而 `TJS_Stop` 通过 `uv_async_send` 做到线程安全（`src/vm.c:1130-1133`）。它是构造上的库，而不是契约上的库。

---

## 8. 如果你正在做决定

- **把 JS 嵌入一个已经拥有时间的宿主应用**（编辑器、游戏、服务器）→ rsvim 的形状。让宿主的循环驱动一切，绝不让 JS 阻塞它，接受通道与序列化的额外开销。另一条路——嵌入一个 libuv 形状的运行时并从你的循环里泵它——是 dune 的形状，它可行，但你继承了第二个调度器。
- **交付一个独立的、Node 风味的工具，API 覆盖要广** → 要广度就选 dune 的形状（HTTP 服务端、SQLite、REPL、inspector），要 Lambda 冷启动与 CJS 互操作就选 LLRT 的形状。
- **交付一个带 Web API 与 worker 的小型通用运行时** → txiki 的形状。静态库、libuv、标准、FFI 逃生舱口、WAMR。
- **最小化运行时代码** → LLRT。把循环外包给 `rquickjs`，只写泵。

共同线索是**每者把 npm 问题放在哪里**：

- rsvim —— *在解析时解决*（`oxc_resolver` 遍历 `node_modules`，`default` 条件），然后因为禁掉 CJS 而失去大部分生态。
- dune —— *拒绝它*（导入映射、URL 导入或预打包）。
- LLRT —— *在构建时解决*（esbuild + shims），把忠于 Node 的解析器作为逃生舱口。
- txiki.js —— *拒绝它*，并把这种拒绝当作一项特性。

而具体到循环问题：**tokio 想拥有时间；libuv 想拥有句柄；V8 和 QuickJS 都拥有一个谁也看不见的队列。** 一个运行时在很大程度上就是调和这三种主张的胶水——而四者的差异主要在于它们愿意写多少胶水。

---

## 9. 来源 / 注意事项

以上全部读自顶部所列提交的四个工作树；没有构建或基准测试过任何运行时。已知缺口：

- **txiki.js**：`deps/` 子模块（包括 `quickjs`）**未检出**（`git submodule status` 对所有子模块显示 `-`），因此任何住在引擎内部的东西都是依据 txiki 的调用点断言的，而不是依据 QuickJS-ng 源码。这涵盖动态 `import()` 的 promise 敲定、模块缓存键和循环导入语义——txiki 对这些不贡献任何代码，这本身就是已核验的发现。
- **LLRT**：`rquickjs-sys` 0.14.0 随源码附带（vendors）**QuickJS-NG 0.16.2**（`quickjs/quickjs.h:1450-1453`；未记录 git revision），但该 crate 的 Rust 源码没有本地 vendored，因此其调度器是依据 LLRT 的用法与该 crate 的公开 API 描述的。`THIRD_PARTY_LICENSES` 已过时（仍引用 QuickJS 2021-03-27 / rquickjs 0.4.0），README 的兼容性矩阵也有多处过时（仍把 `dgram` 标为 ✘）；以 `modules/` 树为准。
- **dune**：这里没有构建发布二进制，因此二进制体积与启动延迟未测量，`crabuv` 读的是其钉住的 rev `2fda099`（v0.2.0）。V8 引擎版本除了 `rusty_v8` crate 版本（`150.2.0`）之外未作断言；`process.versions.v8` 会在运行时报告它。
- **rsvim**："约 0 的序言（prologue）成本"与启动延迟是架构主张，不是测量；二进制体积（64,227,600 B）与版本字符串读自 HEAD 上一个已有的发布构建。
- **两个 V8 运行时**共享继承来的 ``JSON.parse(`…`)`` 与按 UTF-8 读 `.wasm` 的加载器缺陷；这些是作为代码层观察报告的，没有用 fixture 实测。
