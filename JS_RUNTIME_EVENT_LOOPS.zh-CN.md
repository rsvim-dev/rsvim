# 事件循环架构：rsvim vs. dune vs. LLRT vs. txiki.js

**libuv vs. tokio，以及为什么它几乎决定了其余一切。**

本文只聚焦一个问题——*谁拥有时间，以及 JavaScript 的工作如何被拥有时间的那一方看见？*——并为四个可嵌入运行时给出答案。它是 `JS_RUNTIME_COMPARISON.md` 的深度配套篇，后者涵盖模块、API 面、启动与分发。

分析的快照（本地工作树；未构建或基准测试任何运行时）：

| | 仓库 | HEAD | 语言 | 引擎 | 循环宿主 |
|---|---|---|---|---|---|
| **rsvim** | `rsvim-dev/rsvim` | `ef9a345` (0.1.3-alpha.2) | Rust | V8 15.0.245.2 (`rusty_v8` 150.4.0) | **tokio**——编辑器的 `select!` |
| **dune** | `aalykiot/dune` | `5958b64` (v0.11.4) | Rust | V8 (`rusty_v8` 150.2.0) | **crabuv**——一个架在 `mio` + `rayon` 之上、形似 libuv 的循环 |
| **LLRT** | `awslabs/llrt` | `d9fd873` (v0.9.0-beta-147) | Rust | QuickJS-NG 0.16.2，经由 `rquickjs` 0.14 | **rquickjs** 的 `AsyncRuntime`，跑在 tokio 上 |
| **txiki.js** | `saghul/txiki.js` | `8a9b6044` (v26.6.0-136) | C | QuickJS-NG | **libuv** |

这个 2×2 的格局整齐得几乎可以当作论点：**两个 V8 运行时、两个 QuickJS 运行时；两个自己驱动循环，两个借用别人的循环。** 但两条轴并不对齐——dune 是 V8 *且*拥有循环，LLRT 是 QuickJS *且*借用循环——而错位之处，正是有意思的工程所在。

---

## 0. 一句话版本

| | 谁拥有事件循环？ |
|---|---|
| rsvim | **宿主编辑器拥有**——在输入、通道和取消之上做 `tokio::select!`；JS 只是乘客 |
| dune | **dune 拥有**——`JsRuntime::run_event_loop` 驱动 `crabuv` 的 `RunMode::Once` 迭代 |
| LLRT | **rquickjs 拥有**——`vm.idle().await`；LLRT 没有*主*循环，只有若干个泵送器 |
| txiki.js | **txiki 拥有**——`uv_run`，外加一座 `uv_prepare`/`uv_check`/`uv_idle` 的作业队列桥 |

---

## 1. tokio 和 libuv 并不是同一个想法的两种实现

这是最容易被一笔带过的地方，而后面的一切都由此推出。

**libuv 是一张带回调的句柄表。** 每个待处理的东西都是一个引用计数的句柄——`uv_timer_t`、`uv_tcp_t`、`uv_work_t`、`uv_idle_t`。`uv_run` 跑一次迭代：到期定时器、轮询、check、close。当且仅当仍有被引用的句柄（或活跃请求）时，循环才算"活着"，而这个引用计数是一个*公开、可检查*的事实（`uv_ref`/`uv_unref`、`uv_loop_alive`）。时间是循环的一等设施。调用 `uv_run` 的宿主只有在回调跑完或循环排空后才重新拿回控制权。

**tokio 是一个带 reactor 的 future 执行器。** 时间和 I/O 都以 future 的形式暴露；推进就是轮询一棵任务树；waker 决定何时轮询。这里没有可检查的句柄表，没有 `loop_alive`，没有"第 N 次迭代的回调"这一概念，也没有引用计数。宿主 await，执行器决定接下来跑什么。时间是一项全局设施（运行时内部的时间轮），而不是循环的一个阶段。

**JS 引擎对这两者都是第三方。** QuickJS 有一个*作业队列*，只有一个入口（`JS_ExecutePendingJob`，并用 `JS_IsJobPending` 询问是否还有工作）。V8 有一个*微任务队列*，配一个显式策略下的检查点——但没有 `JS_IsJobPending` 的等价物；你可以问后台编译是否待处理，却问不了微任务是否已排队。

