import js from "@eslint/js";
import typescript from "@typescript-eslint/eslint-plugin";
import typescriptParser from "@typescript-eslint/parser";
import solid from "eslint-plugin-solid";

export default [
  js.configs.recommended,
  {
    files: ["src/**/*.{ts,tsx}"],
    languageOptions: {
      parser: typescriptParser,
    },
    plugins: {
      "@typescript-eslint": typescript,
      solid,
    },
    rules: {
      "@typescript-eslint/no-unused-vars": "error",
      "solid/reactivity": "warn",
    },
  },
];