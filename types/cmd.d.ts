export declare namespace cmd {
    function create(name: string, callback: cmd.CommandCallback, attributes?: cmd.CommandAttributes, options?: cmd.CommandOptions): cmd.CommandDefinition | undefined;
    function echo(message: any): void;
    function list(): string[];
    function get(name: string): cmd.CommandDefinition | undefined;
    function remove(name: string): cmd.CommandDefinition | undefined;
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
export default cmd;