所以每个运行时都必须回答一个其基底回答不了的问题：**"还有 JavaScript 工作剩下吗，它该在什么时候跑？"** 四个答案就是四种架构。

---

## 2. 四种循环

### 2.1 rsvim——tokio 当消息总线；JS 根本没有循环

`rsvim_cli/src/bin/rsvim.rs:107` 创建一个多线程的 `tokio::runtime::Runtime::new()`，并对编辑器 future 调用 `block_on`，因此即便有工作线程来跑派生任务，V8 仍留在主线程上。程序整个控制流就是一个 `select!`（`rsvim_core/src/evloop.rs:1347-1365`），在终端输入、master 消息、JS 消息和取消信号之间做选择。

`JsRuntime` 是**编辑器 `EventLoop` 结构体的一个字段**（`evloop.rs:142`），JS 在某个 select 分支内部同步推进：

```rust
// rsvim_core/src/evloop.rs:1321-1327
async fn forward_js_message(&mut self, message: Option<JsMessage>) {
    if let Some(message) = message {
        self.jsrt_tx.send(message).unwrap();
        self.js_runtime.tick_event_loop();
    }
}
```

一个 tick（轮次）是 `checkpoint → fast_forward_imports → run_pending_futures`（`js.rs:610-614`）。任何地方都没有 `uv_run` 的对应物：编辑器循环*就是*运行时循环，而 JS 是它的一个子过程。

原生异步是一套 actor 协议。绑定层投递一条 `MasterMessage`；select 分支在 `detached_tracker` 上派生一个 tokio 任务（`evloop.rs:816-835`）；该任务执行 I/O，并通过一条**回环（loopback）**通道发回 `JsMessage`，这条消息唤醒 select，重新进入 `forward_js_message`。结果以 `postcard` 字节的形式跨越边界（`evloop.rs:828`）——这是一条刻意的所有权边界，代价是每次操作都要编解码。

最能说明问题的时刻，是 JS 需要轮到自己、而 tokio 无事可做的时候。rsvim 给自己发消息：

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

master 的处理方式，是派生一个任务，其全部函数体就是把 `TickAgainResp` 原样回送（`evloop.rs:810-814`）。这相当于用一次通道往返，在执行器上拼出一个 `await`：两次调度器跳转，只为换来一次 JS 轮次。

定时器同样不是循环的设施。每个定时器都会派生一个 `tokio::time::sleep_until` 任务，到期时投递一条消息（`evloop.rs:774-789`），而 `setInterval` 通过重新插入回调来重新挂载（`js.rs:687-696`）。因此，间隔漂移完全取决于 tokio 调度器给你什么。

微任务是显式的（`js.rs:144-146`），并且只有**一个**检查点，位于一个 tick 的*开头*（`js.rs:1097-1131`）。`run_next_tick_callbacks` 只做检查点；它的 nextTick 函数体被注释掉了——Deno 骨架的残留。因此，在排空 future 期间敲定的 promise，其续体要到下一个 tick 才会被排空。

"JS 只是乘客"这一立场的两个后果，都值得直说：

- 配置模块**只运行一次，且在 TUI 存在之前**（`evloop.rs:541-574`；回显的消息先被缓冲，初始化后再冲刷，`evloop.rs:671-689`）。没有 REPL，没有 `eval`。
- `*Sync` API 在编辑器任务上就地执行（`binding/global_rsvim/fs/open.rs:176-197`），所以一次慢的同步调用会同时冻结输入*和*渲染。没有第二个线程可以把它藏起来。

### 2.2 dune——用 Rust 写 libuv，再从 JS 驱动它

`crabuv`（aalykiot 的 crate，v0.2.0）**不是 libuv**：它是从零开始的重实现，架在 `mio`（就绪通知）、`rayon`（线程池）、`notify`（文件系统事件）和 `signal-hook` 之上。但它忠实复制了 *API 与迭代顺序*：

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

