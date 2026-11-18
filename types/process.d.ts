export declare namespace process {
    class Command {
        #private;
        constructor(execPath: string, options?: process.CommandOptions);
        get execPath(): string;
        get options(): process.CommandOptions;
        spawn(): process.ChildProcess;
    }
    class ChildProcess {
        #private;
        constructor(execPath: string, options: process.CommandOptions, rid: number, stdinRid: number | null | undefined, stdoutRid: number | null | undefined, stderrRid: number | null | undefined);
        get execPath(): string;
        get options(): process.CommandOptions;
        get stdout(): process.ChildProcessReadableStream | null | undefined;
        get stderr(): process.ChildProcessReadableStream | null | undefined;
        get isDisposed(): boolean;
        [Symbol.asyncDispose](): Promise<void>;
        wait(): Promise<process.ChildProcessExitStatus>;
        get exitStatus(): process.ChildProcessExitStatus | null | undefined;
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
    function exit(exitCode?: number): void;
}
