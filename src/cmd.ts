import { isBoolean } from "es-toolkit/predicate/isBoolean";
import { isFunction } from "es-toolkit/predicate/isFunction";
import { isNil } from "es-toolkit/predicate/isNil";
import { isString } from "es-toolkit/predicate/isString";
import { assert } from "es-toolkit/util";

/**
 * ---
 * title: cmd API
 * sidebar_position: 4
 * ---
 *
 * The `cmd` module for Ex commands.
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
 * The `cmd` module for Ex commands.
 *
 * @example
 * ```javascript
 * import cmd from "rsvim:cmd";
 *
 * cmd.echo("Hello, rsvim!");
 * ```
 *
 * @category Editor APIs
 */
export namespace cmd {
  /**
   * Create a ex command with a callback function.
   *
   * :::warning
   * The builtin command `js` cannot be override.
   * :::
   *
   * @param {string} name - Command name that is going to create. Only letters (`a-z` and `A-Z`), digits (`0-9`), underscore (`_`) and exclamation (`!`) are allowed in a command name. Command name must not begin with a digit.
   * @param {cmd.CommandCallback} callback - Callback function that implements the command. It accepts an `ctx` parameter that contains all the information when user is running it.
   * @param {cmd.CommandAttributes} attributes - (Optional) Attributes that control the command behavior, by default is `{bang:false, nargs:"0"}`.
   * @param {cmd.CommandOptions} options - (Optional) Options that control how the command is created, by default is `{force:true}`.
   * @returns {(cmd.CommandDefinition | undefined)} It returns `undefined` is the command is newly created. Or it returns a command definition that was defined previously.
   *
   * @throws Throws {@link !Error} if any parameters are invalid, or the command name or alias already exists and the `force` option is not set to override it forcibly.
   *
   * @example
   * ```javascript
   * import buf from "rsvim:buf";
   * import fs from "rsvim:fs";
   *
   * async function write(ctx: cmd.CommandContext): void {
   *   try {
   *     const bytes = buf.writeSync(ctx.currentBufferId);
   *
   *     // Call other async APIs
   *     const file = await fs.open("message.txt");
   *     const buffer = new Uint8Array(100);
   *     const read = await file.read(buffer);
   *     const message = new TextDecoder().decode(buffer);
   *
   *     cmd.echo(`Buffer ${bufId} has been saved, ${bytes} bytes written with message: ${message}`);
   *   } catch (e) {
   *     cmd.echo(`Error: failed to save buffer ${bufId}, exception: ${e}`);
   *   }
   * }
   * cmd.create("write", write);
   * ```
   */
  export function create(
    name: string,
    callback: cmd.CommandCallback,
    attributes?: cmd.CommandAttributes,
    options?: cmd.CommandOptions,
  ): cmd.CommandDefinition | undefined {
    assert(
      isString(name),
      `cmd.create: name must be a string, got ${typeof name}`,
    );
    assert(
      /^[A-Za-z_!][A-Za-z0-9_!]*$/.test(name),
      `cmd.create: name is invalid: ${name}`,
    );
    assert(
      isFunction(callback),
      `cmd.create: callback must be a function, got ${typeof callback}`,
    );

    assert(
      attributes === undefined ||
        attributes === null ||
        typeof attributes === "object",
      `cmd.create: attributes must be an object, got ${typeof attributes}`,
    );
    attributes = { bang: false, nargs: "0", ...attributes };
    assert(
      isBoolean(attributes.bang),
      `cmd.create: bang attribute must be a boolean, got ${typeof attributes.bang}`,
    );
    assert(
      attributes.nargs !== undefined &&
        ["0", "1", "?", "+", "*"].includes(attributes.nargs),
      `cmd.create: nargs attribute is invalid: ${attributes.nargs}`,
    );

    assert(
      options === undefined || options === null || typeof options === "object",
      `cmd.create: options must be an object, got ${typeof options}`,
    );
    options = { force: true, ...options };
    assert(
      isBoolean(options.force),
      `cmd.create: force option must be a boolean, got ${typeof options.force}`,
    );
    assert(
      isString(options.alias) || isNil(options.alias),
      `cmd.create: alias option must be a string or nil, got ${typeof options.alias}`,
    );

    // @ts-ignore Ignore warning
    return __InternalRsvimGlobalObject.cmd_create(
      name,
      callback,
      attributes,
      options,
    );
  }

