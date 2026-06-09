import { defineConfig } from "rolldown";

const production = process.env.NODE_ENV === "production";

export default defineConfig({
  input: "client/src/extension.ts",
  platform: "node",
  external: ["vscode"],
  output: {
    file: "dist/extension.js",
    format: "cjs",
    sourcemap: !production,
    minify: production,
  },
});
