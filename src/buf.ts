/**
 * ---
 * title: buf API
 * sidebar_position: 7
 * ---
 *
 * The `buf` module for Vim buffers.
 *
 * @packageDocumentation
 *
 * @categoryDescription Editor APIs
 * These APIs are specific for editor, such as buffers, windows, key mappings, etc.
 *
 * @categoryDescription General APIs
 * These APIs are general for common javascript-based runtime, similar to [Deno APIs](https://docs.deno.com/api/deno/).
 */

import { isNumber } from "es-toolkit/predicate/isNumber";
import { assert } from "es-toolkit/util";

/**
 * The `buf` module for Vim buffers.
 *
 * @example
 * ```javascript
 * import buf from "rsvim:buf";
 *
 * const bufId = buf.current();
 * ```
 *
 * @category Editor APIs
 */
export namespace buf {
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
   * const bufId = buf.current();
   * ```
   */
  export function current(): number | undefined {
    // @ts-ignore Ignore warning
    return __InternalRsvimGlobalObject.buf_current();
  }

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
   * const bufIds = buf.list();
   * ```
   */
  export function list(): number[] {
    // @ts-ignore Ignore warning
    return __InternalRsvimGlobalObject.buf_list();
  }

  /**
   * Write (save) buffer's text contents to local filesystem synchronizely.
   *
   * @param {number} bufId - The buffer's ID that you want to write to filesystem.
   *
   * @returns {number} It returns a positive integer to indicate how many bytes
   * have been written to the file, if written successfully.
   *
   * @throws Throws {@link !Error} if the parameter is invalid, or failed to write buffer to the file system.
   *
   * @example
   * ```javascript
   * const bufId = buf.currentBufferId();
   * try {
   *   const bytes = buf.writeSync(bufId);
   *   cmd.echo(`Buffer ${bufId} has been saved, ${bytes} bytes written`);
   * } catch (e) {
   *   cmd.echo(`Error: failed to save buffer ${bufId}, exception: ${e}`);
   * }
   * ```
   */
  export function writeSync(bufId: number): number {
    assert(
      isNumber(bufId),
      `buf.writeSync: bufId must be a number, got ${typeof bufId}`,
    );
    assert(
      Number.isInteger(bufId),
      `buf.writeSync: bufId must be an integer, got ${typeof bufId}`,
    );

    // @ts-ignore Ignore warning
    return __InternalRsvimGlobalObject.buf_write_sync(bufId);
  }
}

export default buf;
