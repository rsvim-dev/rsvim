export declare class TextEncoder {
    constructor();
    encode(input: string): Uint8Array;
    encodeInto(src: string, dest: Uint8Array): TextEncoder.EncodeIntoResult;
    get encoding(): string;
}
export declare namespace TextEncoder {
    type EncodeIntoResult = {
        read: number;
        written: number;
    };
}
export declare class TextDecoder {
    #private;
    constructor(encoding?: string, options?: TextDecoder.Options);
    decode(input?: ArrayBuffer | TypedArray | DataView, options?: TextDecoder.DecodeOptions): string;
    get encoding(): string;
    get fatal(): boolean;
    get ignoreBOM(): boolean;
}
export declare namespace TextDecoder {
    type Options = {
        fatal?: boolean;
        ignoreBOM?: boolean;
    };
    type DecodeOptions = {
        stream?: boolean;
    };
}
export type TypedArray = Int8Array | Uint8Array | Uint8ClampedArray | Int16Array | Uint16Array | Int32Array | Uint32Array | Float32Array | Float64Array | BigInt64Array | BigUint64Array;
export declare function clearInterval(id: number): void;
export declare function setInterval(callback: (...args: any[]) => void, delay?: number, ...args: any[]): number;
export declare function clearTimeout(id: number): void;
export declare function setTimeout(callback: (...args: any[]) => void, delay: number, ...args: any[]): number;
export declare function queueMicrotask(callback: () => void): void;
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
