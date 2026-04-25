import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import starlightOpenAPI, { openAPISidebarGroups } from "starlight-openapi";
import starlightLlmsTxt from "starlight-llms-txt";
import { GITHUB_URL } from "./src/site.js";

export default defineConfig({
  site: "https://docs.plateauview.mlit.go.jp",
  integrations: [
    starlight({
      title: "PLATEAU 配信サービス",
      customCss: ["./src/styles/custom.css"],
      components: {
        Header: "./src/components/Header.astro",
        Hero: "./src/components/Hero.astro",
      },
      description:
        "PLATEAU 3D都市モデル配信サービスの利用ガイド・APIリファレンス",
      defaultLocale: "root",
      locales: {
        root: { label: "日本語", lang: "ja" },
      },
      logo: {
        src: "./src/assets/logo.svg",
        replacesTitle: false,
      },
      favicon: "/favicon.svg",
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: GITHUB_URL,
        },
      ],
      plugins: [
        starlightLlmsTxt({
          projectName: "PLATEAU 配信サービス",
          description:
            "PLATEAU 3D都市モデルの配信API・データセット利用ガイド",
          exclude: ["playground/**"],
        }),
        starlightOpenAPI([
          {
            base: "api/rest",
            label: "REST API",
            schema: "./src/openapi/plateau-api.json",
          },
        ]),
      ],
      sidebar: [
        {
          label: "はじめに",
          items: [
            { label: "概要", slug: "intro" },
            { label: "クイックスタート", slug: "quickstart" },
          ],
        },
        {
          label: "データセット",
          items: [
            { label: "データセット一覧", slug: "datasets/explorer" },
            { label: "PLATEAU-CityGML", slug: "datasets/citygml" },
            { label: "PLATEAU-3DTiles / MVT", slug: "datasets/3d-tiles" },
            { label: "PLATEAU-Terrain", slug: "datasets/terrain" },
            { label: "PLATEAU-Ortho", slug: "datasets/ortho" },
          ],
        },
        {
          label: "API リファレンス",
          items: [
            { label: "概要", slug: "api" },
            ...openAPISidebarGroups,
            {
              label: "GraphQL API",
              items: [
                { label: "概要", slug: "api/graphql" },
                { label: "スキーマリファレンス", slug: "api/graphql/schema" },
                { label: "プレイグラウンド", slug: "api/graphql/playground" },
              ],
            },
          ],
        },
        {
          label: "MCP Server",
          items: [{ label: "PLATEAU MCP Server", slug: "mcp/overview" }],
        },
        {
          label: "仕様",
          items: [
            { label: "可視化用データ変換仕様", slug: "spec/visualization" },
          ],
        },
      ],
    }),
  ],
});