`has_pending_events()` 是 `uv_loop_alive` 的克隆——资源 + 排队请求 + 在途线程池任务（`event_loop.rs:657-661`）——而 `RunMode::{Default, Once, NoWait}` 对应 `UV_RUN_*`。`handle.spawn(work, cb)` 就是 `uv_queue_work`：作业跑在一个 **rayon** 工作线程上，该线程发送 `Event::ThreadPool` 并调用 `waker.wake()` 来打断轮询（`event_loop.rs:454-466`）。

运行时拥有外层循环，而 JS 工作正是在它的循环条件里变得可见：

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

这四个 JS 谓词是 dune 的发明，因为循环看不见 V8 的队列。一个 tick 是 `nextTick → fast_forward_imports → event_loop.run(Once) → run_pending_futures`（`runtime.rs:427-432`）。注意由此产生的顺序后果：**JS 回调不在循环迭代内部运行。** 完成回调把一个 `JsFuture` 推入 `pending_futures`（`src/file.rs:145-160`），运行时在 `uv_run` 返回后排空那个 vector（`runtime.rs:479-504`），并在那时敲定 promise。

由于 libuv 无事可轮询时仍可能存在就绪的 JS future，dune 必须自己打断轮询——txiki 用 `uv_idle` 解决同一个问题，dune 则用一个 waker 和一个标志位解决：

```rust
// dune/src/timers.rs — inside create_timeout's loop callback
state.pending_futures.push(Box::new(future));
if !state.wake_event_queued {
    state.interrupt_handle.interrupt();   // mio::Waker
    state.wake_event_queued = true;
}
```

`setImmediate` 是一个 `CheckHandle`（uv 的 check 阶段），不是定时器。REPL 以微缩形式依赖同一套机制：一个空的 `u32::MAX` 间隔定时器让循环保持存活，而只要有行输入，`rustyline` 线程就会打断循环（`src/repl.rs:281-294`）。

### 2.3 LLRT——rquickjs *就是*循环

`#[tokio::main]` 加上 `vm.idle().await`（`llrt/src/main.rs:115,131`）。VM 包装层总共 219 行（`llrt_core/src/vm.rs`），而 `idle()` 只是一次委托：

```rust
// llrt_core/src/vm.rs:29-32, 194-197
pub struct Vm { pub runtime: AsyncRuntime, pub ctx: AsyncContext }
pub async fn idle(self) -> ... { self.runtime.idle().await; Ok(()) }
```

`rquickjs` 的 `AsyncRuntime` 在 QuickJS 作业队列旁边维护一个侵入式的 Rust future 调度器；`idle()` 在同一把 `async_lock::Mutex` 下，把 `JS_ExecutePendingJob` 与对该调度器的轮询交替进行。JS future **不是 tokio 任务**——它们住在那个调度器里，只在 `idle()` / `drive()` 中被轮询，这就是为什么多线程的 tokio 进程仍然会把 JS 串行化。原生异步函数不需要任何额外接线：`Func::from(Async(read_file))` 会变成 `Promise::wrap_future` → `JS_NewPromiseCapability` + `ctx.spawn`；只有底层的 I/O 被交给 tokio 卸载，future 本身仍在 JS 线程上被轮询。

凡 `rquickjs` 没有概念的地方，LLRT 就自己写泵送器：

- **定时器**（`modules/llrt_timers/src/lib.rs:35-41`、`305-398`）：一个带 `deadline` 和 `Notify` 的 `RuntimeTimerState`，由一个带 `select! { notify, sleep }` 的任务驱动。它把休眠时间钳制到 `MIN_SLEEP = 4ms`，在第一个回调之前排空一次作业队列，并在每个回调之后各排空一次，还通过给 `setImmediate` 一个早于现在 600 秒的截止时间，让它排在最前面。
- **`require`**（`llrt_modules/src/module/require.rs:111-121`）：一个同步 CJS require，建立在异步的 `Module::import` 之上，靠空转作业队列——并就地触发到期定时器——直到 import 敲定。
- **Lambda 调用**（`runtime_client.rs:400-410`）：一个针对 Runtime API 的请求/响应循环。

