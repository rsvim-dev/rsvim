export declare namespace fs {
    function open(path: string, options?: fs.OpenOptions): Promise<fs.File>;
    function openSync(path: string, options?: fs.OpenOptions): fs.File;
    function readDir(path: string): AsyncIterable<fs.DirEntry>;
    function readDirSync(path: string): Iterable<fs.DirEntry>;
    function readFile(path: string): Promise<Uint8Array>;
    function readFileSync(path: string): Uint8Array;
    function readTextFile(path: string): Promise<string>;
    function readTextFileSync(path: string): string;
    function lstat(path: string): Promise<fs.FileInfo>;
    function lstatSync(path: string): fs.FileInfo;
    function stat(path: string): Promise<fs.FileInfo>;
    function statSync(path: string): fs.FileInfo;
    function symlink(oldpath: string, newpath: string, options?: fs.SymlinkOptions): Promise<void>;
    function symlinkSync(oldpath: string, newpath: string, options?: fs.SymlinkOptions): void;
    function link(oldpath: string, newpath: string): Promise<void>;
    function linkSync(oldpath: string, newpath: string): void;
    function mkdir(path: string, options?: fs.MkdirOptions): Promise<void>;
    function mkdirSync(path: string, options?: fs.MkdirOptions): void;
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
