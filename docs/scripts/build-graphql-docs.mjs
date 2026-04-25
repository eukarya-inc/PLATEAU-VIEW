#!/usr/bin/env node
// PLATEAU GraphQL API のスキーマを introspection で取得し、
// Starlight 用の Markdown リファレンスを生成する。
//
// 出力:
//   src/graphql/schema.graphql                          ← SDL
//   src/content/docs/api/graphql/schema.md              ← 自動生成 MD

import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  buildClientSchema,
  buildSchema,
  getIntrospectionQuery,
  printSchema,
} from "graphql";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");
const ENDPOINT = "https://api.plateauview.mlit.go.jp/datacatalog/graphql";
const SDL_PATH = resolve(ROOT, "src/graphql/schema.graphql");
const MD_PATH = resolve(ROOT, "src/content/docs/api/graphql/schema.md");
const LOCAL_SDL_PATH = resolve(
  ROOT,
  "..",
  "server",
  "datacatalog/plateauapi/schema.graphql",
);

function loadLocalSchema() {
  console.log(`[gql] Reading local SDL from ${LOCAL_SDL_PATH}`);
  const sdl = readFileSync(LOCAL_SDL_PATH, "utf8");
  // Re-print through buildSchema to canonicalize formatting (matches the
  // shape we get from introspection).
  return printSchema(buildSchema(sdl));
}

async function fetchSchema() {
  console.log(`[gql] Fetching introspection from ${ENDPOINT}`);
  const res = await fetch(ENDPOINT, {
    method: "POST",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: JSON.stringify({ query: getIntrospectionQuery() }),
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  const json = await res.json();
  if (json.errors) throw new Error(JSON.stringify(json.errors));
  const schema = buildClientSchema(json.data);
  return printSchema(schema);
}

function saveSdl(sdl) {
  mkdirSync(dirname(SDL_PATH), { recursive: true });
  writeFileSync(SDL_PATH, sdl, "utf8");
  console.log(`[gql] Saved SDL: ${SDL_PATH} (${sdl.length} chars)`);
}

function generateMarkdown() {
  mkdirSync(dirname(MD_PATH), { recursive: true });
  // graphql-markdown は標準出力に1つの長い MD を吐く
  const generated = execFileSync(
    "npx",
    [
      "--yes",
      "graphql-markdown",
      "--no-title",
      "--no-toc",
      "--heading-level",
      "2",
      SDL_PATH,
    ],
    { encoding: "utf8", cwd: ROOT, stdio: ["ignore", "pipe", "inherit"] },
  );

  const frontmatter = `---
title: GraphQL スキーマリファレンス
description: PLATEAU データカタログ API の GraphQL スキーマ全型定義
tableOfContents:
  maxHeadingLevel: 4
---

:::note[自動生成]
このページは PLATEAU データカタログ GraphQL API の introspection から自動生成されています。実装の真実の源は [\`/datacatalog/graphql\` エンドポイント](https://api.plateauview.mlit.go.jp/datacatalog/graphql) です。対話的にクエリを試したい場合は [プレイグラウンド](/api/graphql/playground/) を使ってください。
:::

`;
  // .md として書き出し（MDX を使わないので {z}/{x}/{y} のような波括弧も安全）
  writeFileSync(MD_PATH, frontmatter + generated, "utf8");
  console.log(`[gql] Wrote: ${MD_PATH} (${generated.length} chars)`);
}

const cmd = process.argv[2] || "all";
if (cmd === "fetch" || cmd === "all") {
  const sdl = await fetchSchema();
  saveSdl(sdl);
  // sanity check: ensure SDL parses
  buildSchema(sdl);
}
if (cmd === "local") {
  // 本番反映を待たずにローカル schema.graphql から SDL+MD を生成する。
  saveSdl(loadLocalSchema());
  generateMarkdown();
}
if (cmd === "gen" || cmd === "all") {
  // SDL must exist by now
  readFileSync(SDL_PATH, "utf8");
  generateMarkdown();
}
console.log("[gql] Done.");
