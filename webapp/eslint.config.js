export default [
  {
    files: ["src/**/*.{ts,tsx}"],
    plugins: ["@typescript-eslint", "solid"],
    rules: {
      "@typescript-eslint/no-unused-vars": "error",
      "solid/reactivity": "warn"
    }
  }
];