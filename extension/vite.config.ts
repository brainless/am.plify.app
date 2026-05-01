import { defineConfig } from "vite";
import solid from "vite-plugin-solid";
import { resolve } from "path";
import { copyFileSync } from "fs";

export default defineConfig({
  plugins: [
    solid(),
    {
      name: "copy-manifest",
      closeBundle() {
        copyFileSync(
          resolve(__dirname, "manifest.json"),
          resolve(__dirname, "dist/manifest.json")
        );
      },
    },
  ],
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "chrome100",
    rollupOptions: {
      input: {
        background: resolve(__dirname, "src/background.ts"),
        popup: resolve(__dirname, "popup.html"),
        "content-script": resolve(__dirname, "src/content-script.ts"),
      },
      output: {
        entryFileNames: "[name].js",
        chunkFileNames: "chunks/[name]-[hash].js",
      },
    },
  },
});
