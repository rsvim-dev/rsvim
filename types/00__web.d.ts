/**
 * ---
 * title: Web API
 * sidebar_position: 3
 * ---
 *
 * The [globalThis](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/globalThis) global object, compatible with [WinterTC](https://min-common-api.proposal.wintertc.org/) web platform APIs.
 *
 * @see [MDN - Web APIs](https://developer.mozilla.org/docs/Web/API).
 *
 * @packageDocumentation
 */
/**
 * Encode string text into bytes, it only supports "utf-8" encoding.
 *
 * @see {@link !TextEncoder}
 */
export declare class TextEncoder {
    /**
     * @example
     * ```javascript
     * const encoder = new TextEncoder();
     * ```
     */
    constructor();
    /**
     * Encode string text to {@link !Uint8Array}.
     *
     * @example
     * ```javascript
     * const encodedBytes = new TextEncoder().encode("Hello, World!");
     * ```
     *
     * @param {string} input - Text that need encode.
     * @returns {Uint8Array} Encoded uint8 bytes array.
     * @throws Throws {@link !TypeError} if input is not a string.
     */
    encode(input: string): Uint8Array;
    /**
     * Encode string text into {@link !Uint8Array}.
     *
     * @param {string} src - Text that need encode.
     * @param {Uint8Array} dest - Destination that receives the encoded uint8 bytes array.
     * @returns {TextEncoder.EncodeIntoResult} Encode result, it contains two numbers: the "read" Unicode code units from src string, and the "written" UTF-8 bytes into the dest buffer.
     * @throws Throws {@link !TypeError} if src is not a string, or dest is not a {@link !Uint8Array}.
     */
    encodeInto(src: string, dest: Uint8Array): TextEncoder.EncodeIntoResult;
    /**
     * The encoding used by encoder, this always returns "utf-8".
     */
    get encoding(): string;
}
export declare namespace TextEncoder {
    /**
     * @see {@link TextEncoder}
     * @inline
     */
    type EncodeIntoResult = {
        read: number;
        written: number;
    };
}
/**
 * Decode bytes array into string text.
 *
 * @see {@link !TextDecoder}
 */