所以 LLRT 终究还是写了循环代码——只是不是 JS 循环：大约 150 行泵送代码，它们的存在恰恰是因为 `rquickjs` 会调度 future，却对定时器或同步 `require` 一无所知。

单队列设计带来两处语义偏离：`process.nextTick` 就是 `queueMicrotask`（两者都走 `cb.defer`），并且不存在 ref/unref 记账——定时器句柄是不带任何 `.unref()`/`.ref()` 的裸数字 ID，而 `dgram` 的这些方法被文档化为空操作（`modules/llrt_dgram/src/socket.rs:527-539`）。

### 2.4 txiki.js——真正的 libuv，嫁接上作业队列

`TJS_Run` 启动 `uv_prepare`/`uv_check`，把两者都 **unref**（`src/vm.c:1073-1076`），然后循环：

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

关键在一个函数：

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

一个被 ref 的 `uv_idle` 句柄既能*让* `uv_run` 保持存活，*又*会把 libuv 的轮询超时强行压到零，因此 JS 工作永远不会被阻塞式轮询搁浅。`uv_check` 运行作业并重新求值（`vm.c:1039-1046`）。这是四者中最干净的答案，因为它使用 libuv 自己的引用计数，而不是发明谓词。

微任务在**三**个点排空：`uv__check_cb`；每个经由 `tjs_call_handler` 路由的回调之后（`src/utils.c:205`，13 个文件中共 41 处 C 调用点）；以及 `EventTarget.dispatchEvent`（`src/js/polyfills/event-target.js:657` → `core.drainMicrotasks()`），其重入保护 `draining_microtasks` 位于 C 绑定中（`src/vm.c:184-198`）。

原生异步使用 libuv 的线程池：`uv_queue_work` 在 `src/mod_fs.c:1365`，一个在 `:1339` 敲定 promise 的 after-work 回调，promise 在 `:1372` 创建。每个定时器对应一个 `uv_timer_t`。从不重入 `uv_run` 的代价是：HTTP import 会阻塞循环——一个 `while (!load_ctx->done) lws_service(...)` 循环，每次尝试有硬性 10 秒截止时间，最多 20 次重定向（`src/lws-utils.c:798-843`），全都发生在一个 JS 求值内部。

退出语义是四者中最锋利的：`tjs.exit(n)` 立即调用 `uv_tty_reset_mode(); exit(n);`（`src/mod_os.c:37-38`），并**绕过** `beforeunload`。可取消的 `beforeunload` 只在循环*正常*终止时触发，在未捕获异常或未处理拒绝之后则完全跳过（`src/vm.c:1103-1119`）。

---

## 3. 基底解决不了的问题

### 3.1 活跃性，并排对比

| | "还有 JS 工作剩下吗？" | 什么让进程保持存活 | 唤醒机制 |
|---|---|---|---|
| rsvim | `tick_event_loop` 里的四个谓词（`js.rs:627-637`） | 编辑器永远运行；只能通过 `Rsvim.rt.exit` 退出 | 通道；JS 需要轮次时走 `TickAgainReq` 往返 |
| dune | 四个谓词 + `has_pending_events()`（`runtime.rs:442-465`） | 被引用的 `crabuv` 资源 | `mio::Waker`（`interrupt_handle`）；工作线程完成时调用 `waker.wake()` |
| LLRT | `rquickjs` 调度器（对 LLRT 不可见） | `idle()` 中尚未完成的驱动 future | 定时器驱动任务里的 tokio `Notify` + `Sleep` |
| txiki.js | `JS_IsJobPending`，由 `uv__maybe_idle` 查询 | libuv 引用计数；作业待处理时 `uv_idle` 处于被引用状态 | libuv 自身 |

模式是：**libuv 给了你一套词汇，把"JS 待处理"表达为句柄状态，而 tokio 没有。** txiki 在一个函数里直接使用它。dune 有同样的词汇，但对 libuv 的保证只有近似，于是它在循环*之外*加上 `has_next_tick_callbacks()` 之流，然后再打断循环去处理它们。rsvim 无法打断自己的 select，就给自己发一条消息。LLRT 通过借用 `rquickjs` 绕开这个问题，然后为它付出两个泵送器的代价。

