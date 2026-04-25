#!/usr/bin/env node
// PLATEAU REST API の OpenAPI スナップショット (`src/openapi/plateau-api.json`) を、
// ローカルの真の出典 (`server/openapi/openapi.yml`) から再生成する。
//
// 本番反映を待たずに docs と OpenAPI スキーマを同期させたいときに使う。
// `api:fetch` の代替で、prod から取得する代わりにローカル yml を使う。

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parse as parseYaml } from "yaml";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");
const YAML_PATH = resolve(ROOT, "..", "server", "openapi", "openapi.yml");
const OUT_PATH = resolve(ROOT, "src", "openapi", "plateau-api.json");

const yamlText = readFileSync(YAML_PATH, "utf8");
const obj = parseYaml(yamlText);
writeFileSync(OUT_PATH, JSON.stringify(obj));
console.log(`[openapi] Wrote ${OUT_PATH} from ${YAML_PATH}`);
