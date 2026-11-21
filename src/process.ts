import { isBoolean } from "es-toolkit/predicate/isBoolean";
import { isNil } from "es-toolkit/predicate/isNil";
import { isNumber } from "es-toolkit/predicate/isNumber";
import { isString } from "es-toolkit/predicate/isString";
import { assert } from "es-toolkit/util";

/**
 * ---
 * title: process API
 * sidebar_position: 8
 * ---
 *
 * The `process` module for child process and editor process control.
 *
 * @packageDocumentation
 *
 * @categoryDescription Editor APIs
 * These APIs are specific for editor, such as buffers, windows, key mappings, etc.
 *
 * @categoryDescription General APIs
 * These APIs are general for common javascript-based runtime, similar to [Deno APIs](https://docs.deno.com/api/deno/).
 */

/**
 * The `process` module for child process and editor process control.
 *
 * @example
 * ```javascript
 * import process from "rsvim:process";
 *
 * const child = new process.Command("ls");
 * ```
 *
 * @category General APIs
 */
export namespace process {
  /**
   * The command that create a child process.
   */
  export class Command {
    /** @hidden */
    #execPath: string;
    /** @hidden */
    #options: process.CommandOptions;

    constructor(execPath: string, options?: process.CommandOptions) {
      assert(
        isString(execPath),
        `process.Command: execPath must be a string, got ${typeof execPath}`,
      );

      assert(
        options === undefined ||
          options === null ||
          typeof options === "object",
        `process.Command: options must be an object, got ${typeof options}`,
      );
      options = {
        args: [],
        clearEnv: false,
        detached: false,
        env: { __proto__: null },
        stdin: "null",
        stdout: "piped",
        stderr: "piped",
        ...options,
      };
      assert(
        Array.isArray(options.args),
        `process.Command: args option must be an array, got ${typeof options.args}`,
      );
      assert(
        isBoolean(options.clearEnv),
        `process.Command: clearEnv option must be a boolean, got ${typeof options.clearEnv}`,
      );
      assert(
        isBoolean(options.detached),
        `process.Command: detached option must be a boolean, got ${typeof options.detached}`,
      );
      assert(
        typeof options.env === "object",
        `process.Command: env option must be an object, got ${typeof options.env}`,
      );
      assert(
        options.stdin !== undefined &&
          ["null", "piped", "inherit"].includes(options.stdin),
        `process.Command: stdin option is invalid: ${options.stdin}`,
      );
      assert(
        options.stdout !== undefined &&
          ["null", "piped", "inherit"].includes(options.stdout),
        `process.Command: stdout option is invalid: ${options.stdout}`,
      );
      assert(
        options.stderr !== undefined &&
          ["null", "piped", "inherit"].includes(options.stderr),
        `process.Command: stderr option is invalid: ${options.stderr}`,
      );

      this.#execPath = execPath;
      this.#options = options;
    }

    get execPath(): string {
      return this.#execPath;
    }

    get options(): process.CommandOptions {
      return this.#options;
    }

    /**
     * Spawn a child process.
     *
     * @returns {process.ChildProcess} It returns a child process.
     * @throws Throws {@link !Error} if failed to spawn the child process.
     *
     * @example
     * ```javascript
     * try {
     *   const cmd = new process.Command("ls");
     *   const child = cmd.spawn();
     * } catch (e) {
     *   cmd.echo(`Failed to spawn child process: ${e}`);
     * }
     * ```
     */
    spawn(): process.ChildProcess {
      // @ts-ignore Ignore warning
      const child = __InternalRsvimGlobalObject.process_spawn_child(
        this.#execPath,
        this.#options,
      ) as {
        execPath: string;
        options: process.CommandOptions;
        rid: number;
        stdinRid: number | undefined;
        stdoutRid: number | undefined;
        stderrRid: number | undefined;
      };
      return new process.ChildProcess(
        child.execPath,
        child.options,
        child.rid,
        child.stdinRid,
        child.stdoutRid,
        child.stderrRid,
      );
    }
  }

  /**
   * Child process spawned from command.
   */
  export class ChildProcess {
    /** @hidden */
    #execPath: string;
    /** @hidden */
    #options: process.CommandOptions;
    /** @hidden */
    #rid: number | null | undefined;
    /** @hidden */
    #exitStatus: process.ChildProcessExitStatus | null | undefined;
    /** @hidden */
    #stdinRid: number | null | undefined;
    /** @hidden */
    #stdout: process.ChildProcessReadableStream | null | undefined;
    /** @hidden */
    #stderr: process.ChildProcessReadableStream | null | undefined;

    /** @hideconstructor */
    constructor(
      execPath: string,
      options: process.CommandOptions,
      rid: number,
      stdinRid: number | null | undefined,
      stdoutRid: number | null | undefined,
      stderrRid: number | null | undefined,
    ) {
      this.#execPath = execPath;
      this.#options = options;
      this.#rid = rid;
      this.#exitStatus = null;
      this.#stdinRid = stdinRid;
      this.#stdout =
        stdoutRid != null
          ? new process.ChildProcessReadableStream(stdoutRid)
          : null;
      this.#stderr =
        stderrRid != null
          ? new process.ChildProcessReadableStream(stderrRid)
          : null;
    }

    get execPath(): string {
      return this.#execPath;
    }

    get options(): process.CommandOptions {
      return this.#options;
    }

    get stdout(): process.ChildProcessReadableStream | null | undefined {
      return this.#stdout;
    }

    get stderr(): process.ChildProcessReadableStream | null | undefined {
      return this.#stderr;
    }

