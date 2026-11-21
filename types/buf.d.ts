export declare namespace buf {
    function current(): number | undefined;
    function list(): number[];
    function writeSync(bufId: number): number;
}
export default buf;