  /**
   * Echo message to the command-line.
   *
   * @param {any} message - It accepts string and other primitive types, except `null` and `undefined`.
   *
   * @throws Throws {@link !Error} if the parameter is `null` or `undefined` or no parameter provided.
   *
   * @example
   * ```javascript
   * cmd.echo("Hello Rsvim!");
   * ```
   */
  export function echo(message: any): void {
    assert(!isNil(message), `cmd.echo: message is required`);

    // @ts-ignore Ignore warning
    __InternalRsvimGlobalObject.cmd_echo(message);
  }

  /**
   * List all registered ex command names.
   *
   * :::warning
   * The builtin `js` command will not be listed here.
   * :::
   *
   * @returns {string[]} Returns all registered ex command names, except the `js` command.
   *
   * @example
   * ```javascript
   * cmd.list().forEach((name) => {
   *   cmd.echo(`Command: ${name}`);
   * });
   * ```
   */
  export function list(): string[] {
    // @ts-ignore Ignore warning
    return __InternalRsvimGlobalObject.cmd_list();
  }

  /**
   * Get ex command definition by name.
   *
   * :::warning
   * The builtin `js` command cannot be get.
   * :::
   *
   * @returns {(cmd.CommandDefinition | undefined)} Returns command definition by its name, except the `js` command.
   *
   * @example
   * ```javascript
   * const def = cmd.get("write");
   * cmd.echo(`Command: ${def.name}`);
   * ```
   */
  export function get(name: string): cmd.CommandDefinition | undefined {
    assert(
      isString(name),
      `cmd.get: name must be a string, got ${typeof name}`,
    );

    // @ts-ignore Ignore warning
    return __InternalRsvimGlobalObject.cmd_get(name);
  }

  /**
   * Remove an ex command by name.
   *
   * :::warning
   * The builtin command `js` cannot be removed.
   * :::
   *
   * @param {string} name - The command name to be removed.
   * @returns {(cmd.CommandDefinition | undefined)} Returns the removed {@link cmd.CommandDefinition}, or `undefined` if no command is been removed.
   *
   * @throws Throws {@link !Error} if name is not a string.
   *
   * @example
   * ```javascript
   * cmd.list().forEach((cmd) => {
   *   // Remove all registered commands.
   *   cmd.remove(cmd.name);
   * });
   * ```
   */
  export function remove(name: string): cmd.CommandDefinition | undefined {
    assert(
      isString(name),
      `cmd.remove: name must be a string, got ${typeof name}`,
    );

    // @ts-ignore Ignore warning
    return __InternalRsvimGlobalObject.cmd_remove(name);
  }

  /**
   * Command attributes.
   *
   * @see {@link cmd.create}
   */
  export type CommandAttributes = {
    /**
     * Whether the command can take a `!` modifier, for example: `:w!`, `:qall!`.
     *
     * @defaultValue `false`
,    */
    bang?: boolean;

    /**
     * Whether The command can take any arguments, and how many it can take:
     *
     * - `0`: No arguments are allowed.
     * - `1`: Exactly 1 argument is required.
     * - `*`: Any number of arguments are allowed, i.e. 0, 1 or more.
     * - `?`: 0 or 1 arguments are allowed.
     * - `+`: At least 1 arguments are required.
     *
     * @defaultValue `0`
,    */
    nargs?: "0" | "1" | "*" | "+" | "?";
  };

  /**
   * Command options when creating a command.
   *
   * @see {@link cmd.create}
   */
  export type CommandOptions = {
    /**
     * Whether force override the command if there's already an existing one.
     *
     * @defaultValue `true`
     */
    force?: boolean;

    /**
     * Command alias, i.e. short name.
     *
     * For example, the `w` is alias for `write`.
     *
     * @defaultValue `undefined`
     */
    alias?: string;
  };

  /**
   * Command callback function, the backend logic that implements a user ex command.
   *
   * Note: The callback function can be either sync or async.
   *
   * It accepts a `ctx` parameter that indicates runtime information when the command is executed.
   *
   * @see {@link cmd.create}
   * @see {@link cmd.CommandContext}
,  */
  export type CommandCallback = (ctx: CommandContext) => void | Promise<void>;

  /**
   * Command definition.
   */
  export type CommandDefinition = {
    name: string;
    callback: CommandCallback;
    attributes: CommandAttributes;
    options: CommandOptions;
  };

  /**
   * Command runtime context.
   *
   * When a command is been execute, runtime information will be passed to the command callback function.
   */
  export type CommandContext = {
    /**
     * Whether the command is executed with a bang "!".
     */
    bang: boolean;

    /**
     * Arguments that are passed to the command when executed.
     */
    args: string[];

    /**
     * Current buffer ID when the command is executed.
     */
    currentBufferId: number;

    /**
     * Current window ID when the command is executed.
     */
    currentWindowId: number;
  };
}

export default cmd;
