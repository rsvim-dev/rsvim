import { isBoolean } from "es-toolkit/predicate/isBoolean";
import { isNil } from "es-toolkit/predicate/isNil";
import { isString } from "es-toolkit/predicate/isString";
import { assert } from "es-toolkit/util";

/**
 * ---
 * title: fs API
 * sidebar_position: 5
 * ---
 *
 * The `fs` module for file system and file IO.
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
 * The `fs` module for file system and file IO.
 *
 * @example
 * ```javascript
 * import fs from "rsvim:fs";
 *
 * const file = await fs.open("README.md");
 * ```
 *
 * @category General APIs
 */
export namespace fs {
  /**
   * Open a file and resolve to an instance of {@link fs.File}. The file does not need to previously exist if using the `create` or `createNew` open options.
   * The caller have to close the file to prevent resource leaking, see {@link fs.File.close}.
   *
   * @param {string} path - File path.
   * @param {fs.OpenOptions} options - (Optional) Open options, by default is `{read: true}`. See {@link fs.OpenOptions}.
   * @returns {Promise<fs.File>} It resolves to an instance of {@link fs.File}.
   *
   * @throws Throws {@link !Error} if any parameters are invalid, or failed to open/create the file.
   *
   * @example
   * ```javascript
   * const file = await fs.open("README.md");
   * ```
   */
  export async function open(
    path: string,
    options?: fs.OpenOptions,
  ): Promise<fs.File> {
    assert(
      isString(path),
      `fs.open: path must be a string, got ${typeof path}`,
    );

    options = options ?? { read: true };
    assert(
      typeof options === "object",
      `fs.open: options must be an object, got ${typeof options}`,
    );
    options = {
      append: false,
      create: false,
      createNew: false,
      read: false,
      truncate: false,
      write: false,
      ...options,
    };
    assert(
      isBoolean(options.append),
      `fs.open: append option must be a boolean, got ${typeof options.append}`,
    );
    assert(
      isBoolean(options.create),
      `fs.open: create option must be a boolean, got ${typeof options.create}`,
    );
    assert(
      isBoolean(options.createNew),
      `fs.open: createNew option must be a boolean, got ${typeof options.createNew}`,
    );
    assert(
      isBoolean(options.read),
      `fs.open: read option must be a boolean, got ${typeof options.read}`,
    );
    assert(
      isBoolean(options.truncate),
      `fs.open: truncate option must be a boolean, got ${typeof options.truncate}`,
    );
    assert(
      isBoolean(options.write),
      `fs.open: write option must be a boolean, got ${typeof options.write}`,
    );

    // @ts-ignore Ignore warning
    const handle = await __InternalRsvimGlobalObject.fs_open(path, options);
    return new fs.File(handle);
  }

  /**
   * Sync version of {@link open}.
   *
   * @example
   * ```javascript
   * const file = fs.openSync("README.md");
   * ```
   */
  export function openSync(path: string, options?: fs.OpenOptions): fs.File {
    assert(
      isString(path),
      `fs.openSync: path must be a string, got ${typeof path}`,
    );

    options = options ?? { read: true };
    assert(
      typeof options === "object",
      `fs.openSync: options must be an object, got ${typeof options}`,
    );
    options = {
      append: false,
      create: false,
      createNew: false,
      read: false,
      truncate: false,
      write: false,
      ...options,
    };
    assert(
      isBoolean(options.append),
      `fs.openSync: append option must be a boolean, got ${typeof options.append}`,
    );
    assert(
      isBoolean(options.create),
      `fs.openSync: create option must be a boolean, got ${typeof options.create}`,
    );
    assert(
      isBoolean(options.createNew),
      `fs.openSync: createNew option must be a boolean, got ${typeof options.createNew}`,
    );
    assert(
      isBoolean(options.read),
      `fs.openSync: read option must be a boolean, got ${typeof options.read}`,
    );
    assert(
      isBoolean(options.truncate),
      `fs.openSync: truncate option must be a boolean, got ${typeof options.truncate}`,
    );
    assert(
      isBoolean(options.write),
      `fs.openSync: write option must be a boolean, got ${typeof options.write}`,
    );

    // @ts-ignore Ignore warning
    const handle = __InternalRsvimGlobalObject.fs_open_sync(path, options);
    return new fs.File(handle);
  }