### 3.2 原生异步 → promise

| | 原生操作运行在 | 结果如何跨越 | promise 在哪创建 | 在哪敲定 |
|---|---|---|---|---|
| rsvim | tokio 工作线程 / 阻塞池 | 经 `mpsc` 传 `postcard` 字节 | `JsFuture` 里的 `v8::PromiseResolver` | 下一个 tick，从 `pending_futures` 中 |
| dune | rayon 线程池 | `Box<dyn Any + Send>`，在循环里向下转型（downcast） | `JsFuture` 里的 `v8::PromiseResolver` | 循环迭代之后，从 `pending_futures` 中 |
| LLRT | tokio I/O；future 在 JS 线程上轮询 | 就地（借用的 `Ctx`） | `Promise::wrap_future` | 在 `idle()` 期间轮询 future 时 |
| txiki.js | libuv 线程池 / 循环线程 | 就地（循环线程） | `JS_NewPromiseCapability`（fs/dns）或 JS `withResolvers()` | after-work / C 回调 |

### 3.3 微任务顺序

| | 策略 | 检查点 | 后果 |
|---|---|---|---|
| rsvim | `Explicit`（`js.rs:144-146`） | **一个**，在 tick 开头（`js.rs:1097-1131`） | 本 tick 敲定的 future，其续体在下一 tick 运行 |
| dune | `Explicit`（`runtime.rs:148`） | nextTick 之后（`:718`）、imports 之后（`:615`）、每个 future 之后（`:500`）、REPL（`:294`） | 续体在敲定它们的那个 tick 里运行 |
| LLRT | 引擎管理 | 定时器回调前后；`idle()` 内部 | 回调总能看到已排空的队列 |
| txiki.js | 引擎管理（作业队列） | `uv_check` 之后、经辅助函数路由的回调之后、每个监听器之后 | 以单个回调为粒度；由 `tests/test-async-task-sequence.js` 固定验证 |

rsvim 的顺序在两个方向上都不同寻常：检查点在 tick 的*开头*，而 `run_next_tick_callbacks` 里根本没有 nextTick 逻辑（`js.rs:1097-1118`）——这是它血统的残留。

### 3.4 定时器

| | 机制 | 间隔处理 | 备注 |
|---|---|---|---|
| rsvim | 每个定时器一个 tokio `sleep_until` 任务，投递消息 | 每次触发后重建，重置 `start_at` | 会漂移；与 I/O 的相对顺序取决于调度器 |
| dune | 循环原生的定时器集合，在轮询之前运行 | 循环内部的 `TimerKind::Interval` | 真正的循环定时器；`setImmediate` 是 check 句柄 |
| LLRT | 架在 tokio `Sleep` + `Notify` 上的 LLRT 泵送器 | 在 `poll_timers` 中重新挂载 | 4 毫秒下限；`setImmediate` 靠过去的截止时间 |
| txiki.js | 每个定时器一个 `uv_timer_t` | 由循环重新挂载 | 真正的循环定时器，不受作业队列影响 |

### 3.5 线程

| | JS 线程 | I/O 线程 | 串行化 |
|---|---|---|---|
| rsvim | 主线程，在编辑器任务内部 | tokio 多线程运行时 + 阻塞池 | 通道 + `postcard`（JS 从不被跨线程触碰） |
| dune | 主线程 | rayon 线程池（`--threadpool-size`，默认等于核数） | `Send` 装箱 + 循环回调（JS 从不被跨线程触碰） |
| LLRT | 一个线程，处于运行时互斥锁保护之下 | tokio 多线程 | future 在 JS 线程上轮询；结果无需序列化 |
| txiki.js | libuv 线程 | libuv 线程池（4） | 就地；JS 只从循环线程触碰；工作线程各有自己的循环 |

---

## 4. 每种立场的收益与代价

**rsvim：宿主拥有时间。** 当 JS 必须作为应用内部的客人、而应用已经有事件循环时，这是最好的选择——比如编辑器、游戏、自带 reactor 的服务器。JS 的等待永远不会阻塞宿主，关闭则是一次 `TaskTracker` 排空。账单有三重：结果要跨边界序列化，一个轮次必须被*请求*而不是被观察到（`TickAgainReq`），而 `*Sync` 调用会冻结 UI，因为没别的地方可跑它们。

