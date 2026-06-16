// @ts-nocheck
/**
 * ---
 * title: Rsvim API
 * sidebar_position: 2
 * ---
 *
 * The `Rsvim` global object, it contains two groups:
 * - General APIs.
 * - Editor APIs.
 *
 * @packageDocumentation
 *
 * @categoryDescription Global Object
 * The global object.
 *
 * @categoryDescription Editor APIs
 * These APIs are specific for editor, such as buffers, windows, key mappings, etc.
 *
 * @categoryDescription General APIs
 * These APIs are general for common javascript-based runtime, similar to [Deno APIs](https://docs.deno.com/api/deno/).
 */
/**
 * The `Rsvim.buf` global object for Vim buffers.
 *
 * @example
 * ```javascript
 * // Create a alias to 'Rsvim.buf'.
 * const buf = Rsvim.buf;
 * ```
 *
 * @category Editor APIs
 */
export declare namespace RsvimBuf {
    /**
     * Get current buffer's ID.
     *
     * The "current" buffer is the buffer that the window where your cursor is
     * located is binded to.
     *
     * :::warning
     * When the editor is not initialized, i.e. there's no buffer/window created. It
     * will return `undefined`. Once the editor is initialized, there will always have a
     * valid buffer binded to the "current" window (where your cursor is). It will return
     * the valid buffer ID.
     * :::
     *
     * @returns {(number | undefined)} It returns a valid buffer ID if the editor is initialized.
     * Otherwise it returns `undefined` if the editor is not initialized.
     *
     * @example
     * ```javascript
     * const bufId = Rsvim.buf.current();
     * ```
     */
    function current(): number | undefined;
    /**
     * List all buffers' IDs.
     *
     * :::warning
     * When the editor is not initialized, i.e. there's no buffer/window created. It
     * will return an empty array. Once the editor is initialized, there will have at least 1
     * buffer binded to the "current" window (where your cursor is). It will return all the
     * buffer IDs as an array.
     * :::
     *
     * @returns {number[]} All the buffers' IDs as an array. If there's no
     * buffer (i.e. the editor is not initialized), it returns an empty array.
     *
     * @example
     * ```javascript
     * const bufIds = Rsvim.buf.list();
     * ```
     */
    function list(): number[];
    /**
     * Write (save) buffer's text contents to local filesystem synchronizely.
     *
     * @param {number} bufId - The buffer's ID that you want to write to filesystem.
     *
     * @returns {number} It returns a positive integer to indicate how many bytes
     * have been written to the file, if written successfully.
     *
     * @throws Throws {@link !TypeError} if the parameter is invalid, or {@link !Error} if failed to write buffer to file system.
     *
     * @example
     * ```javascript
     * const bufId = Rsvim.buf.currentBufferId();
     * try {
     *   const bytes = Rsvim.buf.writeSync(bufId);
     *   Rsvim.cmd.echo(`Buffer ${bufId} has been saved, ${bytes} bytes written`);
     * } catch (e) {
     *   Rsvim.cmd.echo(`Error: failed to save buffer ${bufId}, exception: ${e}`);
     * }
     * ```
     */
    function writeSync(bufId: number): number;
}
/**
 * The `Rsvim.cmd` global object for Ex commands.
 *
 * @example
 * ```javascript
 * // Create a alias to 'Rsvim.cmd'.
 * const cmd = Rsvim.cmd;
 * ```
 *
 * @category Editor APIs
 */