export declare class TextDecoder {
    #private;
    /**
     * Create a TextDecoder instance with specified encoding.
     *
     * Per the [WHATWG Encoding Standard](https://encoding.spec.whatwg.org/), the encodings supported by the TextDecoder API are outlined in the tables below. For each encoding, one or more aliases may be used.
     *
     * | Encoding | Aliases |
     * |----------|---------|
     * |'ibm866'  | '866', 'cp866', 'csibm866' |
     * |'iso-8859-2' | 'csisolatin2', 'iso-ir-101', 'iso8859-2', 'iso88592', 'iso_8859-2', 'iso_8859-2:1987', 'l2', 'latin2' |
     * |'iso-8859-3' | 'csisolatin3', 'iso-ir-109', 'iso8859-3', 'iso88593', 'iso_8859-3', 'iso_8859-3:1988', 'l3', 'latin3' |
     * | 'iso-8859-4' | 'csisolatin4', 'iso-ir-110', 'iso8859-4', 'iso88594', 'iso_8859-4', 'iso_8859-4:1988', 'l4', 'latin4' |
     * | 'iso-8859-5' | 'csisolatincyrillic', 'cyrillic', 'iso-ir-144', 'iso8859-5', 'iso88595', 'iso_8859-5', 'iso_8859-5:1988' |
     * | 'iso-8859-6' | 'arabic', 'asmo-708', 'csiso88596e', 'csiso88596i', 'csisolatinarabic', 'ecma-114', 'iso-8859-6-e', 'iso-8859-6-i', 'iso-ir-127', 'iso8859-6', 'iso88596', 'iso_8859-6', 'iso_8859-6:1987' |
     * | 'iso-8859-7' | 'csisolatingreek', 'ecma-118', 'elot_928', 'greek', 'greek8', 'iso-ir-126', 'iso8859-7', 'iso88597', 'iso_8859-7', 'iso_8859-7:1987', 'sun_eu_greek' |
     * | 'iso-8859-8' | 'csiso88598e', 'csisolatinhebrew', 'hebrew', 'iso-8859-8-e', 'iso-ir-138', 'iso8859-8', 'iso88598', 'iso_8859-8', 'iso_8859-8:1988', 'visual' |
     * | 'iso-8859-8-i' | 'csiso88598i', 'logical' |
     * | 'iso-8859-10' | 'csisolatin6', 'iso-ir-157', 'iso8859-10', 'iso885910', 'l6', 'latin6' |
     * | 'iso-8859-13' | 'iso8859-13', 'iso885913' |
     * | 'iso-8859-14' | 'iso8859-14', 'iso885914' |
     * | 'iso-8859-15' | 'csisolatin9', 'iso8859-15', 'iso885915', 'iso_8859-15', 'l9' |
     * | 'koi8-r' | 'cskoi8r', 'koi', 'koi8', 'koi8_r' |
     * | 'koi8-u' | 'koi8-ru' |
     * | 'macintosh' | 'csmacintosh', 'mac', 'x-mac-roman' |
     * | 'windows-874' | 'dos-874', 'iso-8859-11', 'iso8859-11', 'iso885911', 'tis-620' |
     * | 'windows-1250' | 'cp1250', 'x-cp1250' |
     * | 'windows-1251' | 'cp1251', 'x-cp1251' |
     * | 'windows-1252' | 'ansi_x3.4-1968', 'ascii', 'cp1252', 'cp819', 'csisolatin1', 'ibm819', 'iso-8859-1', 'iso-ir-100', 'iso8859-1', 'iso88591', 'iso_8859-1', 'iso_8859-1:1987', 'l1', 'latin1', 'us-ascii', 'x-cp1252' |
     * | 'windows-1253' | 'cp1253', 'x-cp1253' |
     * | 'windows-1254' | 'cp1254', 'csisolatin5', 'iso-8859-9', 'iso-ir-148', 'iso8859-9', 'iso88599', 'iso_8859-9', 'iso_8859-9:1989', 'l5', 'latin5', 'x-cp1254' |
     * | 'windows-1255' | 'cp1255', 'x-cp1255' |
     * | 'windows-1256' | 'cp1256', 'x-cp1256' |
     * | 'windows-1257' | 'cp1257', 'x-cp1257' |
     * | 'windows-1258' | 'cp1258', 'x-cp1258' |
     * | 'x-mac-cyrillic' | 'x-mac-ukrainian' |
     * | 'gbk' | 'chinese', 'csgb2312', 'csiso58gb231280', 'gb2312', 'gb_2312', 'gb_2312-80', 'iso-ir-58', 'x-gbk'
     * | 'gb18030' | |
     * | 'big5' | 'big5-hkscs', 'cn-big5', 'csbig5', 'x-x-big5' |
     * | 'euc-jp' | 'cseucpkdfmtjapanese', 'x-euc-jp' |
     * | 'iso-2022-jp' | 'csiso2022jp' |
     * | 'shift_jis' | 'csshiftjis', 'ms932', 'ms_kanji', 'shift-jis', 'sjis', 'windows-31j', 'x-sjis' |
     * | 'euc-kr' | 'cseuckr', 'csksc56011987', 'iso-ir-149', 'korean', 'ks_c_5601-1987', 'ks_c_5601-1989', 'ksc5601', 'ksc_5601', 'windows-949' |
     *
     * @example
     * ```javascript
     * const bytes = new Uint8Array([
     *   0xf0, 0x9d, 0x93, 0xbd,
     *   0xf0, 0x9d, 0x93, 0xae,
     *   0xf0, 0x9d, 0x94, 0x81,
     *   0xf0, 0x9d, 0x93, 0xbd
     * ]);
     * if (new TextDecoder().decode(bytes) !== "𝓽𝓮𝔁𝓽") {
     *   Rsvim.cmd.echo("Failed to decode");
     * }
     * ```
     *
     * @see [Node.js - WHATWG supported encodings](https://nodejs.org/api/util.html#whatwg-supported-encodings)
     * @see [encoding_rs - Relationship with Windows Code Pages](https://docs.rs/encoding_rs/latest/encoding_rs/#relationship-with-windows-code-pages)
     * @see [encoding_rs - Supported Encodings](https://docs.rs/encoding_rs/latest/encoding_rs/#statics)
     *
     * @param {string} encoding - (Optional) Decoder encoding, by default is "utf-8".
     * @param {TextDecoder.Options} options - (Optional) Decode options, by default is `{fatal: false, ignoreBOM: false}`.
     * @throws Throws {@link !TypeError} if encoding is not a string or options is invalid. Throw {@link !RangeError} if encoding is unknown or not support.
     */
    constructor(encoding?: string, options?: TextDecoder.Options);
    /**
     * Decode a bytes array to string text. The bytes array can be a {@link !ArrayBuffer}, {@link !TypedArray} or {@link !DataView}.
     *
     * @example
     * ```javascript
     * // Single pass, non-stream
     * const str1 = new TextDecoder().decode(new Uint8Array([1,2,3,4]));
     *
     * // Stream
     * const decoder = new TextDecoder();
     * let str2 = "";
     * str2 += decoder.decode(new Uint8Array([1]), {stream: true});
     * str2 += decoder.decode(new Uint8Array([2,3]), {stream: true});
     * str2 += decoder.decode(new Uint8Array([4]), {stream: true});
     * str2 += decoder.decode(undefined, {stream: false}); // Flush buffer and finish decoding.
     * ```
     *
     * @see {@link !TextDecoder}
     *
     * @param {(ArrayBuffer | TypedArray | DataView)} input - (Optional) Bytes array, by default is `new Uint8Array()`.
     * @param {TextDecoder.DecodeOptions} options - (Optional) Decode options, by default is `{stream: false}`. When decode a stream data (e.g. read from tcp network) while reading it and cannot determine the end of bytes, should set `stream` option to `true`.
     * @returns {string} Decoded string text.
     * @throws Throws {@link !TypeError} if input is not a Uint8Array, or options is invalid, or the data is malformed and `fatal` option is set.
     */
    decode(input?: ArrayBuffer | TypedArray | DataView, options?: TextDecoder.DecodeOptions): string;
    /**
     * The encoding used by decoder.
     */
    get encoding(): string;
    /**
     * Whether throw {@link !TypeError} when decoding error because the data is malformed.
     */
    get fatal(): boolean;
    /**
     * Whether ignore unicode "Byte-Order-Mark" (BOM) when decoding the data.
     */
    get ignoreBOM(): boolean;
}
export declare namespace TextDecoder {
    /**
     * @see {@link TextDecoder}
     * @inline
     */
    type Options = {
        fatal?: boolean;
        ignoreBOM?: boolean;
    };
    /**
     * @see {@link TextDecoder}
     * @inline
     */
    type DecodeOptions = {
        stream?: boolean;
    };
}
/**
 * {@link !TypedArray}
 */