  /**
   * Read a directory with async iterator.
   *
   * @param {string} path - Directory path to read.
   * @returns {AsyncIterable<fs.DirEntry>} Async iterator.
   *
   * @throws Throws {@link !Error} if the path is invalid, or failed to read the directory.
   *
   * @example
   * ```javascript
   * for await (const entry of fs.readDir(".")) {
   *   cmd.echo(entry.name);
   * }
   * ```
   */
  export async function* readDir(path: string): AsyncIterable<fs.DirEntry> {
    assert(
      isString(path),
      `fs.readDir: path must be a string, got ${typeof path}`,
    );

    let rid: number | undefined | null;

    try {
      // @ts-ignore Ignore warning
      rid = await __InternalRsvimGlobalObject.fs_read_dir(path);

      while (true) {
        const entry =
          // @ts-ignore Ignore warning
          await __InternalRsvimGlobalObject.fs_read_dir_next(rid);
        if (entry == null) {
          break;
        }
        yield entry as fs.DirEntry;
      }
    } finally {
      if (rid != null) {
        // @ts-ignore Ignore warning
        __InternalRsvimGlobalObject.fs_read_dir_close(rid);
        rid = null;
      }
    }
  }

  /**
   * Sync version of {@link readDir}.
   *
   * @param {string} path - Directory path to read.
   * @returns {Iterable<fs.DirEntry>} Iterator.
   *
   * @throws Throws {@link !Error} if the path is invalid, or failed to read the directory.
   *
   * @example
   * ```javascript
   * for (const entry of fs.readDirSync(".")) {
   *   cmd.echo(entry.name);
   * }
   * ```
   */
  export function* readDirSync(path: string): Iterable<fs.DirEntry> {
    assert(
      isString(path),
      `fs.readDirSync: path must be a string, got ${typeof path}`,
    );

    let rid: number | undefined | null;

    try {
      // @ts-ignore Ignore warning
      rid = __InternalRsvimGlobalObject.fs_read_dir_sync(path);

      while (true) {
        // @ts-ignore Ignore warning
        const entry = __InternalRsvimGlobalObject.fs_read_dir_next_sync(rid);
        if (entry == null) {
          break;
        }
        yield entry as fs.DirEntry;
      }
    } finally {
      if (rid != null) {
        // @ts-ignore Ignore warning
        __InternalRsvimGlobalObject.fs_read_dir_close(rid);
        rid = null;
      }
    }
  }

  /**
   * Read a file in binary mode, i.e. into an array of bytes buffer, without open/close a file descriptor/handle.
   *
   * @param {string} path - File path to read.
   * @returns {Promise<Uint8Array>} It resolves to {@link !Uint8Array} that contains all the file contents as bytes array.
   *
   * @throws Throws {@link !Error} if the file name is invalid, or failed to read the file.
   *
   * @example
   * ```javascript
   * const buffer = await fs.readFile("README.md");
   * ```
   */
  export async function readFile(path: string): Promise<Uint8Array> {
    assert(
      isString(path),
      `fs.readFile: path must be a string, got ${typeof path}`,
    );

    // @ts-ignore Ignore warning
    return await __InternalRsvimGlobalObject.fs_read_file(path);
  }

  /**
   * Sync version of {@link readFile}.
   *
   * @example
   * ```javascript
   * const buffer = fs.readFileSync("README.md");
   * ```
   */
  export function readFileSync(path: string): Uint8Array {
    assert(
      isString(path),
      `fs.readFileSync: path must be a string, got ${typeof path}`,
    );

    // @ts-ignore Ignore warning
    return __InternalRsvimGlobalObject.fs_read_file_sync(path);
  }

