/**
 * ---
 * title: Utilities
 * sidebar_position: 4
 * ---
 *
 * Internal utility functions shared by the `Rsvim` and Web APIs. These are not
 * exposed on `globalThis`.
 *
 * @packageDocumentation
 */

/** @hidden */
export function assertTrue(condition: boolean, message: string) {
  if (!condition) {
    throw new Error(message);
  }
}

/** @hidden */
export function assertFalse(condition: boolean, message: string) {
  if (!!condition) {
    throw new Error(message);
  }
}

/** @hidden */
export function isNull(arg: any): boolean {
  return arg === undefined || arg === null;
}

/** @hidden */
export function isString(arg: any): boolean {
  return typeof arg === "string";
}

/** @hidden */
export function checkNotNull(arg: any, msg: string) {
  if (isNull(arg)) {
    throw new TypeError(`${msg} cannot be undefined or null`);
  }
}

/** @hidden */
export function checkIsNumber(arg: any, msg: string) {
  if (typeof arg !== "number") {
    throw new TypeError(`${msg} must be a number, but found ${typeof arg}`);
  }
}

/** @hidden */
export function checkIsInteger(arg: any, msg: string) {
  checkIsNumber(arg, msg);
  if (!Number.isInteger(arg)) {
    throw new TypeError(`${msg} must be an integer, but found ${typeof arg}`);
  }
}

/** @hidden */
export function checkIsBoolean(arg: any, msg: string) {
  if (typeof arg !== "boolean") {
    throw new TypeError(`${msg} must be a boolean, but found ${typeof arg}`);
  }
}

/** @hidden */
export function checkIsString(arg: any, msg: string) {
  if (!isString(arg)) {
    throw new TypeError(`${msg} must be a string, but found ${typeof arg}`);
  }
}

/** @hidden */
export function checkOptionalString(arg: any, msg: string) {
  if (!(isString(arg) || isNull(arg))) {
    throw new TypeError(`${msg} must be a string?, but found ${typeof arg}`);
  }
}

/** @hidden */
export function checkMatchPattern(arg: any, pat: RegExp, msg: string) {
  checkIsString(arg, msg);
  if (!pat.test(arg)) {
    throw new Error(`${msg} is invalid pattern: ${arg}"`);
  }
}

/** @hidden */
export function checkIsFunction(arg: any, msg: string) {
  if (typeof arg !== "function") {
    throw new TypeError(`${msg} must be a function, but found ${typeof arg}`);
  }
}

/** @hidden */
export function checkIsArray(arg: any, msg: string) {
  if (!Array.isArray(arg)) {
    throw new TypeError(`${msg} must be an array, but found ${typeof arg}`);
  }
}

/** @hidden */
export function checkIsObject(arg: any, msg: string) {
  if (typeof arg !== "object") {
    throw new TypeError(`${msg} must be an object, but found ${typeof arg}`);
  }
}

/** @hidden */
export function checkIsUint8Array(arg: any, msg: string) {
  if (!(arg instanceof Uint8Array)) {
    throw new TypeError(`${msg} must be a Uint8Array, buf found ${typeof arg}`);
  }
}

/** @hidden */
export function isTypedArray(arg: any): boolean {
  return (
    arg instanceof Int8Array ||
    arg instanceof Uint8Array ||
    arg instanceof Uint8ClampedArray ||
    arg instanceof Int16Array ||
    arg instanceof Uint16Array ||
    arg instanceof Int32Array ||
    arg instanceof Uint32Array ||
    arg instanceof Float32Array ||
    arg instanceof Float64Array ||
    arg instanceof BigInt64Array ||
    arg instanceof BigUint64Array
  );
}

/** @hidden */
export function isArrayBuffer(arg: any): boolean {
  return arg instanceof ArrayBuffer;
}

/** @hidden */
export function isDataView(arg: any): boolean {
  return arg instanceof DataView;
}

/** @hidden */
export function checkIsArrayBufferFamily(arg: any, msg: string) {
  if (!(isArrayBuffer(arg) || isDataView(arg) || isTypedArray(arg))) {
    throw new TypeError(
      `${msg} must be either ArrayBuffer/DataView/TypedArray, buf found ${typeof arg}`,
    );
  }
}

/** @hidden */
export function checkIsOptions(arg: any, options: any[], msg: string) {
  if (!options.includes(arg)) {
    throw new RangeError(`${msg} is an invalid option: ${arg}`);
  }
}

/**
 * @hidden
 *
 * Bound a number into the integer range of `[bound[0], bound[1]]`.
 *
 * The value is truncated to an integer first, e.g. `1.5` becomes `1`. A `NaN`
 * falls back to the lower bound, which matches the web standard behavior of
 * `setTimeout`/`setInterval`: a `NaN` delay is treated as `0`, then clamped up
 * to the minimum.
 */
export function boundByIntegers(arg: number, bound: [number, number]): number {
  if (Number.isNaN(arg)) {
    return bound[0];
  }

  const integer = Math.trunc(arg);
  if (integer < bound[0]) {
    return bound[0];
  }
  if (integer > bound[1]) {
    return bound[1];
  }
  return integer;
}

/** @hidden */
export function setDefaultFields(arg: object, defaults: object) {
  for (const [key, val] of Object.entries(defaults)) {
    if (!Object.hasOwn(arg, key)) {
      Object.defineProperty(arg, key, { value: val, writable: true });
    }
  }
}
