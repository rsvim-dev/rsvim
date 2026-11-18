export declare namespace syn {
    function loadParser(options: syn.LoadParserOptions): Promise<string[]>;
    function loadParserSync(options: syn.LoadParserOptions): string[];
    function listParsers(): string[];
    function getParserMetadata(name: string): syn.ParserMetadata | undefined;
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
