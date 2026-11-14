export declare namespace RsvimBuf {
    function current(): number | undefined;
    function list(): number[];
    function writeSync(bufId: number): number;
}
export declare namespace RsvimCmd {
    function create(name: string, callback: RsvimCmd.CommandCallback, attributes?: RsvimCmd.CommandAttributes, options?: RsvimCmd.CommandOptions): RsvimCmd.CommandDefinition | undefined;
    function echo(message: any): void;
    function list(): string[];
    function get(name: string): RsvimCmd.CommandDefinition | undefined;
    function remove(name: string): RsvimCmd.CommandDefinition | undefined;
    type CommandAttributes = {
        bang?: boolean;
        nargs?: "0" | "1" | "*" | "+" | "?";
    };
    type CommandOptions = {
        force?: boolean;
        alias?: string;
    };
    type CommandCallback = (ctx: CommandContext) => void | Promise<void>;
    type CommandDefinition = {
        name: string;
        callback: CommandCallback;
        attributes: CommandAttributes;
        options: CommandOptions;
    };
    type CommandContext = {
        bang: boolean;
        args: string[];
        currentBufferId: number;
        currentWindowId: number;
    };
}
export declare namespace RsvimFs {
    function open(path: string, options?: RsvimFs.OpenOptions): Promise<RsvimFs.File>;
    function openSync(path: string, options?: RsvimFs.OpenOptions): RsvimFs.File;
    function readDir(path: string): AsyncIterable<RsvimFs.DirEntry>;
    function readDirSync(path: string): Iterable<RsvimFs.DirEntry>;
    function readFile(path: string): Promise<Uint8Array>;
    function readFileSync(path: string): Uint8Array;
    function readTextFile(path: string): Promise<string>;
    function readTextFileSync(path: string): string;
    function lstat(path: string): Promise<RsvimFs.FileInfo>;
    function lstatSync(path: string): RsvimFs.FileInfo;
    function stat(path: string): Promise<RsvimFs.FileInfo>;
    function statSync(path: string): RsvimFs.FileInfo;
    function symlink(oldpath: string, newpath: string, options?: RsvimFs.SymlinkOptions): Promise<void>;
    function symlinkSync(oldpath: string, newpath: string, options?: RsvimFs.SymlinkOptions): void;
    function link(oldpath: string, newpath: string): Promise<void>;
    function linkSync(oldpath: string, newpath: string): void;
    function mkdir(path: string, options?: RsvimFs.MkdirOptions): Promise<void>;
    function mkdirSync(path: string, options?: RsvimFs.MkdirOptions): void;
    type OpenOptions = {
        append?: boolean;
        create?: boolean;
        createNew?: boolean;
        read?: boolean;
        truncate?: boolean;
        write?: boolean;
    };
    class File {
        #private;
        constructor(rid: number);
        close(): void;
        [Symbol.dispose](): void;
        get isDisposed(): boolean;
        read(buf: Uint8Array): Promise<number>;
        readSync(buf: Uint8Array): number;
        write(buf: Uint8Array): Promise<number>;
        writeSync(buf: Uint8Array): number;
    }
    type DirEntry = {
        fileName: string;
        isDir: boolean;
        isFile: boolean;
        isSymlink: boolean;
    };
    type FileInfo = {
        accessed?: Date;
        created?: Date;
        modified?: Date;
        isDir: boolean;
        isFile: boolean;
        isSymlink: boolean;
        len: number;
        readOnly: boolean;
        fileAttributes?: number;
        creationTime?: number;
        lastAccessTime?: number;
        lastWriteTime?: number;
        fileSize?: number;
        dev?: number;
        ino?: number;
        mode?: number;
        nlink?: number;
        uid?: number;
        gid?: number;
        rdev?: number;
        size?: number;
        atime?: number;
        atimeNsec?: number;
        mtime?: number;
        mtimeNsec?: number;
        ctime?: number;
        ctimeNsec?: number;
        blksize?: number;
        blocks?: number;
    };
    type SymlinkOptions = "file" | "dir" | "junction";
    type MkdirOptions = {
        recursive?: boolean;
        mode?: number;
    };
}
export declare namespace RsvimOpt {
    type FileEncodingOption = "utf-8";
    type FileFormatOption = "dos" | "unix" | "mac";
}
export declare class RsvimOpt {
    get expandTab(): boolean;
    set expandTab(value: boolean);
    get fileEncoding(): RsvimOpt.FileEncodingOption;
    set fileEncoding(value: RsvimOpt.FileEncodingOption);
    get fileFormat(): RsvimOpt.FileFormatOption;
    set fileFormat(value: RsvimOpt.FileFormatOption);
    get fixEndOfLine(): boolean;
    set fixEndOfLine(value: boolean);
    get lineBreak(): boolean;
    set lineBreak(value: boolean);
    get shiftWidth(): number;
    set shiftWidth(value: number);
    get syntaxParserLibPath(): string;
    set syntaxParserLibPath(value: string);
    get tabStop(): number;
    set tabStop(value: number);
    get wrap(): boolean;
    set wrap(value: boolean);
}
export declare namespace RsvimProc {
    class Command {
        #private;
        constructor(execPath: string, options?: RsvimProc.CommandOptions);
        get execPath(): string;
        get options(): RsvimProc.CommandOptions;
        spawn(): RsvimProc.ChildProcess;
    }
    class ChildProcess {
        #private;
        constructor(execPath: string, options: RsvimProc.CommandOptions, rid: number, stdinRid: number | null | undefined, stdoutRid: number | null | undefined, stderrRid: number | null | undefined);
        get execPath(): string;
        get options(): RsvimProc.CommandOptions;
        get stdout(): RsvimProc.ChildProcessReadableStream | null | undefined;
        get stderr(): RsvimProc.ChildProcessReadableStream | null | undefined;
        get isDisposed(): boolean;
        [Symbol.asyncDispose](): Promise<void>;
        wait(): Promise<RsvimProc.ChildProcessExitStatus>;
        get exitStatus(): RsvimProc.ChildProcessExitStatus | null | undefined;
    }
    class ChildProcessReadableStream {
        #private;
        constructor(rid: number);
        text(): Promise<string>;
    }
    type CommandOptions = {
        args?: string[];
        cwd?: string;
        clearEnv?: boolean;
        detached?: boolean;
        env?: Record<string, string | undefined | null>;
        stdin?: "piped" | "inherit" | "null";
        stdout?: "piped" | "inherit" | "null";
        stderr?: "piped" | "inherit" | "null";
    };
    type ChildProcessExitStatus = {
        success: boolean;
        exit?: number;
        signal?: number;
    };
}
export declare namespace RsvimRt {
    function exit(exitCode?: number): void;
}
export declare namespace RsvimSyn {
    function loadParser(options: RsvimSyn.LoadParserOptions): Promise<string[]>;
    function loadParserSync(options: RsvimSyn.LoadParserOptions): string[];
    function listParsers(): string[];
    function getParserMetadata(name: string): RsvimSyn.ParserMetadata | undefined;
    type LoadParserOptions = {
        grammarPath: string;
    };
    type ParserMetadata = {
        name: string;
        camelcase: string;
        scope: string;
        path: string;
        fileTypes: string[];
        highlightsPath?: string;
        highlightsQuery?: string;
        tagsPath?: string;
        tagsQuery?: string;
        injectionsPath?: string;
        injectionsQuery?: string;
        injectionRegex?: string;
    };
}
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
