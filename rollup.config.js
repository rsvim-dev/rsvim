// Rollup configuration for bundling the JavaScript runtime.
//
// The `.js` files under `./build/` are compiled from `./src/*.ts` by `tsc`, see
// `tsconfig.json`. Rollup bundles them into `./build/*.bundle.js` and
// tree-shakes unused code. Keep the "es" format, since the js runtime loads
// them as v8 modules.
//
// The js runtime has no module resolution of its own: it only knows the two
// builtin modules embedded by `rsvim_core/src/js.rs`. Any npm dependency must
// therefore be inlined into the bundle, which is what `node-resolve` does.
//
// Usage: npx rollup -c

import { nodeResolve } from "@rollup/plugin-node-resolve";

/** @type {import("rollup").RollupOptions[]} */
export default [
  {
    input: "./build/00__web.js",
    output: {
      file: "./build/00__web.bundle.js",
      format: "es",
    },
    plugins: [nodeResolve()],
  },
  {
    input: "./build/01__rsvim.js",
    output: {
      file: "./build/01__rsvim.bundle.js",
      format: "es",
    },
    plugins: [nodeResolve()],
  },
];
