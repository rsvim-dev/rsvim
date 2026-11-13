// Rollup configuration for bundling the JavaScript runtime.
//
// The `.js` files under `./build/` are compiled from `./src/*.ts` by `tsc`, see
// `tsconfig.json`. Rollup bundles them into `./build/*.bundle.js` and
// tree-shakes unused code. Keep the "es" format, since the js runtime loads
// them as v8 modules.
//
// Usage: npx rollup -c

/** @type {import("rollup").RollupOptions[]} */
export default [
  {
    input: "./build/00__web.js",
    output: {
      file: "./build/00__web.bundle.js",
      format: "es",
    },
  },
  {
    input: "./build/01__rsvim.js",
    output: {
      file: "./build/01__rsvim.bundle.js",
      format: "es",
    },
  },
];