export declare namespace RsvimCmd {
    /**
     * Create a ex command with a callback function.
     *
     * :::warning
     * The builtin command `js` cannot be override.
     * :::
     *
     * @param {string} name - Command name that is going to create. Only letters (`a-z` and `A-Z`), digits (`0-9`), underscore (`_`) and exclamation (`!`) are allowed in a command name. Command name must not begin with a digit.
     * @param {RsvimCmd.CommandCallback} callback - Async callback function that implements the command. It accepts an `ctx` parameter that contains all the information when user is running it. See {@link RsvimCmd.CommandCallback}.
     * @param {RsvimCmd.CommandAttributes} attributes - (Optional) Attributes that control the command behavior, by default is `{bang:false, nargs:"0"}`, see {@link RsvimCmd.CommandAttributes}.
     * @param {RsvimCmd.CommandOptions} options - (Optional) Options that control how the command is created, by default is `{force:true}`, see {@link RsvimCmd.CommandOptions}.
     * @returns {(RsvimCmd.CommandDefinition | undefined)} It returns `undefined` is the command is newly created. Or it returns a command definition that was defined previously.
     *
     * @throws Throws {@link !TypeError} if any parameters are invalid. Or throws {@link Error} if command name or alias already exists, but `force` option is not set to override existing command forcibly.
     *
     * @example
     * ```javascript
     * async function write(ctx: RsvimCmd.CommandContext): void {
     *   try {
     *     const bytes = Rsvim.buf.writeSync(ctx.currentBufferId);
     *
     *     // Call other async APIs
     *     const file = await Rsvim.fs.open("message.txt");
     *     const buffer = new Uint8Array(100);
     *     const read = await file.read(buffer);
     *     const message = new TextDecoder().decode(buffer);
     *
     *     Rsvim.cmd.echo(`Buffer ${bufId} has been saved, ${bytes} bytes written with message: ${message}`);
     *   } catch (e) {
     *     Rsvim.cmd.echo(`Error: failed to save buffer ${bufId}, exception: ${e}`);
     *   }
     * }
     * Rsvim.cmd.create("write", write);
     * ```
     */
    function create(name: string, callback: RsvimCmd.CommandCallback, attributes?: RsvimCmd.CommandAttributes, options?: RsvimCmd.CommandOptions): RsvimCmd.CommandDefinition | undefined;
    /**
     * Echo message to the command-line.
     *
     * @param {any} message - It accepts string and other primitive types, except `null` and `undefined`.
     *
     * @throws Throws {@link !TypeError} if the parameter is `null` or `undefined` or no parameter provided.
     *
     * @example
     * ```javascript
     * Rsvim.cmd.echo("Hello Rsvim!");
     * ```
     */
    function echo(message: any): void;
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
     * Rsvim.cmd.list().forEach((name) => {
     *   Rsvim.cmd.echo(`Command: ${name}`);
     * });
     * ```
     */
    function list(): string[];
    /**
     * Get ex command definition by name.
     *
     * :::warning
     * The builtin `js` command cannot be get.
     * :::
     *
     * @returns {(RsvimCmd.CommandDefinition | undefined)} Returns command definition by its name, except the `js` command.
     *
     * @example
     * ```javascript
     * const def = Rsvim.cmd.get("write");
     * Rsvim.cmd.echo(`Command: ${def.name}`);
     * ```
     */
    function get(name: string): RsvimCmd.CommandDefinition | undefined;
    /**
     * Remove an ex command by name.
     *
     * :::warning
     * The builtin command `js` cannot be removed.
     * :::
     *
     * @param {string} name - The command name to be removed.
     * @returns {(RsvimCmd.CommandDefinition | undefined)} Returns the removed {@link RsvimCmd.CommandDefinition}, or `undefined` if no command is been removed.
     *
     * @throws Throws {@link !TypeError} if name is not a string.
     *
     * @example
     * ```javascript
     * Rsvim.cmd.list().forEach((cmd) => {
     *   // Remove all registered commands.
     *   Rsvim.cmd.remove(cmd.name);
     * });
     * ```
     */
    function remove(name: string): RsvimCmd.CommandDefinition | undefined;
    /**
     * Command attributes.
     *
     * @see {@link RsvimCmd.create}
     */
    type CommandAttributes = {
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
     * @see {@link RsvimCmd.create}
     */
    type CommandOptions = {
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
     * Command callback function, this is the backend logic that implements a user ex command.
     *
     * It accepts a `ctx` parameter that indicates runtime information when the command is executed.
     *
     * @see {@link RsvimCmd.create}
     * @see {@link CommandContext}
  ,  */
    type CommandCallback = (ctx: CommandContext) => Promise<void>;
    /**
     * Command definition.
     */
    type CommandDefinition = {
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
    type CommandContext = {
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
/**
 * The `Rsvim.fs` global object for file system and file IO.
 *
 * @example
 * ```javascript
 * // Create a alias to 'Rsvim.fs'.
 * const fs = Rsvim.fs;
 * ```
 *
 * @category General APIs
 */
export declare namespace RsvimFs {
    /**
     * Open a file and resolve to an instance of {@link RsvimFs.File}. The file does not need to previously exist if using the `create` or `createNew` open options.
     * The caller have to close the file to prevent resource leaking, see {@link RsvimFs.File.close}.
     *
     * @param {string} path - File path.
     * @param {RsvimFs.OpenOptions} options - (Optional) Open options, by default is `{read: true}`. See {@link RsvimFs.OpenOptions}.
     * @returns {Promise<RsvimFs.File>} It resolves to an instance of {@link RsvimFs.File}.
     *
     * @throws Throws {@link !TypeError} if any parameters are invalid. Or throws {@link Error} if failed to open/create the file.
     *
     * @example
     * ```javascript
     * const file = await Rsvim.fs.open("README.md");
     * ```
     */
    function open(path: string, options?: RsvimFs.OpenOptions): Promise<RsvimFs.File>;
    /**
     * Sync version of {@link open}.
     *
     * @example
     * ```javascript
     * const file = Rsvim.fs.openSync("README.md");
     * ```
     */
    function openSync(path: string, options?: RsvimFs.OpenOptions): RsvimFs.File;
    /**
     * Read a file in binary mode, i.e. into an array of bytes buffer, without open/close a file descriptor/handle.
     *
     * @param {string} path - File path to read.
     * @returns {Promise<Uint8Array>} It resolves to {@link !Uint8Array} that contains all the file contents as bytes array.
     *
     * @throws Throws {@link !TypeError} if the file name is invalid. Or throws {@link Error} if failed to read the file.
     *
     * @example
     * ```javascript
     * const buffer = await Rsvim.fs.readFile("README.md");
     * ```
     */
    function readFile(path: string): Promise<Uint8Array>;
    /**
     * Sync version of {@link readFile}.
     *
     * @example
     * ```javascript
     * const buffer = Rsvim.fs.readFileSync("README.md");
     * ```
     */
    function readFileSync(path: string): Uint8Array;
    /**
     * Read a file in text mode, i.e. into a string, without open/close a file descriptor/handle.
     *
     * @param {string} path - File path to read.
     * @returns {Promise<string>} It resolves to text string that contains all the file contents.
     *
     * @throws Throws {@link !TypeError} if the file name is invalid. Or throws {@link Error} if failed to read the file.
     *
     * @example
     * ```javascript
     * const payload = await Rsvim.fs.readTextFile("README.md");
     * ```
     */
    function readTextFile(path: string): Promise<string>;
    /**
     * Sync version of {@link readTextFile}.
     *
     * @example
     * ```javascript
     * const payload = Rsvim.fs.readTextFileSync("README.md");
     * ```
     */
    function readTextFileSync(path: string): string;
    /**
     * Get the status of a file by path.
     *
     * :::note
     * This api doesn't follow symbolic link.
     * :::
     *
     * @see {@link RsvimFs.stat}
     *
     * @param {string} path - File path.
     * @returns {Promise<RsvimFs.FileInfo>} It resolves to the file status.
     *
     * @throws Throws {@link !TypeError} if the file name is invalid. Or throws {@link Error} if failed to get file status.
     *
     * @example
     * ```javascript
     * const fstat = await Rsvim.fs.lstat("README.md");
     * ```
     */
    function lstat(path: string): Promise<RsvimFs.FileInfo>;
    /**
     * Sync version of {@link lstat}.
     *
     * @example
     * ```javascript
     * const fstat = Rsvim.fs.lstatSync("README.md");
     * ```
     */
    function lstatSync(path: string): RsvimFs.FileInfo;
    /**
     * Get the status of a file by path.
     *
     * :::note
     * This api follows symbolic link.
     * :::
     *
     * @see {@link RsvimFs.lstat}
     *
     * @param {string} path - File path.
     * @returns {Promise<RsvimFs.FileInfo>} It resolves to the file status.
     *
     * @throws Throws {@link !TypeError} if the file name is invalid. Or throws {@link Error} if failed to get file status.
     *
     * @example
     * ```javascript
     * const fstat = await Rsvim.fs.stat("README.md");
     * ```
     */
    function stat(path: string): Promise<RsvimFs.FileInfo>;
    /**
     * Sync version of {@link stat}.
     *
     * @example
     * ```javascript
     * const fstat = Rsvim.fs.statSync("README.md");
     * ```
     */
    function statSync(path: string): RsvimFs.FileInfo;
    /**
     * Create symbolic link from a file path.
     *
     * On Windows platforms, you should specify the 3rd parameter "options" with below 3 options:
     *
     * - `"file"` or `"dir"`: Pointing to [CreateSymbolicLinkW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createsymboliclinkw) function.
     * - `"junction"`: NTFS v5+ features roughly equivalent to Unix directory symbolic links.
     *
     * @param {string} oldpath - Original file path.
     * @param {string} newpath - New symbolic link that pointing to the original file.
     * @param {RsvimFs.SymlinkOptions} options - (Optional) Only used on Windows platforms, will be ignored on Unix platforms. By default is `"junction"`.
     * @returns {Promise<void>} It resolves to nothing.
     *
     * @throws Throws {@link !TypeError} if any parameter is invalid. Or throws {@link Error} if failed to create symbolic link from the file.
     *
     * @example
     * ```javascript
     * try {
     *   // Linux
     *   await Rsvim.fs.symlink("README.md", "linked-README.md");
     *   // Windows
     *   await Rsvim.fs.symlink("README.md", "linked-README.md", "junction");
     *   Rsvim.cmd.echo(`Created symbolic link "linked-README.md" pointing to "README.md"`);
     * } catch (e) {
     *   Rsvim.cmd.echo(`Failed to create symbolic link pointing to "README.md": ${e}`);
     * }
     * ```
     */
    function symlink(oldpath: string, newpath: string, options?: RsvimFs.SymlinkOptions): Promise<void>;
    /**
     * Sync version of {@link symlink}.
     *
     * @example
     * ```javascript
     * try {
     *   // Linux
     *   Rsvim.fs.symlinkSync("README.md", "linked-README.md");
     *   // Windows
     *   Rsvim.fs.symlinkSync("README.md", "linked-README.md", "junction");
     *   Rsvim.cmd.echo(`Created symbolic link "linked-README.md" pointing to "README.md"`);
     * } catch (e) {
     *   Rsvim.cmd.echo(`Failed to create symbolic link pointing to "README.md": ${e}`);
     * }
     * ```
     */
    function symlinkSync(oldpath: string, newpath: string, options?: RsvimFs.SymlinkOptions): void;
    /**
     * Create hard link from a file path.
     *
     * @param {string} oldpath - Original file path.
     * @param {string} newpath - New symbolic link that pointing to the original file.
     * @returns {Promise<void>} It resolves to nothing.
     *
     * @throws Throws {@link !TypeError} if any parameter is invalid. Or throws {@link Error} if failed to create hard link from the file.
     *
     * @example
     * ```javascript
     * try {
     *   await Rsvim.fs.link("README.md", "linked-README.md");
     *   Rsvim.cmd.echo(`Created hard link "linked-README.md" pointing to "README.md"`);
     * } catch (e) {
     *   Rsvim.cmd.echo(`Failed to create hard link pointing to "README.md": ${e}`);
     * }
     * ```
     */
    function link(oldpath: string, newpath: string): Promise<void>;
    /**
     * Sync version of {@link link}.
     *
     * @example
     * ```javascript
     * try {
     *   Rsvim.fs.linkSync("README.md", "linked-README.md");
     *   Rsvim.cmd.echo(`Created hard link "linked-README.md" pointing to "README.md"`);
     * } catch (e) {
     *   Rsvim.cmd.echo(`Failed to create hard link pointing to "README.md": ${e}`);
     * }
     * ```
     */
    function linkSync(oldpath: string, newpath: string): void;
    /**
     * Make a directory.
     *
     * @param {string} path - Directory path.
     * @param {MkdirOptions} newpath - New symbolic mkdir that pointing to the original file.
     * @returns {Promise<void>} It resolves to nothing.
     *
     * @throws Throws {@mkdir !TypeError} if any parameter is invalid. Or throws {@mkdir Error} if failed to create hard mkdir from the file.
     *
     * @example
     * ```javascript
     * try {
     *   await Rsvim.fs.mkdir(".rsvim");
     *   Rsvim.cmd.echo(`Created directory ".rsvim"`);
     * } catch (e) {
     *   Rsvim.cmd.echo(`Failed to create directory ".rsvim": ${e}`);
     * }
     * ```
     */
    function mkdir(path: string, options?: RsvimFs.MkdirOptions): Promise<void>;
    /**
     * Sync version of {@link mkdir}.
     *
     * @example
     * ```javascript
     * try {
     *   Rsvim.fs.mkdirSync(".rsvim");
     *   Rsvim.cmd.echo(`Created directory ".rsvim"`);
     * } catch (e) {
     *   Rsvim.cmd.echo(`Failed to create directory ".rsvim": ${e}`);
     * }
     * ```
     */
    function mkdirSync(path: string, options?: RsvimFs.MkdirOptions): void;
    /**
     * Open options.
     *
     * :::tip
     * It is same with [std::fs::OpenOptions](https://doc.rust-lang.org/std/fs/struct.OpenOptions.html) in rust std library.
     * :::
     *
     * @see {@link RsvimFs.open}
     */
    type OpenOptions = {
        /**
         * Set the file for append mode.
         *
         * @defaultValue `false`
    ,    */
        append?: boolean;
        /**
         * Create a new file or open it if it already exists.
         *
         * In order for the file to be created, `write` or `append` access must be used.
         *
         * @defaultValue `false`
    ,    */
        create?: boolean;
        /**
         * Create a new file, failing if it already exists.
         *
         * If this option is set, `create` and `truncate` options are ignored.
         *
         * @defaultValue `false`
    ,    */
        createNew?: boolean;
        /**
         * Set the file for read access.
         *
         * @defaultValue `false`
    ,    */
        read?: boolean;
        /**
         * Open the file and truncate the file to `0` length if it already exists.
         *
         * @defaultValue `false`
    ,    */
        truncate?: boolean;
        /**
         * Set the file for write access. If the file already exists, any "write" calls on it will
         * overwrite the contents, without truncating it.
         *
         * @defaultValue `false`
    ,    */
        write?: boolean;
    };
    /**
     * The File object that access to an open file on filesystem.
     *
     * @see {@link RsvimFs.open}
     */
    class File {
        #private;
        /** @hidden */
        constructor(rid: number);
        /**
         * Close the file.
         *
         * @throws Throws {@link !Error} if the file is already been closed.
         *
         * @example
         * ```javascript
         * const file = await Rsvim.fs.open("README.md");
         * // do work with the `file` object
         * file.close();
         *
         * // Or
         *
         * using file = await Rsvim.fs.open("README.md");
         * // do work with the `file` object
         * ```
         */
        close(): void;
        /**
         * Close the file with `using` API instead of `close`.
         *
         * @example
         * ```javascript
         * using file = await Rsvim.fs.open("README.md");
         * // do work with the `file` object
         * ```
         *
         * @see {@link close}
         */
        [Symbol.dispose](): void;
        /**
         * File is already been closed.
         *
         * @example
         * ```javascript
         * const file = await Rsvim.fs.open("README.md");
         * if (!file.isDisposed()) {
         *   file.close();
         * }
         * ```
         */
        get isDisposed(): boolean;
        /**
         * Read a file into a buffer.
         *
         * :::warning
         * It is not guaranteed that the full buffer will be read in a single call.
         * :::
         *
         * @param {Uint8Array} buf - Read bytes into buffer.
         * @returns {Promise<number>} It resolves to either the number of bytes read during the operation or `0`(EOF) if there was no more to read.
         *
         * @throws Throws {@link !TypeError} if buf is not a Uint8Array.
         *
         * @example
         * ```javascript
         * using file = await Rsvim.fs.open("README.md");
         * const buf = new Uint8Array(100);
         * const n = await file.read(buf); // read 11 bytes
         * const text = new TextDecoder().decode(buf); // decode into UTF-8 string "hello world"
         * ```
         */
        read(buf: Uint8Array): Promise<number>;
        /**
         * Sync version of {@link read}.
         *
         * @example
         * ```javascript
         * using file = await Rsvim.fs.open("README.md");
         * const buf = new Uint8Array(100);
         * const n = file.readSync(buf); // read 11 bytes
         * const text = new TextDecoder().decode(buf); // decode into UTF-8 string "hello world"
         * ```
         */
        readSync(buf: Uint8Array): number;
        /**
         * Write a buffer into a file.
         *
         * :::warning
         * It is not guaranteed that the full buffer will be written in a single call.
         * :::
         *
         * @param {Uint8Array} buf - Read bytes into buffer.
         * @returns {Promise<number>} It resolves to either the number of bytes written during the operation or `0` if there was nothing to write.
         *
         * @throws Throws {@link !TypeError} if buf is not a Uint8Array.
         *
         * @example
         * ```javascript
         * using file = await Rsvim.fs.open("README.md", {write:true,create:true});
         * const buf = new TextEncoder().encode("hello world");
         * const n = await file.write(buf); // write 11 bytes
         * ```
         */
        write(buf: Uint8Array): Promise<number>;
        /**
         * Sync version of {@link write}.
         *
         * @example
         * ```javascript
         * using file = await Rsvim.fs.open("README.md", {write:true,create:true});
         * const buf = new TextEncoder().encode("hello world");
         * const n = file.writeSync(buf); // write 11 bytes
         * ```
         */
        writeSync(buf: Uint8Array): number;
    }
    /**
     * File information, it contains 3 groups of properties:
     * - Common properties that are available for all platforms.
     * - Windows platforms only properties
     * - Unix platforms only properties
     *
     * @categoryDescription Common
     * @categoryDescription Windows Only
     * @categoryDescription Unix Only
     */
    type FileInfo = {
        /**
         * Last access time of the file.
         *
         * The value can be `undefined` if failed to get this information.
         *
         * @category Common
         */
        accessed?: Date;
        /**
         * Creation time of the file.
         *
         * The value can be `undefined` if failed to get this information.
         *
         * @category Common
         */
        created?: Date;
        /**
         * Last modification time of the file.
         *
         * The value can be `undefined` if failed to get this information.
         *
         * @category Common
         */
        modified?: Date;
        /**
         * Whether the file is a directory. This value is mutually exclusive to `isFile`.
         *
         * @category Common
         */
        isDir: boolean;
        /**
         * Whether the file is a regular file. This value is mutually exclusive to `isDir`.
         *
         * @category Common
         */
        isFile: boolean;
        /**
         * Whether the file is a symbolic link.
         *
         * @category Common
         */
        isSymlink: boolean;
        /**
         * The size of the file in bytes.
         *
         * @category Common
         */
        len: number;
        /**
         * Whether the file is read-only (unwritable).
         *
         * @category Common
         */
        readOnly: boolean;
        /**
         * The `dwFileAttributes` value of the file.
         *
         * For possible values and their descriptions, see [File Attribute Constants](https://learn.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants) in the Windows Dev Center.
         *
         * @category Windows Only
         */
        fileAttributes?: number;
        /**
         * The `ftCreationTime` value of the file.
         *
         * The value is equivalent to a [FILETIME](https://learn.microsoft.com/en-us/windows/win32/api/minwinbase/ns-minwinbase-filetime) struct, which represents the number of 100-nanosecond intervals since January 1, 1601 (UTC).
         *
         * @category Windows Only
         */
        creationTime?: number;
        /**
         * The `ftLastAccessTime` value of the file.
         *
         * The value is equivalent to a [FILETIME](https://learn.microsoft.com/en-us/windows/win32/api/minwinbase/ns-minwinbase-filetime) struct, which represents the number of 100-nanosecond intervals since January 1, 1601 (UTC).
         *
         * @category Windows Only
         */
        lastAccessTime?: number;
        /**
         * The `ftLastWriteTime` value of the file.
         *
         * The value is equivalent to a [FILETIME](https://learn.microsoft.com/en-us/windows/win32/api/minwinbase/ns-minwinbase-filetime) struct, which represents the number of 100-nanosecond intervals since January 1, 1601 (UTC).
         *
         * @category Windows Only
         */
        lastWriteTime?: number;
        /**
         * The `nFileSize` value of the file.
         *
         * The value doesn't have meaning for directories.
         *
         * @category Windows Only
         */
        fileSize?: number;
        /**
         * The ID of the device containing the file.
         *
         * @category Unix Only
         */
        dev?: number;
        /**
         * The inode number.
         *
         * @category Unix Only
         */
        ino?: number;
        /**
         * The rights applied to the file.
         *
         * @category Unix Only
         */
        mode?: number;
        /**
         * The number of hard links pointing to the file.
         *
         * @category Unix Only
         */
        nlink?: number;
        /**
         * The user ID of the owner of the file.
         *
         * @category Unix Only
         */
        uid?: number;
        /**
         * The group ID of the owner of the file.
         *
         * @category Unix Only
         */
        gid?: number;
        /**
         * The device ID of the file (if it is a special one).
         *
         * @category Unix Only
         */
        rdev?: number;
        /**
         * The total size of the file in bytes.
         *
         * @category Unix Only
         */
        size?: number;
        /**
         * Last access time of the file, in seconds since Unix Epoch.
         *
         * @category Unix Only
         */
        atime?: number;
        /**
         * Last access time of the file, in nanoseconds since `atime`.
         *
         * @category Unix Only
         */
        atimeNsec?: number;
        /**
         * Last modification time of the file, in seconds since Unix Epoch.
         *
         * @category Unix Only
         */
        mtime?: number;
        /**
         * Last modification time of the file, in nanoseconds since `mtime`.
         *
         * @category Unix Only
         */
        mtimeNsec?: number;
        /**
         * Last status change time of the file, in seconds since Unix Epoch.
         *
         * @category Unix Only
         */
        ctime?: number;
        /**
         * Last status change time of the file, in nanoseconds since `ctime`.
         *
         * @category Unix Only
         */
        ctimeNsec?: number;
        /**
         * The block size for filesystem IO.
         *
         * @category Unix Only
         */
        blksize?: number;
        /**
         * The number of blocks allocated to the file, in 512-byte units.
         *
         * Please note that this may be smaller than `st_size / 512` when the file has holes.
         *
         * @category Unix Only
         */
        blocks?: number;
    };
    /**
     * @see {@link RsvimFs.symlink}
     * @inline
     * */
    type SymlinkOptions = "file" | "dir" | "junction";
    /**
     * Mkdir options.
     */
    type MkdirOptions = {
        /**
         * Whether make directory recursively, by default is `false`.
         *
         * @defaultValue `false`
         */
        recursive?: boolean;
        /**
         * Unix permission when creating the directory, by default is `0o777`.
         *
         * :::note
         * This option only works on Unix platforms and is ignored on Windows platforms.
         * :::
         *
         * @defaultValue `0o777`
         */
        mode?: number;
    };
}
export declare namespace RsvimOpt {
    /**
     * @see {@link RsvimOpt.fileEncoding}
     * @inline
     * */
    type FileEncodingOption = "utf-8";
    /**
     * @see {@link RsvimOpt.fileFormat}
     * @inline
     */
    type FileFormatOption = "dos" | "unix" | "mac";
}
/**
 * The `Rsvim.opt` global object for global editor options. These options will change the editor's behavior
 * and suit user's personal habits.
 *
 * There are 3 kinds of editor option:
 * - Global options: Options that are global that you use one value for all Rsvim component instances such
 *   as buffer, window, statusline, etc. When you change the option, it will take effect immediately and
 *   affect all existing instances.
 * - Local options: Options that only apply to one component instance, each instance has its own copy of
 *   this option, thus each can have its own value. This allow you to set an option in one instance, without
 *   modifying other instances.
 * - Global local options: Options that are global, and will be copy to a newly created Rsvim component
 *   instance. A global-local-option always has its corresponding local-option. When you change the option,
 *   it only will apply to the newly created instances, but cannot modify existing instances.
 *
 * @example
 * ```javascript
 * // Create a alias to 'Rsvim.opt'.
 * const opt = Rsvim.opt;
 * ```
 *
 * @category Editor APIs
 * @hideconstructor
 */
export declare class RsvimOpt {
    /**
     * Get the _expand-tab_ option. Local to buffer.
     *
     * When in insert mode, inserts [spaces](https://en.wikipedia.org/wiki/Whitespace_character) (ASCII `32`)
     * instead of a [horizontal tab](https://en.wikipedia.org/wiki/Tab_key) (ASCII `9`).
     *
     * See {@link shiftWidth} to get the number of spaces when inserting.
     *
     * @returns {boolean}
     *
     * @defaultValue `false`
     *
     * @example
     * ```javascript
     * // Get the 'expand-tab' option.
     * const value = Rsvim.opt.expandTab;
     * ```
     */
    get expandTab(): boolean;
    /**
     * Set the _expand-tab_ option.
     *
     * @param {boolean} value - The _expand-tab_ option.
     * @throws Throws {@link !TypeError} if value is not a boolean.
     *
     * @example
     * ```javascript
     * // Set the 'expand-tab' option.
     * Rsvim.opt.expandTab = true;
     * ```
     */
    set expandTab(value: boolean);
    /**
     * Get the _file-encoding_ option. Local to buffer.
     *
     * Sets the [character encoding](https://en.wikipedia.org/wiki/Character_encoding) for the file of this buffer.
     * This will determine which character encoding is used when RSVIM read/write a file from file system.
     *
     * :::warning
     * For now, only **utf-8** encoding is supported.
     * :::
     *
     * @returns {RsvimOpt.FileEncodingOption}
     *
     * @defaultValue `"utf-8"`
     *
     * @example
     * ```javascript
     * // Get the 'file-encoding' option.
     * const value = Rsvim.opt.fileEncoding;
     * ```
     */
    get fileEncoding(): RsvimOpt.FileEncodingOption;
    /**
     * Set the _file-encoding_ option.
     *
     * @param {RsvimOpt.FileEncodingOption} value - The _file-encoding_ option.
     * @throws Throws {@link !RangeError} if value is an invalid option.
     *
     * @example
     * ```javascript
     * // Set the 'file-encoding' option.
     * Rsvim.opt.fileEncoding = "utf-8";
     * ```
     */
    set fileEncoding(value: RsvimOpt.FileEncodingOption);
    /**
     * Get the _file-format_ option. Local to buffer.
     *
     * Sets the [line end](https://en.wikipedia.org/wiki/Newline) for the buffer. There are 3 kinds of line end:
     * - `CRLF`: used by [Windows](https://www.microsoft.com/windows).
     * - `LF`: used by [Linux](https://en.wikipedia.org/wiki/Linux) and [Unix](https://en.wikipedia.org/wiki/Unix) (include [MacOS](https://www.apple.com/macos/)).
     * - `CR`: used by [classic MacOS](https://en.wikipedia.org/wiki/Classic_Mac_OS).
     *
     * :::warning
     * Today's Mac also uses `LF` as line end, you should never use `CR` any more.
     * :::
     *
     * For this option, it has below choices:
     * - `"dos"`: equivalent to `CRLF` line end.
     * - `"unix"`: equivalent to `LF` line end.
     * - `"mac"`: equivalent to `CR` line end.
     *
     * @returns {RsvimOpt.FileFormatOption}
     *
     * @defaultValue `"dos"` for Windows/MS-DOS, `"unix"` for Linux/Unix/MacOS.
     *
     * @example
     * ```javascript
     * // Get the 'file-format' option.
     * const value = Rsvim.opt.fileFormat;
     * ```
     */
    get fileFormat(): RsvimOpt.FileFormatOption;
    /**
     * Set the _file-format_ option.
     *
     * @param {RsvimOpt.FileFormatOption} value - The _file-format_ option.
     * @throws Throws {@link !RangeError} if value is an invalid option.
     *
     * @example
     * ```javascript
     * // Set the 'file-format' option.
     * Rsvim.opt.fileFormat = "unix";
     * ```
     */
    set fileFormat(value: RsvimOpt.FileFormatOption);
    /**
     * Get the _fix-end-of-line_ option. Local to buffer.
     *
     * WHen writing a file and this options is enabled, `EOL` (`LF`, `CR`, `CRLF`) at the end of file will be restored if missing. Disable this option if you want to preserve the situation from the original file.
     *
     * @see {@link fileFormat}
     *
     * @returns {boolean}
     *
     * @defaultValue `true`
     *
     * @example
     * ```javascript
     * // Get the 'fix-end-of-line' option.
     * const value = Rsvim.opt.fixEndOfLine;
     * ```
     */
    get fixEndOfLine(): boolean;
    /**
     * Set the _fix-end-of-line_ option.
     *
     * @param {boolean} value - The _fix-end-of-line_ option.
     * @throws Throws {@link !RangeError} if value is not a boolean.
     *
     * @example
     * ```javascript
     * // Set the 'fix-end-of-line' option.
     * Rsvim.opt.fixEndOfLine = false;
     * ```
     */
    set fixEndOfLine(value: boolean);
    /**
     * Get the _line-break_ option. This options is also known as
     * [word wrap](https://en.wikipedia.org/wiki/Line_wrap_and_word_wrap). Local to window.
     *
     * If `true`, Vim will wrap long lines by a word boundary rather than at the last character that fits on the screen.
     * It only affects the way the file is displayed, not its contents.
     *
     * This option is not used when the {@link wrap} option is `false`.
     *
     * @returns {boolean}
     *
     * @defaultValue `false`
     *
     * @example
     * ```javascript
     * // Get the 'lineBreak' option.
     * const value = Rsvim.opt.lineBreak;
     * ```
     */
    get lineBreak(): boolean;
    /**
     * Set the _line-break_ option.
     *
     * @param {boolean} value - The _line-break_ option.
     * @throws Throws {@link !TypeError} if value is not a boolean.
     *
     * @example
     * ```javascript
     * // Set the 'lineBreak' option.
     * Rsvim.opt.lineBreak = true;
     * ```
     */
    set lineBreak(value: boolean);
    /**
     * Get the _shift-width_ option. Local to buffer.
     *
     * When {@link expandTab} is `true`, the number of spaces that is used when inserts a
     * [horizontal tab](https://en.wikipedia.org/wiki/Tab_key) (ASCII `9`).
     *
     * When {@link expandTab} is `false`, this option is not been used.
     *
     *
     * @returns {number}
     *
     * @defaultValue `8`
     *
     * @example
     * ```javascript
     * // Get the 'shift-width' option.
     * const value = Rsvim.opt.shiftWidth;
     * ```
     */
    get shiftWidth(): number;
    /**
     * Set the _shift-width_ option. It only accepts an integer between `[1,255]`, if the value is out of range, it will be bound to this range.
     *
     *
     * @param {number} value - The _shift-width_ option.
     * @throws Throws {@link !TypeError} if value is not an integer.
     *
     * @example
     * ```javascript
     * // Set the 'shift-width' option.
     * Rsvim.opt.shiftWidth = 4;
     * ```
     */
    set shiftWidth(value: number);
    /**
     * Get the _syntax-parser-lib-path_ option. Global option.
     *
     * By default the syntax parser libs are stored in `${RSVIM_CONFIG_HOME}/.tree-sitter-parsers` folder. `${RSVIM_CONFIG_HOME}` is the configuration home for rsvim.
     *
     * @see [Rsvim Configuration](https://rsvim.github.io/docs/manual/configuration)
     *
     * @returns {string}
     *
     * @defaultValue `${RSVIM_CONFIG_HOME}/.tree-sitter-parsers`
     *
     * @example
     * ```javascript
     * // Get the 'syntax-parser-lib-path' option.
     * const value = Rsvim.opt.syntaxParserLibPath;
     * ```
     */
    get syntaxParserLibPath(): string;
    /**
     * Set the _syntax-parser-lib-path_ option. It only accepts a string which represents the file path on your local machine, which is used to save all compiled tree-sitter parser dynamic libraries.
     *
     * @param {string} value - The _syntax-parser-lib-path_ option.
     * @throws Throws {@link !TypeError} if value is not an string.
     *
     * @example
     * ```javascript
     * // Set the 'syntax-parser-lib-path' option.
     * Rsvim.opt.syntaxParserLibPath = ".";
     * ```
     */
    set syntaxParserLibPath(value: string);
    /**
     * Get the _tab-stop_ option. This option is also known as
     * [tab-size](https://developer.mozilla.org/en-US/docs/Web/CSS/tab-size).
     * Local to buffer.
     *
     * This option changes how text is displayed.
     *
     * Defines how many columns (on the terminal) used to display the
     * [horizontal tab](https://en.wikipedia.org/wiki/Tab_key) (ASCII `9`). This value should be between `[1,255]`.
     *
     *
     * @returns {number}
     *
     * @defaultValue `8`
     *
     * @example
     * ```javascript
     * // Get the 'tab-stop' option.
     * const value = Rsvim.opt.tabStop;
     * ```
     */
    get tabStop(): number;
    /**
     * Set the _tab-stop_ option. It only accepts an integer between `[1,255]`, if the value is out of range, it will be bound to this range.
     *
     * @param {number} value - The _tab-stop_ option.
     * @throws Throws {@link !TypeError} if value is not an integer.
     *
     * @example
     * ```javascript
     * // Set the 'tab-stop' option.
     * Rsvim.opt.tabStop = 4;
     * ```
     */
    set tabStop(value: number);
    /**
     * Get the _wrap_ option. This option is also known as
     * [line wrap](https://en.wikipedia.org/wiki/Line_wrap_and_word_wrap). Local to window.
     *
     * This option changes how text is displayed.
     *
     * When `true`, lines longer than the width of the window will wrap and
     * displaying continues on the next line. When `false` lines will not wrap
     * and only part of long lines will be displayed. When the cursor is
     * moved to a part that is not shown, the screen will scroll horizontally.
     *
     * The line will be broken in the middle of a word if necessary. See {@link lineBreak}
     * to get the break at a word boundary.
     *
     * @returns {boolean}
     *
     * @defaultValue `true`
     *
     * @example
     * ```javascript
     * // Get the 'wrap' option.
     * const value = Rsvim.opt.wrap;
     * ```
     */
    get wrap(): boolean;
    /**
     * Set the _wrap_ option.
     *
     * @param {boolean} value - The _wrap_ option.
     * @throws Throws {@link !TypeError} if value is not a boolean.
     *
     * @example
     * ```javascript
     * // Set the 'wrap' option.
     * Rsvim.opt.wrap = true;
     * ```
     */
    set wrap(value: boolean);
}
/**
 * The `Rsvim.proc` global object for child process.
 *
 * @example
 * ```javascript
 * // Create a alias to 'Rsvim.proc'.
 * const proc = Rsvim.proc;
 * ```
 *
 * @category General APIs
 */
export declare namespace RsvimProc {
    /**
     * The command that create a child process.
     */
    class Command {
        #private;
        constructor(execPath: string, options?: RsvimProc.CommandOptions);
        get execPath(): string;
        get options(): RsvimProc.CommandOptions;
        /**
         * Spawn a child process.
         *
         * @returns {RsvimProc.ChildProcess} It returns a child process.
         * @throws Throws {@link !Error} if failed to spawn the child process.
         *
         * @example
         * ```javascript
         * try {
         *   const cmd = new Rsvim.proc.Command("ls");
         *   const child = cmd.spawn();
         * } catch (e) {
         *   Rsvim.cmd.echo(`Failed to spawn child process: ${e}`);
         * }
         * ```
         */
        spawn(): RsvimProc.ChildProcess;
    }
    /**
     * Child process spawned from command.
     */
    class ChildProcess {
        #private;
        /** @hideconstructor */
        constructor(execPath: string, options: RsvimProc.CommandOptions, rid: number, stdinRid: number | null | undefined, stdoutRid: number | null | undefined, stderrRid: number | null | undefined);
        get execPath(): string;
        get options(): RsvimProc.CommandOptions;
        get stdout(): RsvimProc.ChildProcessReadableStream | null | undefined;
        get stderr(): RsvimProc.ChildProcessReadableStream | null | undefined;
        /**
         * Child process is already completed.
         *
         * @example
         * ```javascript
         * const child = new Rsvim.proc.Command("ls").spawn();
         * {
         *   await using usedChild = child;
         *   Rsvim.cmd.echo(usedChild.isDisposed); // false
         * }
         * Rsvim.cmd.echo(usedChild.isDisposed); // true
         * ```
         */
        get isDisposed(): boolean;
        /**
         * Wait for the child process finish with `await using` API instead of `wait`.
         *
         * @returns {void} It returns nothing.
         */
        [Symbol.asyncDispose](): Promise<void>;
        /**
         * Wait for child process complete.
         *
         * @returns {RsvimProc.ChildProcessExitStatus} It returns a child process exit status.
         * @throws Throws {@link !Error} if the child process is already finished, or failed to wait.
         *
         * @example
         * ```javascript
         * try {
         *   const cmd = new Rsvim.proc.Command("ls");
         *   const child = cmd.spawn();
         *   const output = await child.stdout.text();
         *   const exitStatus = await child.wait();
         *   Rsvim.cmd.echo(`"ls" command is completed successfully: ${exitStatus.success}.`);
         * } catch (e) {
         *   Rsvim.cmd.echo(`Failed to run "ls" command.`);
         * }
         * ```
         */
        wait(): Promise<RsvimProc.ChildProcessExitStatus>;
        /**
         * Get child process exit status.
         *
         * @returns {RsvimProc.ChildProcessExitStatus | null | undefined} It returns exit status if the child process is already finished, otherwise it returns `null`.
         *
         * @example
         * ```javascript
         * const child = new Rsvim.proc.Command("ls").spawn();
         * await child.wait();
         * const exitStatus = child.exitStatus;
         * ```
         */
        get exitStatus(): RsvimProc.ChildProcessExitStatus | null | undefined;
    }
    /**
     * Child process readable stream.
     */
    class ChildProcessReadableStream {
        #private;
        /** @hideconstructor */
        constructor(rid: number);
        /**
         * Read text from the stream.
         *
         * @returns {string} It returns the read text from child process stdio channel.
         * @throws Throws {@link !Error} if failed to read.
         *
         * @example
         * ```javascript
         * try {
         *   const cmd = new Rsvim.proc.Command("ls");
         *   const child = cmd.spawn();
         *   const output = await child.stdout.text();
         *   Rsvim.cmd.echo(`"ls" command output:${output}`);
         * } catch (e) {
         *   Rsvim.cmd.echo(`Failed to run "ls" command.`);
         * }
         * ```
         */
        text(): Promise<string>;
    }
    /**
     * Command options when creating a child-process command.
     *
     * @see {@link RsvimProc.Command}
     */
    type CommandOptions = {
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
     * @see {@link RsvimProc.ChildProcess}
     */
    type ChildProcessExitStatus = {
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
}
/**
 * The `Rsvim.rt` global object for javascript runtime (editor process).
 *
 * @example
 * ```javascript
 * // Create a alias to 'Rsvim.rt'.
 * const rt = Rsvim.rt;
 * ```
 *
 * @category General APIs
 */
export declare namespace RsvimRt {
    /**
     * Exit editor.
     *
     * :::tip
     * To ensure file system data safety, editor will wait for all the ongoing file write operations
     * to complete before actually exiting, however any new write requests will be rejected.
     * :::
     *
     * @param {exitCode?} exitCode - (Optional) The editor process exit with this exit code, by default with code `0`.
     *
     * @throws Throws {@link !TypeError} if `exitCode` is neither an integer nor `undefined`.
     *
     * @example
     * ```javascript
     * // Exit with default exit code `0`.
     * Rsvim.rt.exit();
     *
     * // Exit with error exit code `-1`.
     * Rsvim.rt.exit(-1);
     * ```
     */
    function exit(exitCode?: number): void;
}
/**
 * The `Rsvim.syn` global object for javascript runtime (editor process).
 *
 * @example
 * ```javascript
 * // Create a alias to 'Rsvim.syn'.
 * const syn = Rsvim.syn;
 * ```
 *
 * @category Editor APIs
 */
export declare namespace RsvimSyn {
    /**
     * Load tree-sitter parsers.
     *
     * @see [tree-sitter - List of parsers](https://github.com/tree-sitter/tree-sitter/wiki/List-of-parsers)
     *
     * @param {RsvimSyn.LoadParserOptions} options - Load options.
     *
     * @returns {string[]} It returns all the loaded parser names.
     *
     * @throws Throws {@link !TypeError} if `options` is an invalid option, throws {@link !Error} if failed to load.
     *
     * @example
     * ```javascript
     * // Load `tree-sitter-c` parser.
     * const parserNames = await Rsvim.syn.loadParser({grammarPath: "./tree-sitter-c"});
     * Rsvim.cmd.echo(`Loaded parsers: ${parserNames}`);
     * ```
     */
    function loadParser(options: RsvimSyn.LoadParserOptions): Promise<string[]>;
    /**
     * Load tree-sitter parsers synchronizely.
     *
     * @see {@link loadParser}
     *
     * @param {RsvimSyn.LoadParserOptions} options - Load options.
     *
     * @returns {string[]} It returns all the loaded parser names.
     *
     * @throws Throws {@link !TypeError} if `options` is an invalid option, throws {@link !Error} if failed to load.
     *
     * @example
     * ```javascript
     * // Load `tree-sitter-c` parser synchronizely.
     * const parserNames = Rsvim.syn.loadParserSync({grammarPath: "./tree-sitter-c"});
     * Rsvim.cmd.echo(`Loaded parsers: ${parserNames}`);
     * ```
     */
    function loadParserSync(options: RsvimSyn.LoadParserOptions): string[];
    /**
     * List all loaded tree-sitter parsers.
     *
     * @returns {string[]} It returns all the loaded parser names.
     *
     * @example
     * ```javascript
     * // Print all loaded parser names.
     * const allParserNames = Rsvim.syn.listParsers();
     * Rsvim.cmd.echo(`All loaded parsers: ${allParserNames}`);
     * ```
     */
    function listParsers(): string[];
    /**
     * Get tree-sitter parser metadata by parser name.
     *
     * @param {string} name - The parser's name.
     *
     * @returns {RsvimSyn.ParserMetadata | undefined} It returns all the loaded parser names.
     *
     * @example
     * ```javascript
     * // Get parser metadata by name.
     * const parserMetadata = Rsvim.syn.getParserMetadata("rust");
     * Rsvim.cmd.echo(`Rust parser metadata: ${parserMetadata}`);
     * ```
     */
    function getParserMetadata(name: string): RsvimSyn.ParserMetadata | undefined;
    /**
     * Options to load a tree-sitter parser.
     *
     * @see {@link RsvimSyn.loadParser}
     */
    type LoadParserOptions = {
        /**
         * The tree-sitter parser path to load.
         */
        grammarPath: string;
    };
    /**
     * Tree-sitter parser metadata.
     *
     * @see {@link RsvimSyn.getParserMetadata}
     */
    type ParserMetadata = {
        /**
         * The tree-sitter parser name.
         */
        name: string;
        /**
         * The tree-sitter parser name's camelcase.
         */
        camelcase: string;
        /**
         * The tree-sitter parser scope.
         */
        scope: string;
        /**
         * The tree-sitter parser path.
         */
        path: string;
        /**
         * The tree-sitter parser file types.
         */
        fileTypes: string[];
        /**
         * The tree-sitter parser highlights query path.
         */
        highlightsPath?: string;
        /**
         * The tree-sitter parser highlights query.
         */
        highlightsQuery?: string;
        /**
         * The tree-sitter parser tags query path.
         */
        tagsPath?: string;
        /**
         * The tree-sitter parser tags query.
         */
        tagsQuery?: string;
        /**
         * The tree-sitter parser injections query path.
         */
        injectionsPath?: string;
        /**
         * The tree-sitter parser injections query.
         */
        injectionsQuery?: string;
        /**
         * The tree-sitter parser injection regex.
         */
        injectionRegex?: string;
    };
}
/**
 * The `Rsvim` global object.
 *
 * @example
 * ```javascript
 * // Create a alias to 'Rsvim'.
 * const vim = Rsvim;
 * ```
 *
 * @category Global Object
 */
export declare namespace Rsvim {
    export import buf = RsvimBuf;
    export import cmd = RsvimCmd;
    export import fs = RsvimFs;
    const opt: RsvimOpt;
    export import proc = RsvimProc;
    export import rt = RsvimRt;
    export import syn = RsvimSyn;
}
type RsvimNamespaceType = typeof Rsvim;
declare global {
    var Rsvim: RsvimNamespaceType;
}
export {};