  /**
   * Read a file in text mode, i.e. into a string, without open/close a file descriptor/handle.
   *
   * @param {string} path - File path to read.
   * @returns {Promise<string>} It resolves to text string that contains all the file contents.
   *
   * @throws Throws {@link !Error} if the file name is invalid, or failed to read the file.
   *
   * @example
   * ```javascript
   * const payload = await fs.readTextFile("README.md");
   * ```
   */
  export async function readTextFile(path: string): Promise<string> {
    assert(
      isString(path),
      `fs.readTextFile: path must be a string, got ${typeof path}`,
    );

    // @ts-ignore Ignore warning
    return await __InternalRsvimGlobalObject.fs_read_text_file(path);
  }

  /**
   * Sync version of {@link readTextFile}.
   *
   * @example
   * ```javascript
   * const payload = fs.readTextFileSync("README.md");
   * ```
   */
  export function readTextFileSync(path: string): string {
    assert(
      isString(path),
      `fs.readTextFileSync: path must be a string, got ${typeof path}`,
    );

    // @ts-ignore Ignore warning
    return __InternalRsvimGlobalObject.fs_read_text_file_sync(path);
  }

  /**
   * Get the status of a file by path.
   *
   * :::note
   * This api doesn't follow symbolic link.
   * :::
   *
   * @see {@link fs.stat}
   *
   * @param {string} path - File path.
   * @returns {Promise<fs.FileInfo>} It resolves to the file status.
   *
   * @throws Throws {@link !Error} if the file name is invalid, or failed to get file status.
   *
   * @example
   * ```javascript
   * const fstat = await fs.lstat("README.md");
   * ```
   */
  export async function lstat(path: string): Promise<fs.FileInfo> {
    assert(
      isString(path),
      `fs.lstat: path must be a string, got ${typeof path}`,
    );

    // @ts-ignore Ignore warning
    return await __InternalRsvimGlobalObject.fs_lstat(path);
  }

  /**
   * Sync version of {@link lstat}.
   *
   * @example
   * ```javascript
   * const fstat = fs.lstatSync("README.md");
   * ```
   */
  export function lstatSync(path: string): fs.FileInfo {
    assert(
      isString(path),
      `fs.lstatSync: path must be a string, got ${typeof path}`,
    );

    // @ts-ignore Ignore warning
    return __InternalRsvimGlobalObject.fs_lstat_sync(path);
  }

  /**
   * Get the status of a file by path.
   *
   * :::note
   * This api follows symbolic link.
   * :::
   *
   * @see {@link fs.lstat}
   *
   * @param {string} path - File path.
   * @returns {Promise<fs.FileInfo>} It resolves to the file status.
   *
   * @throws Throws {@link !Error} if the file name is invalid, or failed to get file status.
   *
   * @example
   * ```javascript
   * const fstat = await fs.stat("README.md");
   * ```
   */
  export async function stat(path: string): Promise<fs.FileInfo> {
    assert(
      isString(path),
      `fs.stat: path must be a string, got ${typeof path}`,
    );

    // @ts-ignore Ignore warning
    return await __InternalRsvimGlobalObject.fs_stat(path);
  }

