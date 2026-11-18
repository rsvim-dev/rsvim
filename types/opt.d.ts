export declare namespace opt {
    type FileEncodingOption = "utf-8";
    type FileFormatOption = "dos" | "unix" | "mac";
}
export declare class Opt {
    get expandTab(): boolean;
    set expandTab(value: boolean);
    get fileEncoding(): opt.FileEncodingOption;
    set fileEncoding(value: opt.FileEncodingOption);
    get fileFormat(): opt.FileFormatOption;
    set fileFormat(value: opt.FileFormatOption);
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
export declare const opt: Opt;
