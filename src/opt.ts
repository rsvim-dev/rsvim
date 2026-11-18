import { isBoolean } from "es-toolkit/predicate/isBoolean";
import { isNumber } from "es-toolkit/predicate/isNumber";
import { isString } from "es-toolkit/predicate/isString";
import { assert } from "es-toolkit/util";

/**
 * ---
 * title: opt API
 * sidebar_position: 6
 * ---
 *
 * The `opt` module for editor options.
 *
 * @packageDocumentation
 *
 * @categoryDescription Editor APIs
 * These APIs are specific for editor, such as buffers, windows, key mappings, etc.
 *
 * @categoryDescription General APIs
 * These APIs are general for common javascript-based runtime, similar to [Deno APIs](https://docs.deno.com/api/deno/).
 */

export namespace opt {
  /**
   * @see {@link opt.fileEncoding}
   * @inline
   * */
  export type FileEncodingOption = "utf-8";

  /**
   * @see {@link opt.fileFormat}
   * @inline
   */
  export type FileFormatOption = "dos" | "unix" | "mac";
}

/**
 * The `opt` module for editor options. These options will change the editor's behavior
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
 * import { opt } from "rsvim:ext/opt";
 *
 * opt.wrap = true;
 * ```
 *
 * @category Editor APIs
 * @hideconstructor
 */
export class Opt {
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
   * const value = opt.expandTab;
   * ```
   */
  get expandTab(): boolean {
    // @ts-ignore Ignore warning
    return __InternalRsvimGlobalObject.opt_get_expand_tab();
  }

  /**
   * Set the _expand-tab_ option.
   *
   * @param {boolean} value - The _expand-tab_ option.
   * @throws Throws {@link !Error} if value is not a boolean.
   *
   * @example
   * ```javascript
   * // Set the 'expand-tab' option.
   * opt.expandTab = true;
   * ```
   */
  set expandTab(value: boolean) {
    assert(
      isBoolean(value),
      `opt.expandTab: value must be a boolean, got ${typeof value}`,
    );
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
   * @returns {opt.FileEncodingOption}
   *
   * @defaultValue `"utf-8"`
   *
   * @example
   * ```javascript
   * // Get the 'file-encoding' option.
   * const value = opt.fileEncoding;
   * ```
   */
  get fileEncoding(): opt.FileEncodingOption {
    // @ts-ignore Ignore warning
    return __InternalRsvimGlobalObject.opt_get_file_encoding();
  }

  /**
   * Set the _file-encoding_ option.
   *
   * @param {opt.FileEncodingOption} value - The _file-encoding_ option.
   * @throws Throws {@link !Error} if value is an invalid option.
   *
   * @example
   * ```javascript
   * // Set the 'file-encoding' option.
   * opt.fileEncoding = "utf-8";
   * ```
   */
  set fileEncoding(value: opt.FileEncodingOption) {
    assert(value === "utf-8", `opt.fileEncoding: value is invalid: ${value}`);
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
   * @returns {opt.FileFormatOption}
   *
   * @defaultValue `"dos"` for Windows/MS-DOS, `"unix"` for Linux/Unix/MacOS.
   *
   * @example
   * ```javascript
   * // Get the 'file-format' option.
   * const value = opt.fileFormat;
   * ```
   */
  get fileFormat(): opt.FileFormatOption {
    // @ts-ignore Ignore warning
    return __InternalRsvimGlobalObject.opt_get_file_format();
  }

  /**
   * Set the _file-format_ option.
   *
   * @param {opt.FileFormatOption} value - The _file-format_ option.
   * @throws Throws {@link !Error} if value is an invalid option.
   *
   * @example
   * ```javascript
   * // Set the 'file-format' option.
   * opt.fileFormat = "unix";
   * ```
   */
  set fileFormat(value: opt.FileFormatOption) {
    assert(
      ["dos", "unix", "mac"].includes(value),
      `opt.fileFormat: value is invalid: ${value}`,
    );
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
   * const value = opt.fixEndOfLine;
   * ```
   */
  get fixEndOfLine(): boolean {
    // @ts-ignore Ignore warning
    return __InternalRsvimGlobalObject.opt_get_fix_end_of_line();
  }