**dune：没有 libuv 的 libuv 模型。** 它获得了回调阶段的编程模型和带类型的 Rust 回调，并且能与 `spawn` 干净地集成。账单是它必须重新实现那些让 libuv 可信的部分——轮询超时计算、引用语义、信号、文件系统监视、TCP——而且它仍然需要 JS 层面的活跃性谓词加一个 waker 中断，因为 `mio` 不提供 libuv 的保证。它是一个循环，但还不是运行时级的循环。

**LLRT：借一个调度器，写泵送器。** 循环代码是四者中最少的（219 行的 VM 文件），因为 `rquickjs` 已经解决了没有 V8 的调度问题。账单是僵硬的耦合：LLRT 想要而 rquickjs 不建模的一切——定时器、同步 `require`、Lambda 调用——都变成手写的泵送器，而它必须知道何时排空作业队列。

**txiki.js：拥有循环，拥有顺序。** 唯一一个把"JS 待处理"当作一等循环状态的地方。这换来了精确的回调顺序、可取消的 `beforeunload`，以及作为真正"运行时 + 循环"配对的 worker。账单是到处都需要 C 层手工桥接，一个会被 HTTP import 卡住 10 秒的循环，以及一条绕过优雅退出机制的退出路径。

**压缩成一句：** 如果你今天用 Rust 写一个运行时，选择不是"tokio 还是 libuv"——而是"我想当谁的循环的子过程？"rsvim 回答*宿主的*；LLRT 回答*rquickjs 的*；dune 回答*我自己的，重新实现*；txiki 回答*libuv 的，继承而来*。

---

## 5. 核验说明

这里的一切都是在固定 commit 上从四棵代码树里读出来的。针对这些仓库跑了三次源码审计；涉及*循环*论断的更正如下：

- **LLRT**：调度器由 `idle()` 和 `drive()` 轮询，而不是 `async_with()`；`poll_timers` 在第一个回调之前排空一次，之后每个回调各排空一次（不是每个回调之前都排空）；定时器驱动任务、`require` 泵送器和 Lambda 循环合计约 150 行 LLRT 自有的循环代码，所以"没有循环代码"的说法言过其实；定时器句柄完全没有 `unref`/`ref`（比"没有 ref 记账"更强）。
- **txiki.js**：`tjs.exit` 绕过 `beforeunload`；该事件只在正常终止时触发，致命错误之后跳过；微任务排空发生在经由 `tjs_call_handler` 路由的回调之后（41 处），而不是每次 `JS_Call` 之后；核心对象是内部 ABI（`tjs:internal/*` 对用户导入方会被拒绝），因此公开面是冻结的 `tjs` 全局对象。
- **rsvim**：import maps 只是个桩，不是特性；WASM 加载按 UTF-8 读取，所以真实二进制会失败；`File` 暴露的是 `[Symbol.dispose]`，不是 `asyncDispose`；配置在 TUI 之前运行一次；`*Sync` 调用会阻塞编辑器；微任务检查点的位置如上所述。

以下内容在此无法核验，因此只根据调用点断言：

- **txiki.js**：所有 `deps/` 子模块（包括 `quickjs`）都未初始化，所以引擎内部行为——原生 `import()` 的敲定、模块缓存键、循环导入——是从 txiki 自己的代码推断的，而 txiki 自己并不实现其中任何一项。
- **LLRT**：本地没有随源码附带的（vendored）`rquickjs` Rust 源码，所以其调度器是根据 LLRT 的用法和该 crate 的公开 API 描述的。随源码附带的 QuickJS-NG 是 0.16.2。
- **dune**：`crabuv` 是在其固定版本 `2fda099`（v0.2.0）上读的；没有构建发布二进制，所以体积和启动都未测量。
- **rsvim**：启动延迟和"约 0 前导开销"是架构层面的主张；二进制被测过大小（64,227,600 B），但没有计时。