export type TypedArray = Int8Array | Uint8Array | Uint8ClampedArray | Int16Array | Uint16Array | Int32Array | Uint32Array | Float32Array | Float64Array | BigInt64Array | BigUint64Array;
/**
 * Cancel a repeated timer previously established by calling {@link setInterval}.
 *
 * @param {number} id - The ID (integer) which identifies the schedule.
 * @throws Throws {@link !TypeError} if ID is not an integer.
 */
export declare function clearInterval(id: number): void;
/**
 * Set a repeated timer that calls a function, with a fixed time delay between each call.
 *
 * @param {function} callback - A function to be executed every `delay` milliseconds.
 * @param {number} delay - (Optional) The milliseconds that the timer should delay in between execution of the function, by default is `1`.
 * @param {...any} args - (Optional) Additional arguments which are passed through to the function.
 * @returns {number} The ID (integer) which identifies the timer created.
 * @throws Throws {@link !TypeError} if callback is not a function, or delay is neither a number or undefined.
 */
export declare function setInterval(callback: (...args: any[]) => void, delay?: number, ...args: any[]): number;
/**
 * Cancel a timeout previously established by calling {@link setTimeout}.
 *
 * @param {number} id - The ID (integer) which identifies the timer.
 * @throws Throws {@link !TypeError} if ID is not an integer.
 */
export declare function clearTimeout(id: number): void;
/**
 * Set a timer which executes a function or specified piece of code once the timer expires.
 *
 * @param {function} callback - A function to be executed after the timer expires.
 * @param {number} delay - (Optional) The milliseconds that the timer should wait before the function is executed, by default is `1`.
 * @param {...any} args - (Optional) Additional arguments which are passed through to the function.
 * @returns {number} The ID (integer) which identifies the timer created.
 * @throws Throws {@link !TypeError} if callback is not a function, or delay is neither a number or undefined.
 */
export declare function setTimeout(callback: (...args: any[]) => void, delay: number, ...args: any[]): number;
/**
 * A microtask is a short function which is executed after the function or module which created it exits and
 * only if the JavaScript execution stack is empty, but before returning control to the event loop being used
 * to drive the script's execution environment.
 *
 * @param {function} callback - A function to be executed.
 * @throws Throws {@link !TypeError} if callback is not a function.
 */
export declare function queueMicrotask(callback: () => void): void;
/**
 * Dispatch an uncaught exception. Similar to synchronous version of `setTimeout(() => {throw error;}, 0);`.
 *
 * @param {any} error - Anything to be thrown.
 */
export declare function reportError(error: any): void;
declare global {
    var TextEncoder: typeof TextEncoder;
    var TextDecoder: typeof TextDecoder;
    function clearInterval(id: number): void;
    function setInterval(callback: (...args: any[]) => void, delay?: number, ...args: any[]): number;
    function clearTimeout(id: number): void;
    function setTimeout(callback: (...args: any[]) => void, delay: number, ...args: any[]): number;
    function queueMicrotask(callback: () => void): void;
    function reportError(error: any): void;
}
