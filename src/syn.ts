import { isString } from "es-toolkit/predicate/isString";
import { assert } from "es-toolkit/util";

/**
 * ---
 * title: syn API
 * sidebar_position: 9
 * ---
 *
 * The `syn` module for tree-sitter syntax highlighting.
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
 * The `syn` module for tree-sitter syntax highlighting.
 *
 * @example
 * ```javascript
 * import { syn } from "rsvim:ext/syn";
 *
 * await syn.loadParser({ grammarPath: "./tree-sitter-c" });
 * ```
 *
 * @category Editor APIs
 */
export namespace syn {
  /**
   * Load tree-sitter parsers.
   *
   * @see [tree-sitter - List of parsers](https://github.com/tree-sitter/tree-sitter/wiki/List-of-parsers)
   *
   * @param {syn.LoadParserOptions} options - Load options.
   *
   * @returns {string[]} It returns all the loaded parser names.
   *
   * @throws Throws {@link !Error} if `options` is an invalid option, or failed to load.
   *
   * @example
   * ```javascript
   * // Load `tree-sitter-c` parser.
   * const parserNames = await syn.loadParser({grammarPath: "./tree-sitter-c"});
   * cmd.echo(`Loaded parsers: ${parserNames}`);
   * ```
   */
  export async function loadParser(
    options: syn.LoadParserOptions,
  ): Promise<string[]> {
    assert(
      typeof options === "object",
      `syn.loadParser: options must be an object, got ${typeof options}`,
    );
    assert(
      isString(options.grammarPath),
      `syn.loadParser: grammarPath option must be a string, got ${typeof options.grammarPath}`,
    );
    const parserNames =
      // @ts-ignore Ignore warning
      await __InternalRsvimGlobalObject.syn_load_parser(options);
    return parserNames;
  }

  /**
   * Load tree-sitter parsers synchronizely.
   *
   * @see {@link loadParser}
   *
   * @param {syn.LoadParserOptions} options - Load options.
   *
   * @returns {string[]} It returns all the loaded parser names.
   *
   * @throws Throws {@link !Error} if `options` is an invalid option, or failed to load.
   *
   * @example
   * ```javascript
   * // Load `tree-sitter-c` parser synchronizely.
   * const parserNames = syn.loadParserSync({grammarPath: "./tree-sitter-c"});
   * cmd.echo(`Loaded parsers: ${parserNames}`);
   * ```
   */
  export function loadParserSync(options: syn.LoadParserOptions): string[] {
    assert(
      typeof options === "object",
      `syn.loadParserSync: options must be an object, got ${typeof options}`,
    );
    assert(
      isString(options.grammarPath),
      `syn.loadParserSync: grammarPath option must be a string, got ${typeof options.grammarPath}`,
    );
    const parserNames =
      // @ts-ignore Ignore warning
      __InternalRsvimGlobalObject.syn_load_parser_sync(options);
    return parserNames;
  }

  /**
   * List all loaded tree-sitter parsers.
   *
   * @returns {string[]} It returns all the loaded parser names.
   *
   * @example
   * ```javascript
   * // Print all loaded parser names.
   * const allParserNames = syn.listParsers();
   * cmd.echo(`All loaded parsers: ${allParserNames}`);
   * ```
   */
  export function listParsers(): string[] {
    // @ts-ignore Ignore warning
    return __InternalRsvimGlobalObject.syn_list_parsers();
  }

  /**
   * Get tree-sitter parser metadata by parser name.
   *
   * @param {string} name - The parser's name.
   *
   * @returns {syn.ParserMetadata | undefined} It returns all the loaded parser names.
   *
   * @example
   * ```javascript
   * // Get parser metadata by name.
   * const parserMetadata = syn.getParserMetadata("rust");
   * cmd.echo(`Rust parser metadata: ${parserMetadata}`);
   * ```
   */
  export function getParserMetadata(
    name: string,
  ): syn.ParserMetadata | undefined {
    assert(
      isString(name),
      `syn.getParserMetadata: name must be a string, got ${typeof name}`,
    );

    // @ts-ignore Ignore warning
    return __InternalRsvimGlobalObject.syn_get_parser_metadata(name);
  }

  /**
   * Options to load a tree-sitter parser.
   *
   * @see {@link syn.loadParser}
   */
  export type LoadParserOptions = {
    /**
     * The tree-sitter parser path to load.
     */
    grammarPath: string;
  };

  /**
   * Tree-sitter parser metadata.
   *
   * @see {@link syn.getParserMetadata}
   */
  export type ParserMetadata = {
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
