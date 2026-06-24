import js from "@eslint/js";
import ts from "@typescript-eslint/eslint-plugin";
import tsParser from "@typescript-eslint/parser";
import svelte from "eslint-plugin-svelte";

export default [
  js.configs.recommended,
  {
    files: ["**/*.ts"],
    languageOptions: { parser: tsParser },
    plugins: { "@typescript-eslint": ts },
    rules: {
      ...ts.configs.recommended.rules,
    },
  },
  ...svelte.configs["flat/recommended"],
];