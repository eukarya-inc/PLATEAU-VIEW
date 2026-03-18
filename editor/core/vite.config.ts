/// <reference types="vite/client" />
/// <reference types="vitest" />

import { resolve } from "path";

import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import dts from "vite-plugin-dts";
import svgr from "vite-plugin-svgr";
import { configDefaults } from "vitest/config";
// https://vitejs.dev/config/
export default defineConfig(() => ({
  plugins: [
    svgr(),
    react(),
    dts({ rollupTypes: true }),
  ],
  build: {
    lib: {
      entry: resolve(__dirname, "src/index.ts"),
      name: "@reearth/core",
    },
    rollupOptions: {
      external: ["react", "react-dom", "cesium"],
      output: {
        globals: {
          react: "React",
          "react-dom": "ReactDOM",
          cesium: "Cesium",
        },
      },
    },
  },
  resolve: {
    alias: [{ find: "csv-parse", replacement: "csv-parse/browser/esm" }],
  },
  test: {
    environment: "jsdom",
    setupFiles: ["src/test/setup.ts"],
    exclude: [...configDefaults.exclude, "e2e/*"],
    coverage: {
      all: true,
      include: ["src/**/*.ts", "src/**/*.tsx"],
      exclude: ["src/**/*.d.ts", "src/**/*.cy.tsx", "src/**/*.stories.tsx", "src/test/**/*"],
      reporter: ["text", "json", "lcov"],
    },
    alias: [{ find: "csv-parse", replacement: "csv-parse" }],
  },
}));
