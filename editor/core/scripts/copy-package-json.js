import { readFileSync, writeFileSync } from "fs";
import { resolve } from "path";

const pkg = JSON.parse(readFileSync(resolve("package.json"), "utf-8"));
pkg.main = "./core.umd.cjs";
pkg.module = "./core.js";
pkg.types = "./index.d.ts";
writeFileSync(resolve("dist/package.json"), JSON.stringify(pkg, null, 2));