  /**
   * Sync version of {@link stat}.
   *
   * @example
   * ```javascript
   * const fstat = fs.statSync("README.md");
   * ```
   */
  export function statSync(path: string): fs.FileInfo {
    assert(
      isString(path),
      `fs.statSync: path must be a string, got ${typeof path}`,
    );

    // @ts-ignore Ignore warning
    return __InternalRsvimGlobalObject.fs_stat_sync(path);
  }

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
   * @param {fs.SymlinkOptions} options - (Optional) Only used on Windows platforms, will be ignored on Unix platforms. By default is `"junction"`.
   * @returns {Promise<void>} It resolves to nothing.
   *
   * @throws Throws {@link !Error} if any parameter is invalid, or failed to create symbolic link from the file.
   *
   * @example
   * ```javascript
   * try {
   *   // Linux
   *   await fs.symlink("README.md", "linked-README.md");
   *   // Windows
   *   await fs.symlink("README.md", "linked-README.md", "junction");
   *   cmd.echo(`Created symbolic link "linked-README.md" pointing to "README.md"`);
   * } catch (e) {
   *   cmd.echo(`Failed to create symbolic link pointing to "README.md": ${e}`);
   * }
   * ```
   */
  export async function symlink(
    oldpath: string,
    newpath: string,
    options?: fs.SymlinkOptions,
  ): Promise<void> {
    assert(
      isString(oldpath),
      `fs.symlink: oldpath must be a string, got ${typeof oldpath}`,
    );
    assert(
      isString(newpath),
      `fs.symlink: newpath must be a string, got ${typeof newpath}`,
    );

    options = options ?? "junction";
    if (!isNil(options)) {
      assert(
        ["file", "dir", "junction"].includes(options),
        `fs.symlink: options is invalid: ${options}`,
      );
    }

    // @ts-ignore Ignore warning
    return await __InternalRsvimGlobalObject.fs_symlink(
      oldpath,
      newpath,
      options,
    );
  }

  /**
   * Sync version of {@link symlink}.
   *
   * @example
   * ```javascript
   * try {
   *   // Linux
   *   fs.symlinkSync("README.md", "linked-README.md");
   *   // Windows
   *   fs.symlinkSync("README.md", "linked-README.md", "junction");
   *   cmd.echo(`Created symbolic link "linked-README.md" pointing to "README.md"`);
   * } catch (e) {
   *   cmd.echo(`Failed to create symbolic link pointing to "README.md": ${e}`);
   * }
   * ```
   */
  export function symlinkSync(
    oldpath: string,
    newpath: string,
    options?: fs.SymlinkOptions,
  ): void {
    assert(
      isString(oldpath),
      `fs.symlink: oldpath must be a string, got ${typeof oldpath}`,
    );
    assert(
      isString(newpath),
      `fs.symlink: newpath must be a string, got ${typeof newpath}`,
    );

    options = options ?? "junction";
    if (!isNil(options)) {
      assert(
        ["file", "dir", "junction"].includes(options),
        `fs.symlink: options is invalid: ${options}`,
      );
    }

    // @ts-ignore Ignore warning
    __InternalRsvimGlobalObject.fs_symlink_sync(oldpath, newpath, options);
  }

  /**
   * Create hard link from a file path.
   *
   * @param {string} oldpath - Original file path.
   * @param {string} newpath - New symbolic link that pointing to the original file.
   * @returns {Promise<void>} It resolves to nothing.
   *
   * @throws Throws {@link !Error} if any parameter is invalid, or failed to create hard link from the file.
   *
   * @example
   * ```javascript
   * try {
   *   await fs.link("README.md", "linked-README.md");
   *   cmd.echo(`Created hard link "linked-README.md" pointing to "README.md"`);
   * } catch (e) {
   *   cmd.echo(`Failed to create hard link pointing to "README.md": ${e}`);
   * }
   * ```
   */
  export async function link(oldpath: string, newpath: string): Promise<void> {
    assert(
      isString(oldpath),
      `fs.link: oldpath must be a string, got ${typeof oldpath}`,
    );
    assert(
      isString(newpath),
      `fs.link: newpath must be a string, got ${typeof newpath}`,
    );

    // @ts-ignore Ignore warning
    return await __InternalRsvimGlobalObject.fs_link(oldpath, newpath);
  }

