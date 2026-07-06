import { bundle } from "./tools/bundle.mjs";
import { bundle_css } from "./tools/css.mjs";
import { node_modules_external } from "./tools/externals.mjs";

import fs from "fs";
import cpy from "cpy";

const BUNDLES = [
  {
    entryPoints: ["src/ts/index.ts"],
    plugins: [node_modules_external()],
    outfile: "dist/esm/index.js",
  },
  {
    entryPoints: ["src/ts/index.ts"],
    outfile: "dist/cdn/index.js",
  },
];

async function build() {
  // Bundle css
  await bundle_css();

  // Copy HTML
  await cpy("src/html/*", "dist/");

  // Copy images
  if (fs.existsSync("src/img")) {
    fs.mkdirSync("dist/img", { recursive: true });
    await cpy("src/img/*", "dist/img");
  }

  await Promise.all(BUNDLES.map(bundle)).catch(() => process.exit(1));

  // wasm-bindgen (--target web) resolves the wasm binary relative to the
  // bundle's own URL (`new URL("dateme_bg.wasm", import.meta.url)`), so place a
  // copy next to each emitted bundle.
  for (const dir of ["dist/cdn", "dist/esm"]) {
    fs.mkdirSync(dir, { recursive: true });
    fs.copyFileSync("dist/pkg/dateme_bg.wasm", `${dir}/dateme_bg.wasm`);
  }

  // Copy servable assets to python extension (exclude esm/)
  fs.rmSync("../dateme/extension", {
    recursive: true,
    force: true,
  });
  fs.mkdirSync("../dateme/extension", { recursive: true });
  await cpy("dist/**/*", "../dateme/extension", {
    filter: (file) =>
      !file.relativePath.startsWith("esm/") &&
      !file.relativePath.startsWith("dist/esm/"),
  });
}

build();