  /**
   * Set the _fix-end-of-line_ option.
   *
   * @param {boolean} value - The _fix-end-of-line_ option.
   * @throws Throws {@link !Error} if value is not a boolean.
   *
   * @example
   * ```javascript
   * // Set the 'fix-end-of-line' option.
   * opt.fixEndOfLine = false;
   * ```
   */
  set fixEndOfLine(value: boolean) {
    assert(
      isBoolean(value),
      `opt.fixEndOfLine: value must be a boolean, got ${typeof value}`,
    );
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
   * const value = opt.lineBreak;
   * ```
   */
  get lineBreak(): boolean {
    // @ts-ignore Ignore warning
    return __InternalRsvimGlobalObject.opt_get_line_break();
  }

  /**
   * Set the _line-break_ option.
   *
   * @param {boolean} value - The _line-break_ option.
   * @throws Throws {@link !Error} if value is not a boolean.
   *
   * @example
   * ```javascript
   * // Set the 'lineBreak' option.
   * opt.lineBreak = true;
   * ```
   */
  set lineBreak(value: boolean) {
    assert(
      isBoolean(value),
      `opt.lineBreak: value must be a boolean, got ${typeof value}`,
    );
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
   * const value = opt.shiftWidth;
   * ```
   */
  get shiftWidth(): number {
    // @ts-ignore Ignore warning
    return __InternalRsvimGlobalObject.opt_get_shift_width();
  }

  /**
   * Set the _shift-width_ option. It only accepts an integer between `[1,255]`, if the value is out of range, it will be bound to this range.
   *
   *
   * @param {number} value - The _shift-width_ option.
   * @throws Throws {@link !Error} if value is not an integer.
   *
   * @example
   * ```javascript
   * // Set the 'shift-width' option.
   * opt.shiftWidth = 4;
   * ```
   */
  set shiftWidth(value: number) {
    assert(
      isNumber(value),
      `opt.shiftWidth: value must be a number, got ${typeof value}`,
    );
    assert(
      Number.isInteger(value),
      `opt.shiftWidth: value must be an integer, got ${typeof value}`,
    );

    // Bound the value into the range of `[1, 255]`.
    if (value < 1) {
      value = 1;
    }
    if (value > 255) {
      value = 255;
    }

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
   * const value = opt.syntaxParserLibPath;
   * ```
   */
  get syntaxParserLibPath(): string {
    // @ts-ignore Ignore warning
    return __InternalRsvimGlobalObject.opt_get_syntax_parser_lib_path();
  }

  /**
   * Set the _syntax-parser-lib-path_ option. It only accepts a string which represents the file path on your local machine, which is used to save all compiled tree-sitter parser dynamic libraries.
   *
   * @param {string} value - The _syntax-parser-lib-path_ option.
   * @throws Throws {@link !Error} if value is not a string.
   *
   * @example
   * ```javascript
   * // Set the 'syntax-parser-lib-path' option.
   * opt.syntaxParserLibPath = ".";
   * ```
   */
  set syntaxParserLibPath(value: string) {
    assert(
      isString(value),
      `opt.syntaxParserLibPath: value must be a string, got ${typeof value}`,
    );
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
   * const value = opt.tabStop;
   * ```
   */
  get tabStop(): number {
    // @ts-ignore Ignore warning
    return __InternalRsvimGlobalObject.opt_get_tab_stop();
  }

  /**
   * Set the _tab-stop_ option. It only accepts an integer between `[1,255]`, if the value is out of range, it will be bound to this range.
   *
   * @param {number} value - The _tab-stop_ option.
   * @throws Throws {@link !Error} if value is not an integer.
   *
   * @example
   * ```javascript
   * // Set the 'tab-stop' option.
   * opt.tabStop = 4;
   * ```
   */
  set tabStop(value: number) {
    assert(
      isNumber(value),
      `opt.tabStop: value must be a number, got ${typeof value}`,
    );
    assert(
      Number.isInteger(value),
      `opt.tabStop: value must be an integer, got ${typeof value}`,
    );

    // Bound the value into the range of `[1, 255]`.
    if (value < 1) {
      value = 1;
    }
    if (value > 255) {
      value = 255;
    }

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
   * const value = opt.wrap;
   * ```
   */
  get wrap(): boolean {
    // @ts-ignore Ignore warning
    return __InternalRsvimGlobalObject.opt_get_wrap();
  }

  /**
   * Set the _wrap_ option.
   *
   * @param {boolean} value - The _wrap_ option.
   * @throws Throws {@link !Error} if value is not a boolean.
   *
   * @example
   * ```javascript
   * // Set the 'wrap' option.
   * opt.wrap = true;
   * ```
   */
  set wrap(value: boolean) {
    assert(
      isBoolean(value),
      `opt.wrap: value must be a boolean, got ${typeof value}`,
    );
    // @ts-ignore Ignore warning
    __InternalRsvimGlobalObject.opt_set_wrap(value);
  }
}

/**
 * The `opt` module singleton, use it to read or write editor options.
 *
 * @example
 * ```javascript
 * import { opt } from "rsvim:ext/opt";
 *
 * opt.wrap = true;
 * ```
 *
 * @category Editor APIs
 */
export const opt = new Opt();