  /**
   * Sync version of {@link link}.
   *
   * @example
   * ```javascript
   * try {
   *   fs.linkSync("README.md", "linked-README.md");
   *   cmd.echo(`Created hard link "linked-README.md" pointing to "README.md"`);
   * } catch (e) {
   *   cmd.echo(`Failed to create hard link pointing to "README.md": ${e}`);
   * }
   * ```
   */
  export function linkSync(oldpath: string, newpath: string): void {
    assert(
      isString(oldpath),
      `fs.link: oldpath must be a string, got ${typeof oldpath}`,
    );
    assert(
      isString(newpath),
      `fs.link: newpath must be a string, got ${typeof newpath}`,
    );

    // @ts-ignore Ignore warning
    __InternalRsvimGlobalObject.fs_link_sync(oldpath, newpath);
  }

  /**
   * Make a directory.
   *
   * @param {string} path - Directory path.
   * @param {fs.MkdirOptions} options - (Optional) Options when create directory, by default is `{recursive: false, mode: 0o777}`, the `mode` option only works on Unix platform, and is ignored on Windows platform.
   * @returns {Promise<void>} It resolves to nothing.
   *
   * @throws Throws {@link !Error} if any parameter is invalid, or failed to create hard mkdir from the file.
   *
   * @example
   * ```javascript
   * try {
   *   await fs.mkdir(".rsvim");
   *   cmd.echo(`Created directory ".rsvim"`);
   * } catch (e) {
   *   cmd.echo(`Failed to create directory ".rsvim": ${e}`);
   * }
   * ```
   */
  export async function mkdir(
    path: string,
    options?: fs.MkdirOptions,
  ): Promise<void> {
    assert(
      isString(path),
      `fs.mkdir: path must be a string, got ${typeof path}`,
    );

    assert(
      options === undefined || options === null || typeof options === "object",
      `fs.mkdir: options must be an object, got ${typeof options}`,
    );
    options = {
      recursive: false,
      mode: 0o777,
      ...options,
    };

    // @ts-ignore Ignore warning
    return await __InternalRsvimGlobalObject.fs_mkdir(path, options);
  }

  /**
   * Sync version of {@link mkdir}.
   *
   * @example
   * ```javascript
   * try {
   *   fs.mkdirSync(".rsvim");
   *   cmd.echo(`Created directory ".rsvim"`);
   * } catch (e) {
   *   cmd.echo(`Failed to create directory ".rsvim": ${e}`);
   * }
   * ```
   */
  export function mkdirSync(path: string, options?: fs.MkdirOptions): void {
    assert(
      isString(path),
      `fs.mkdirSync: path must be a string, got ${typeof path}`,
    );

    assert(
      options === undefined || options === null || typeof options === "object",
      `fs.mkdirSync: options must be an object, got ${typeof options}`,
    );
    options = {
      recursive: false,
      mode: 0o777,
      ...options,
    };

    // @ts-ignore Ignore warning
    __InternalRsvimGlobalObject.fs_mkdir_sync(path, options);
  }

