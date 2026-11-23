// Rollup configuration for bundling the JavaScript runtime.
//
// The `.js` files under `./build/` are compiled from `./src/*.ts` by `tsc`, see
// `tsconfig.json`. Rollup bundles them into `./build/*.bundle.js` and
// tree-shakes unused code. Keep the "es" format, since the js runtime loads
// them as v8 modules.
//
// The js runtime has no module resolution for npm packages: it only knows the
// builtin modules embedded by `rsvim_core/src/js.rs`. Any npm dependency must
// therefore be inlined into the bundle, which is what `node-resolve` does.
//
// Every builtin module is bundled separately, because the runtime loads each of
// them as a standalone v8 module. The imports between the builtin modules are
// rewritten to the `rsvim:<name>` scheme and kept external, so
// they are resolved by the runtime's core module loader instead of being
// inlined, see `rsvim_core/src/js/loader/core_loader.rs`.
//
// Usage: npx rollup -c

import { nodeResolve } from "@rollup/plugin-node-resolve";

// Builtin modules that are imported by other builtin modules.
const CORE_MODULES = ["cmd", "fs", "opt", "process", "syn", "buf"];

// Entry modules, `00__web` and the core modules above.
const ENTRIES = ["00__web", ...CORE_MODULES];

const moduleName = (id) => id.split("/").pop().replace(/\.js$/, "");

// Rewrites imports between builtin modules to the `rsvim:<name>` scheme.
const coreModules = {
  name: "core-modules",
  resolveId(source, importer) {
    // Entry modules themselves are never external.
    if (!importer) {
      return null;
    }
    const name = moduleName(source);
    if (!CORE_MODULES.includes(name)) {
      return null;
    }
    return { id: `rsvim:${name}`, external: true };
  },
};

/** @type {import("rollup").RollupOptions[]} */
export default ENTRIES.map((name) => ({
  input: `./build/${name}.js`,
  output: {
    file: `./build/${name}.bundle.js`,
    format: "es",
  },
  plugins: [coreModules, nodeResolve()],
}));
