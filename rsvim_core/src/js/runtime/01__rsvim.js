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
/** @hidden */
function assertTrue(condition, message) {
    if (!condition) {
        throw new Error(message);
    }
}
/** @hidden */
function assertFalse(condition, message) {
    if (!!condition) {
        throw new Error(message);
    }
}
/** @hidden */
function isNull(arg) {
    return arg === undefined || arg === null;
}
/** @hidden */
function isString(arg) {
    return typeof arg === "string";
}
/** @hidden */
function checkNotNull(arg, msg) {
    if (isNull(arg)) {
        throw new TypeError(`${msg} cannot be undefined or null`);
    }
}
/** @hidden */
function checkIsNumber(arg, msg) {
    if (typeof arg !== "number") {
        throw new TypeError(`${msg} must be a number, but found ${typeof arg}`);
    }
}
/** @hidden */
function checkIsInteger(arg, msg) {
    checkIsNumber(arg, msg);
    if (!Number.isInteger(arg)) {
        throw new TypeError(`${msg} must be an integer, but found ${typeof arg}`);
    }
}
/** @hidden */
function checkIsBoolean(arg, msg) {
    if (typeof arg !== "boolean") {
        throw new TypeError(`${msg} must be a boolean, but found ${typeof arg}`);
    }
}
/** @hidden */
function checkIsString(arg, msg) {
    if (!isString(arg)) {
        throw new TypeError(`${msg} must be a string, but found ${typeof arg}`);
    }
}
/** @hidden */
function checkOptionalString(arg, msg) {
    if (!(isString(arg) || isNull(arg))) {
        throw new TypeError(`${msg} must be a string?, but found ${typeof arg}`);
    }
}
/** @hidden */
function checkMatchPattern(arg, pat, msg) {
    checkIsString(arg, msg);
    if (!pat.test(arg)) {
        throw new Error(`${msg} is invalid pattern: ${arg}"`);
    }
}
/** @hidden */
function checkIsFunction(arg, msg) {
    if (typeof arg !== "function") {
        throw new TypeError(`${msg} must be a function, but found ${typeof arg}`);
    }
}
/** @hidden */
function checkIsArray(arg, msg) {
    if (!Array.isArray(arg)) {
        throw new TypeError(`${msg} must be an array, but found ${typeof arg}`);
    }
}
/** @hidden */
function checkIsObject(arg, msg) {
    if (typeof arg !== "object") {
        throw new TypeError(`${msg} must be an object, but found ${typeof arg}`);
    }
}
/** @hidden */
function checkIsUint8Array(arg, msg) {
    if (!(arg instanceof Uint8Array)) {
        throw new TypeError(`${msg} must be a Uint8Array, buf found ${typeof arg}`);
    }
}
/** @hidden */
function checkIsOptions(arg, options, msg) {
    if (!options.includes(arg)) {
        throw new RangeError(`${msg} is an invalid option: ${arg}`);
    }
}
/** @hidden */
function boundByIntegers(arg, bound) {
    if (arg < bound[0]) {
        return bound[0];
    }
    if (arg > bound[1]) {
        return bound[1];
    }
    return arg;
}
/** @hidden */
function setDefaultFields(arg, defaults) {
    for (const [key, val] of Object.entries(defaults)) {
        if (!Object.hasOwn(arg, key)) {
            Object.defineProperty(arg, key, { value: val, writable: true });
        }
    }
}
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
export var RsvimBuf;
(function (RsvimBuf) {
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
    function current() {
        // @ts-ignore Ignore warning
        return __InternalRsvimGlobalObject.buf_current();
    }
    RsvimBuf.current = current;
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
    function list() {
        // @ts-ignore Ignore warning
        return __InternalRsvimGlobalObject.buf_list();
    }
    RsvimBuf.list = list;
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
    function writeSync(bufId) {
        checkIsInteger(bufId, `"Rsvim.buf.writeSync" bufId`);
        // @ts-ignore Ignore warning
        return __InternalRsvimGlobalObject.buf_write_sync(bufId);
    }
    RsvimBuf.writeSync = writeSync;
})(RsvimBuf || (RsvimBuf = {}));
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
export var RsvimCmd;
(function (RsvimCmd) {
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
    function create(name, callback, attributes, options) {
        checkMatchPattern(name, /^[A-Za-z_!][A-Za-z0-9_!]*$/, `"Rsvim.cmd.create" name`);
        checkIsFunction(callback, `"Rsvim.cmd.create" callback`);
        attributes = attributes ?? { bang: false, nargs: "0" };
        checkIsObject(attributes, `"Rsvim.cmd.create" attributes`);
        setDefaultFields(attributes, { bang: false, nargs: "0" });
        checkIsBoolean(attributes.bang, `"Rsvim.cmd.create" bang attribute`);
        checkIsOptions(attributes.nargs, ["0", "1", "?", "+", "*"], `"Rsvim.cmd.create" nargs attribute`);
        options = options ?? { force: true };
        checkIsObject(options, `"Rsvim.cmd.create" options`);
        setDefaultFields(options, { force: true });
        checkIsBoolean(options.force, `"Rsvim.cmd.create" force option`);
        checkOptionalString(options.alias, `"Rsvim.cmd.create" alias option`);
        // @ts-ignore Ignore warning
        return __InternalRsvimGlobalObject.cmd_create(name, callback, attributes, options);
    }
    RsvimCmd.create = create;
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
    function echo(message) {
        checkNotNull(message, `"Rsvim.cmd.echo" message`);
        // @ts-ignore Ignore warning
        __InternalRsvimGlobalObject.cmd_echo(message);
    }
    RsvimCmd.echo = echo;
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
    function list() {
        // @ts-ignore Ignore warning
        return __InternalRsvimGlobalObject.cmd_list();
    }
    RsvimCmd.list = list;
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
    function get(name) {
        checkIsString(name, `"Rsvim.cmd.get" name`);
        // @ts-ignore Ignore warning
        return __InternalRsvimGlobalObject.cmd_get(name);
    }
    RsvimCmd.get = get;
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
    function remove(name) {
        checkIsString(name, `"Rsvim.cmd.remove" name`);
        // @ts-ignore Ignore warning
        return __InternalRsvimGlobalObject.cmd_remove(name);
    }
    RsvimCmd.remove = remove;
})(RsvimCmd || (RsvimCmd = {}));
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
export var RsvimFs;
(function (RsvimFs) {
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
    async function open(path, options) {
        checkIsString(path, `"Rsvim.fs.open" path`);
        options = options ?? { read: true };
        checkIsObject(options, `"Rsvim.fs.open" options`);
        setDefaultFields(options, {
            append: false,
            create: false,
            createNew: false,
            read: false,
            truncate: false,
            write: false,
        });
        checkIsBoolean(options.append, `"Rsvim.fs.open" append option`);
        checkIsBoolean(options.create, `"Rsvim.fs.open" create option`);
        checkIsBoolean(options.createNew, `"Rsvim.fs.open" createNew option`);
        checkIsBoolean(options.read, `"Rsvim.fs.open" read option`);
        checkIsBoolean(options.truncate, `"Rsvim.fs.open" truncate option`);
        checkIsBoolean(options.write, `"Rsvim.fs.open" write option`);
        // @ts-ignore Ignore warning
        const handle = await __InternalRsvimGlobalObject.fs_open(path, options);
        return new RsvimFs.File(handle);
    }
    RsvimFs.open = open;
    /**
     * Sync version of {@link open}.
     *
     * @example
     * ```javascript
     * const file = Rsvim.fs.openSync("README.md");
     * ```
     */
    function openSync(path, options) {
        checkIsString(path, `"Rsvim.fs.openSync" path`);
        options = options ?? { read: true };
        checkIsObject(options, `"Rsvim.fs.open" options`);
        setDefaultFields(options, {
            append: false,
            create: false,
            createNew: false,
            read: false,
            truncate: false,
            write: false,
        });
        checkIsBoolean(options.append, `"Rsvim.fs.open" append option`);
        checkIsBoolean(options.create, `"Rsvim.fs.open" create option`);
        checkIsBoolean(options.createNew, `"Rsvim.fs.open" createNew option`);
        checkIsBoolean(options.read, `"Rsvim.fs.open" read option`);
        checkIsBoolean(options.truncate, `"Rsvim.fs.open" truncate option`);
        checkIsBoolean(options.write, `"Rsvim.fs.open" write option`);
        // @ts-ignore Ignore warning
        const handle = __InternalRsvimGlobalObject.fs_open_sync(path, options);
        return new RsvimFs.File(handle);
    }
    RsvimFs.openSync = openSync;
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
    async function readFile(path) {
        checkIsString(path, `"Rsvim.fs.readFile" path`);
        // @ts-ignore Ignore warning
        return await __InternalRsvimGlobalObject.fs_read_file(path);
    }
    RsvimFs.readFile = readFile;
    /**
     * Sync version of {@link readFile}.
     *
     * @example
     * ```javascript
     * const buffer = Rsvim.fs.readFileSync("README.md");
     * ```
     */
    function readFileSync(path) {
        checkIsString(path, `"Rsvim.fs.readFileSync" path`);
        // @ts-ignore Ignore warning
        return __InternalRsvimGlobalObject.fs_read_file_sync(path);
    }
    RsvimFs.readFileSync = readFileSync;
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
    async function readTextFile(path) {
        checkIsString(path, `"Rsvim.fs.readTextFile" path`);
        // @ts-ignore Ignore warning
        return await __InternalRsvimGlobalObject.fs_read_text_file(path);
    }
    RsvimFs.readTextFile = readTextFile;
    /**
     * Sync version of {@link readTextFile}.
     *
     * @example
     * ```javascript
     * const payload = Rsvim.fs.readTextFileSync("README.md");
     * ```
     */
    function readTextFileSync(path) {
        checkIsString(path, `"Rsvim.fs.readTextFileSync" path`);
        // @ts-ignore Ignore warning
        return __InternalRsvimGlobalObject.fs_read_text_file_sync(path);
    }
    RsvimFs.readTextFileSync = readTextFileSync;
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
    async function lstat(path) {
        checkIsString(path, `"Rsvim.fs.lstat" path`);
        // @ts-ignore Ignore warning
        return await __InternalRsvimGlobalObject.fs_lstat(path);
    }
    RsvimFs.lstat = lstat;
    /**
     * Sync version of {@link lstat}.
     *
     * @example
     * ```javascript
     * const fstat = Rsvim.fs.lstatSync("README.md");
     * ```
     */
    function lstatSync(path) {
        checkIsString(path, `"Rsvim.fs.lstatSync" path`);
        // @ts-ignore Ignore warning
        return __InternalRsvimGlobalObject.fs_lstat_sync(path);
    }
    RsvimFs.lstatSync = lstatSync;
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
    async function stat(path) {
        checkIsString(path, `"Rsvim.fs.stat" path`);
        // @ts-ignore Ignore warning
        return await __InternalRsvimGlobalObject.fs_stat(path);
    }
    RsvimFs.stat = stat;
    /**
     * Sync version of {@link stat}.
     *
     * @example
     * ```javascript
     * const fstat = Rsvim.fs.statSync("README.md");
     * ```
     */
    function statSync(path) {
        checkIsString(path, `"Rsvim.fs.statSync" path`);
        // @ts-ignore Ignore warning
        return __InternalRsvimGlobalObject.fs_stat_sync(path);
    }
    RsvimFs.statSync = statSync;
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
    async function symlink(oldpath, newpath, options) {
        checkIsString(oldpath, `"Rsvim.fs.symlink" oldpath`);
        checkIsString(newpath, `"Rsvim.fs.symlink" newpath`);
        options = options ?? "junction";
        if (!isNull(options)) {
            checkIsOptions(options, ["file", "dir", "junction"], `"Rsvim.fs.symlink" options`);
        }
        // @ts-ignore Ignore warning
        return await __InternalRsvimGlobalObject.fs_symlink(oldpath, newpath, options);
    }
    RsvimFs.symlink = symlink;
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
    function symlinkSync(oldpath, newpath, options) {
        checkIsString(oldpath, `"Rsvim.fs.symlink" oldpath`);
        checkIsString(newpath, `"Rsvim.fs.symlink" newpath`);
        options = options ?? "junction";
        if (!isNull(options)) {
            checkIsOptions(options, ["file", "dir", "junction"], `"Rsvim.fs.symlink" options`);
        }
        // @ts-ignore Ignore warning
        __InternalRsvimGlobalObject.fs_symlink_sync(oldpath, newpath, options);
    }
    RsvimFs.symlinkSync = symlinkSync;
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
    async function link(oldpath, newpath) {
        checkIsString(oldpath, `"Rsvim.fs.link" oldpath`);
        checkIsString(newpath, `"Rsvim.fs.link" newpath`);
        // @ts-ignore Ignore warning
        return await __InternalRsvimGlobalObject.fs_link(oldpath, newpath);
    }
    RsvimFs.link = link;
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
    function linkSync(oldpath, newpath) {
        checkIsString(oldpath, `"Rsvim.fs.link" oldpath`);
        checkIsString(newpath, `"Rsvim.fs.link" newpath`);
        // @ts-ignore Ignore warning
        __InternalRsvimGlobalObject.fs_link_sync(oldpath, newpath);
    }
    RsvimFs.linkSync = linkSync;
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
    async function mkdir(path, options) {
        checkIsString(path, `"Rsvim.fs.mkdir" path`);
        options = options ?? {
            recursive: false,
            mode: 0o777,
        };
        checkIsObject(options, `"Rsvim.fs.mkdir" options`);
        setDefaultFields(options, {
            recursive: false,
            mode: 0o777,
        });
        // @ts-ignore Ignore warning
        return await __InternalRsvimGlobalObject.fs_mkdir(path, options);
    }
    RsvimFs.mkdir = mkdir;
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
    function mkdirSync(path, options) {
        checkIsString(path, `"Rsvim.fs.mkdirSync" path`);
        options = options ?? {
            recursive: false,
            mode: 0o777,
        };
        checkIsObject(options, `"Rsvim.fs.mkdirSync" options`);
        setDefaultFields(options, {
            recursive: false,
            mode: 0o777,
        });
        // @ts-ignore Ignore warning
        __InternalRsvimGlobalObject.fs_mkdir_sync(path, options);
    }
    RsvimFs.mkdirSync = mkdirSync;
    /**
     * The File object that access to an open file on filesystem.
     *
     * @see {@link RsvimFs.open}
     */
    class File {
        /** @hidden */
        #rid;
        /** @hidden */
        constructor(rid) {
            this.#rid = rid;
        }
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
        close() {
            assertFalse(isNull(this.#rid), "RsvimFs.File is already closed!");
            // @ts-ignore Ignore warning
            __InternalRsvimGlobalObject.fs_close(this.#rid);
            this.#rid = null;
        }
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
        [Symbol.dispose]() {
            this.close();
        }
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
        get isDisposed() {
            return isNull(this.#rid);
        }
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
        async read(buf) {
            checkIsUint8Array(buf, `"RsvimFs.File.read" buf`);
            // @ts-ignore Ignore warning
            const n = await __InternalRsvimGlobalObject.fs_read(this.#rid, buf.buffer);
            return n;
        }
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
        readSync(buf) {
            checkIsUint8Array(buf, `"RsvimFs.File.readSync" buf`);
            // @ts-ignore Ignore warning
            return __InternalRsvimGlobalObject.fs_read_sync(this.#rid, buf.buffer);
        }
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
        async write(buf) {
            checkIsUint8Array(buf, `"RsvimFs.File.write" buf`);
            // @ts-ignore Ignore warning
            const n = await __InternalRsvimGlobalObject.fs_write(this.#rid, buf.buffer);
            return n;
        }
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
        writeSync(buf) {
            checkIsUint8Array(buf, `"RsvimFs.File.writeSync" buf`);
            // @ts-ignore Ignore warning
            return __InternalRsvimGlobalObject.fs_write_sync(this.#rid, buf.buffer);
        }
    }
    RsvimFs.File = File;
})(RsvimFs || (RsvimFs = {}));
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
export class RsvimOpt {
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
    get expandTab() {
        // @ts-ignore Ignore warning
        return __InternalRsvimGlobalObject.opt_get_expand_tab();
    }
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
    set expandTab(value) {
        checkIsBoolean(value, `"Rsvim.opt.expandTab" value`);
        // @ts-ignore Ignore warning
        __InternalRsvimGlobalObject.opt_set_expand_tab(value);
    }
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
    get fileEncoding() {
        // @ts-ignore Ignore warning
        return __InternalRsvimGlobalObject.opt_get_file_encoding();
    }
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
    set fileEncoding(value) {
        checkIsOptions(value, ["utf-8"], `"Rsvim.opt.fileEncoding" value`);
        // @ts-ignore Ignore warning
        __InternalRsvimGlobalObject.opt_set_file_encoding(value);
    }
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
    get fileFormat() {
        // @ts-ignore Ignore warning
        return __InternalRsvimGlobalObject.opt_get_file_format();
    }
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
    set fileFormat(value) {
        checkIsOptions(value, ["dos", "unix", "mac"], `"Rsvim.opt.fileFormat" value`);
        // @ts-ignore Ignore warning
        __InternalRsvimGlobalObject.opt_set_file_format(value);
    }
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
    get fixEndOfLine() {
        // @ts-ignore Ignore warning
        return __InternalRsvimGlobalObject.opt_get_fix_end_of_line();
    }
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
    set fixEndOfLine(value) {
        checkIsBoolean(value, `"Rsvim.opt.fixEndOfLine" value`);
        // @ts-ignore Ignore warning
        __InternalRsvimGlobalObject.opt_set_fix_end_of_line(value);
    }
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
    get lineBreak() {
        // @ts-ignore Ignore warning
        return __InternalRsvimGlobalObject.opt_get_line_break();
    }
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
    set lineBreak(value) {
        checkIsBoolean(value, `"Rsvim.opt.lineBreak" value`);
        // @ts-ignore Ignore warning
        __InternalRsvimGlobalObject.opt_set_line_break(value);
    }
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
    get shiftWidth() {
        // @ts-ignore Ignore warning
        return __InternalRsvimGlobalObject.opt_get_shift_width();
    }
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
    set shiftWidth(value) {
        checkIsInteger(value, `"Rsvim.opt.shiftWidth" value`);
        value = boundByIntegers(value, [1, 255]);
        // @ts-ignore Ignore warning
        __InternalRsvimGlobalObject.opt_set_shift_width(value);
    }
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
    get syntaxParserLibPath() {
        // @ts-ignore Ignore warning
        return __InternalRsvimGlobalObject.opt_get_syntax_parser_lib_path();
    }
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
    set syntaxParserLibPath(value) {
        checkIsString(value, `"Rsvim.opt.syntaxParserLibPath" value`);
        // @ts-ignore Ignore warning
        __InternalRsvimGlobalObject.opt_set_syntax_parser_lib_path(value);
    }
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
    get tabStop() {
        // @ts-ignore Ignore warning
        return __InternalRsvimGlobalObject.opt_get_tab_stop();
    }
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
    set tabStop(value) {
        checkIsInteger(value, `"Rsvim.opt.tabStop" value`);
        value = boundByIntegers(value, [1, 255]);
        // @ts-ignore Ignore warning
        __InternalRsvimGlobalObject.opt_set_tab_stop(value);
    }
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
    get wrap() {
        // @ts-ignore Ignore warning
        return __InternalRsvimGlobalObject.opt_get_wrap();
    }
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
    set wrap(value) {
        checkIsBoolean(value, `"Rsvim.opt.wrap" value`);
        // @ts-ignore Ignore warning
        __InternalRsvimGlobalObject.opt_set_wrap(value);
    }
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
export var RsvimProc;
(function (RsvimProc) {
    /**
     * The command that create a child process.
     */
    class Command {
        /** @hidden */
        #execPath;
        /** @hidden */
        #options;
        constructor(execPath, options) {
            checkIsString(execPath, `"Rsvim.proc.Command" execPath`);
            options = options ?? {
                args: [],
                clearEnv: false,
                detached: false,
                env: { __proto__: null },
                stdin: "null",
                stdout: "piped",
                stderr: "piped",
            };
            checkIsObject(options, `"Rsvim.proc.Command" options`);
            setDefaultFields(options, {
                args: [],
                clearEnv: false,
                detached: false,
                env: { __proto__: null },
                stdin: "null",
                stdout: "piped",
                stderr: "piped",
            });
            checkIsArray(options.args, `"Rsvim.proc.Command" args option`);
            checkIsBoolean(options.clearEnv, `"Rsvim.proc.Command" clearEnv option`);
            checkIsBoolean(options.detached, `"Rsvim.proc.Command" detached option`);
            checkIsObject(options.env, `"Rsvim.proc.Command" env option`);
            checkIsOptions(options.stdin, ["null", "piped", "inherit"], `"Rsvim.proc.Command" stdin option`);
            checkIsOptions(options.stdout, ["null", "piped", "inherit"], `"Rsvim.proc.Command" stdout option`);
            checkIsOptions(options.stderr, ["null", "piped", "inherit"], `"Rsvim.proc.Command" stderr option`);
            this.#execPath = execPath;
            this.#options = options;
        }
        get execPath() {
            return this.#execPath;
        }
        get options() {
            return this.#options;
        }
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
        spawn() {
            // @ts-ignore Ignore warning
            const child = __InternalRsvimGlobalObject.proc_spawn_child(this.#execPath, this.#options);
            return new RsvimProc.ChildProcess(child.execPath, child.options, child.rid, child.stdinRid, child.stdoutRid, child.stderrRid);
        }
    }
    RsvimProc.Command = Command;
    /**
     * Child process spawned from command.
     */
    class ChildProcess {
        /** @hidden */
        #execPath;
        /** @hidden */
        #options;
        /** @hidden */
        #rid;
        /** @hidden */
        #exitStatus;
        /** @hidden */
        #stdinRid;
        /** @hidden */
        #stdout;
        /** @hidden */
        #stderr;
        /** @hideconstructor */
        constructor(execPath, options, rid, stdinRid, stdoutRid, stderrRid) {
            this.#execPath = execPath;
            this.#options = options;
            this.#rid = rid;
            this.#exitStatus = null;
            this.#stdinRid = stdinRid;
            this.#stdout =
                stdoutRid != null
                    ? new RsvimProc.ChildProcessReadableStream(stdoutRid)
                    : null;
            this.#stderr =
                stderrRid != null
                    ? new RsvimProc.ChildProcessReadableStream(stderrRid)
                    : null;
        }
        get execPath() {
            return this.#execPath;
        }
        get options() {
            return this.#options;
        }
        get stdout() {
            return this.#stdout;
        }
        get stderr() {
            return this.#stderr;
        }
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
        get isDisposed() {
            return isNull(this.#rid);
        }
        /**
         * Wait for the child process finish with `await using` API instead of `wait`.
         *
         * @returns {void} It returns nothing.
         */
        async [Symbol.asyncDispose]() {
            await this.wait();
        }
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
        async wait() {
            assertFalse(isNull(this.#rid), "RsvimProc.ChildProcess is already finished!");
            this.#exitStatus =
                // @ts-ignore Ignore warning
                (await __InternalRsvimGlobalObject.proc_wait_child(this.#rid));
            this.#rid = null;
            return this.#exitStatus;
        }
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
        get exitStatus() {
            return this.#exitStatus;
        }
    }
    RsvimProc.ChildProcess = ChildProcess;
    /**
     * Child process readable stream.
     */
    class ChildProcessReadableStream {
        /** @hidden */
        #rid;
        /** @hideconstructor */
        constructor(rid) {
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
         *   const cmd = new Rsvim.proc.Command("ls");
         *   const child = cmd.spawn();
         *   const output = await child.stdout.text();
         *   Rsvim.cmd.echo(`"ls" command output:${output}`);
         * } catch (e) {
         *   Rsvim.cmd.echo(`Failed to run "ls" command.`);
         * }
         * ```
         */
        async text() {
            const payload = 
            // @ts-ignore Ignore warning
            await __InternalRsvimGlobalObject.proc_read_text_from_child(this.#rid);
            return payload;
        }
    }
    RsvimProc.ChildProcessReadableStream = ChildProcessReadableStream;
})(RsvimProc || (RsvimProc = {}));
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
export var RsvimRt;
(function (RsvimRt) {
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
    function exit(exitCode) {
        exitCode = exitCode ?? 0;
        checkIsInteger(exitCode, `"Rsvim.rt.exit" code`);
        // @ts-ignore Ignore warning
        __InternalRsvimGlobalObject.rt_exit(exitCode);
    }
    RsvimRt.exit = exit;
})(RsvimRt || (RsvimRt = {}));
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
export var RsvimSyn;
(function (RsvimSyn) {
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
    async function loadParser(options) {
        checkIsObject(options, `"Rsvim.syn.loadParser" options`);
        checkIsString(options.grammarPath, `"Rsvim.syn.loadParser" grammarPath option`);
        const parserNames = 
        // @ts-ignore Ignore warning
        await __InternalRsvimGlobalObject.syn_load_parser(options);
        return parserNames;
    }
    RsvimSyn.loadParser = loadParser;
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
    function loadParserSync(options) {
        checkIsObject(options, `"Rsvim.syn.loadParserSync" options`);
        checkIsString(options.grammarPath, `"Rsvim.syn.loadParserSync" grammarPath option`);
        const parserNames = 
        // @ts-ignore Ignore warning
        __InternalRsvimGlobalObject.syn_load_parser_sync(options);
        return parserNames;
    }
    RsvimSyn.loadParserSync = loadParserSync;
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
    function listParsers() {
        // @ts-ignore Ignore warning
        return __InternalRsvimGlobalObject.syn_list_parsers();
    }
    RsvimSyn.listParsers = listParsers;
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
    function getParserMetadata(name) {
        checkIsString(name, `"Rsvim.syn.getParserMetadata" name`);
        // @ts-ignore Ignore warning
        return __InternalRsvimGlobalObject.syn_get_parser_metadata(name);
    }
    RsvimSyn.getParserMetadata = getParserMetadata;
})(RsvimSyn || (RsvimSyn = {}));
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
export var Rsvim;
(function (Rsvim) {
    Rsvim.buf = RsvimBuf;
    Rsvim.cmd = RsvimCmd;
    Rsvim.fs = RsvimFs;
    Rsvim.opt = new RsvimOpt();
    Rsvim.proc = RsvimProc;
    Rsvim.rt = RsvimRt;
    Rsvim.syn = RsvimSyn;
})(Rsvim || (Rsvim = {}));
(function (globalThis) {
    globalThis.Rsvim = Rsvim;
})(globalThis);