  /**
   * Open options.
   *
   * :::tip
   * It is same with [std::fs::OpenOptions](https://doc.rust-lang.org/std/fs/struct.OpenOptions.html) in rust std library.
   * :::
   *
   * @see {@link fs.open}
   */
  export type OpenOptions = {
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
   * @see {@link fs.open}
   */
  export class File {
    /** @hidden */
    #rid: number | null | undefined;

    /** @hidden */
    constructor(rid: number) {
      this.#rid = rid;
    }

    /**
     * Close the file.
     *
     * @throws Throws {@link !Error} if the file is already been closed.
     *
     * @example
     * ```javascript
     * const file = await fs.open("README.md");
     * // do work with the `file` object
     * file.close();
     *
     * // Or
     *
     * using file = await fs.open("README.md");
     * // do work with the `file` object
     * ```
     */
    close(): void {
      assert(!isNil(this.#rid), "fs.File is already closed!");
      // @ts-ignore Ignore warning
      __InternalRsvimGlobalObject.fs_close(this.#rid);
      this.#rid = null;
    }

    /**
     * Close the file with `using` API instead of `close`.
     *
     * @example
     * ```javascript
     * using file = await fs.open("README.md");
     * // do work with the `file` object
     * ```
     *
     * @see {@link close}
     */
    [Symbol.dispose](): void {
      this.close();
    }

    /**
     * File is already been closed.
     *
     * @example
     * ```javascript
     * const file = await fs.open("README.md");
     * if (!file.isDisposed()) {
     *   file.close();
     * }
     * ```
     */
    get isDisposed(): boolean {
      return isNil(this.#rid);
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
     * @throws Throws {@link !Error} if buf is not a Uint8Array.
     *
     * @example
     * ```javascript
     * using file = await fs.open("README.md");
     * const buf = new Uint8Array(100);
     * const n = await file.read(buf); // read 11 bytes
     * const text = new TextDecoder().decode(buf); // decode into UTF-8 string "hello world"
     * ```
     */
    async read(buf: Uint8Array): Promise<number> {
      assert(
        buf instanceof Uint8Array,
        `fs.File.read: buf must be a Uint8Array, got ${typeof buf}`,
      );

      // @ts-ignore Ignore warning
      const n = await __InternalRsvimGlobalObject.fs_read(
        this.#rid,
        buf.buffer,
      );
      return n;
    }

    /**
     * Sync version of {@link read}.
     *
     * @example
     * ```javascript
     * using file = await fs.open("README.md");
     * const buf = new Uint8Array(100);
     * const n = file.readSync(buf); // read 11 bytes
     * const text = new TextDecoder().decode(buf); // decode into UTF-8 string "hello world"
     * ```
     */
    readSync(buf: Uint8Array): number {
      assert(
        buf instanceof Uint8Array,
        `fs.File.readSync: buf must be a Uint8Array, got ${typeof buf}`,
      );

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
     * @throws Throws {@link !Error} if buf is not a Uint8Array.
     *
     * @example
     * ```javascript
     * using file = await fs.open("README.md", {write:true,create:true});
     * const buf = new TextEncoder().encode("hello world");
     * const n = await file.write(buf); // write 11 bytes
     * ```
     */
    async write(buf: Uint8Array): Promise<number> {
      assert(
        buf instanceof Uint8Array,
        `fs.File.write: buf must be a Uint8Array, got ${typeof buf}`,
      );

      // @ts-ignore Ignore warning
      const n = await __InternalRsvimGlobalObject.fs_write(
        this.#rid,
        buf.buffer,
      );
      return n;
    }

    /**
     * Sync version of {@link write}.
     *
     * @example
     * ```javascript
     * using file = await fs.open("README.md", {write:true,create:true});
     * const buf = new TextEncoder().encode("hello world");
     * const n = file.writeSync(buf); // write 11 bytes
     * ```
     */
    writeSync(buf: Uint8Array): number {
      assert(
        buf instanceof Uint8Array,
        `fs.File.writeSync: buf must be a Uint8Array, got ${typeof buf}`,
      );

      // @ts-ignore Ignore warning
      return __InternalRsvimGlobalObject.fs_write_sync(this.#rid, buf.buffer);
    }
  }

  /**
   * Directory entry returned from {@link fs.readDir} and {@link fs.readDirSync}.
   */
  export type DirEntry = {
    /**
     * File name.
     */
    fileName: string;

    /**
     * Whether it is a directory.
     */
    isDir: boolean;

    /**
     * Whether it is a normal file.
     */
    isFile: boolean;

    /**
     * Whether it is a symbolic link.
     */
    isSymlink: boolean;
  };

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
  export type FileInfo = {
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
   * @see {@link fs.symlink}
   * @inline
   * */
  export type SymlinkOptions = "file" | "dir" | "junction";

  /**
   * Mkdir options.
   */
  export type MkdirOptions = {
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

export default fs;