    /**
     * Child process is already completed.
     *
     * @example
     * ```javascript
     * const child = new process.Command("ls").spawn();
     * {
     *   await using usedChild = child;
     *   cmd.echo(usedChild.isDisposed); // false
     * }
     * cmd.echo(usedChild.isDisposed); // true
     * ```
     */
    get isDisposed(): boolean {
      return isNil(this.#rid);
    }

    /**
     * Wait for the child process finish with `await using` API instead of `wait`.
     *
     * @returns {void} It returns nothing.
     */
    async [Symbol.asyncDispose](): Promise<void> {
      await this.wait();
    }

    /**
     * Wait for child process complete.
     *
     * @returns {process.ChildProcessExitStatus} It returns a child process exit status.
     * @throws Throws {@link !Error} if the child process is already finished, or failed to wait.
     *
     * @example
     * ```javascript
     * try {
     *   const cmd = new process.Command("ls");
     *   const child = cmd.spawn();
     *   const output = await child.stdout.text();
     *   const exitStatus = await child.wait();
     *   cmd.echo(`"ls" command is completed successfully: ${exitStatus.success}.`);
     * } catch (e) {
     *   cmd.echo(`Failed to run "ls" command.`);
     * }
     * ```
     */
    async wait(): Promise<process.ChildProcessExitStatus> {
      assert(!isNil(this.#rid), "process.ChildProcess is already finished!");
      this.#exitStatus =
        // @ts-ignore Ignore warning
        (await __InternalRsvimGlobalObject.process_wait_child(
          this.#rid,
        )) as process.ChildProcessExitStatus;
      this.#rid = null;
      return this.#exitStatus as process.ChildProcessExitStatus;
    }

    /**
     * Get child process exit status.
     *
     * @returns {process.ChildProcessExitStatus | null | undefined} It returns exit status if the child process is already finished, otherwise it returns `null`.
     *
     * @example
     * ```javascript
     * const child = new process.Command("ls").spawn();
     * await child.wait();
     * const exitStatus = child.exitStatus;
     * ```
     */
    get exitStatus(): process.ChildProcessExitStatus | null | undefined {
      return this.#exitStatus;
    }
  }

  /**
   * Child process readable stream.
   */
  export class ChildProcessReadableStream {
    /** @hidden */
    #rid: number;

    /** @hideconstructor */
    constructor(rid: number) {
      this.#rid = rid;
    }

    /**
     * Read text from the stream.
     *
     * @returns {string} It returns the read text from child process stdio channel.
     * @throws Throws {@link !Error} if failed to read.
     *
     * @example
     * ```javascript
     * try {
     *   const cmd = new process.Command("ls");
     *   const child = cmd.spawn();
     *   const output = await child.stdout.text();
     *   cmd.echo(`"ls" command output:${output}`);
     * } catch (e) {
     *   cmd.echo(`Failed to run "ls" command.`);
     * }
     * ```
     */
    async text(): Promise<string> {
      const payload =
        // @ts-ignore Ignore warning
        await __InternalRsvimGlobalObject.process_read_text_from_child(
          this.#rid,
        );
      return payload;
    }
  }

  /**
   * Command options when creating a child-process command.
   *
   * @see {@link process.Command}
   */
  export type CommandOptions = {
    /**
     * Command arguments.
     *
     * @defaultValue `[]`
     */
    args?: string[];

    /**
     * Current working directory.
     *
     * @defaultValue `undefined`
     */
    cwd?: string;

    /**
     * Whether to clear environment variables when the command creating a child-process.
     *
     * @defaultValue `false`
     */
    clearEnv?: boolean;

    /**
     * Whether to detach spawned child process from current process (editor process).
     * This allows the spawned child process to continue running after current process exits.
     *
     * @defaultValue `false`
     */
    detached?: boolean;

    /**
     * Environment variables to pass to the child-process.
     *
     * @defaultValue `{}`
     */
    env?: Record<string, string | undefined | null>;

    /**
     * How `stdin` of spawned child process should be handled.
     *
     * @defaultValue `null`
     */
    stdin?: "piped" | "inherit" | "null";

    /**
     * How `stdout` of spawned child process should be handled.
     *
     * @defaultValue `piped`
     */
    stdout?: "piped" | "inherit" | "null";

    /**
     * How `stderr` of spawned child process should be handled.
     *
     * @defaultValue `piped`
     */
    stderr?: "piped" | "inherit" | "null";
  };

  /**
   * Child process exit status.
   *
   * @see {@link process.ChildProcess}
   */
  export type ChildProcessExitStatus = {
    /**
     * Exit successfully.
     */
    success: boolean;

    /**
     * Exit code.
     */
    exit?: number;

    /**
     * Terminated by signal.
     */
    signal?: number;
  };

  /**
   * Exit editor.
   *
   * :::tip
   * To ensure file system data safety, editor will wait for all the ongoing file write operations
   * to complete before actually exiting, however any new write requests will be rejected.
   * :::
   *
   * @param {number} exitCode - (Optional) The editor process exit with this exit code, by default with code `0`.
   *
   * @throws Throws {@link !Error} if `exitCode` is neither an integer nor `undefined`.
   *
   * @example
   * ```javascript
   * import process from "rsvim:process";
   *
   * // Exit with default exit code `0`.
   * process.exit();
   *
   * // Exit with error exit code `-1`.
   * process.exit(-1);
   * ```
   */
  export function exit(exitCode?: number): void {
    exitCode = exitCode ?? 0;
    assert(
      isNumber(exitCode),
      `process.exit: code must be a number, got ${typeof exitCode}`,
    );
    assert(
      Number.isInteger(exitCode),
      `process.exit: code must be an integer, got ${typeof exitCode}`,
    );
    // @ts-ignore Ignore warning
    __InternalRsvimGlobalObject.process_exit(exitCode);
  }
}

export default process;
