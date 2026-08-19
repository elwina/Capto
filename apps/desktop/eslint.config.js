// Flat-config ESLint for the Capto desktop frontend (React + TypeScript + Vite).
// Enforced in CI via `npm run lint` (see ci.yml -> frontend job).
import js from "@eslint/js";
import globals from "globals";
import reactHooks from "eslint-plugin-react-hooks";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["dist", "node_modules", "coverage"] },

  // TypeScript + core recommended rules for all app sources.
  {
    extends: [js.configs.recommended, ...tseslint.configs.recommended],
    files: ["src/**/*.{ts,tsx}"],
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: "module",
      globals: globals.browser,
    },
    plugins: {
      "react-hooks": reactHooks,
    },
    rules: {
      // Enforce React hook rules + exhaustive-deps (real bugs, not style).
      // eslint-plugin-react-hooks v5's stable `recommended-latest` (rules-of-hooks
      // + exhaustive-deps). We intentionally stay on the mainstream v5 preset
      // rather than v7, whose newer experimental rules (set-state-in-effect,
      // refs) would force risky refactors of the recording/webcam hooks.
      ...reactHooks.configs["recommended-latest"].rules,
      // `unknown` forces callers to narrow; overlay settings get explicit types
      // in src/overlays.ts so unchecked `any` is a lint error, not a choice.
      "@typescript-eslint/no-explicit-any": "error",
      // Underscore-prefixed params/vars are the idiomatic "intentionally
      // unused" marker (e.g. useWebcamPreview keeps its (enabled, deviceId)
      // signature for API stability without using them yet).
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_", caughtErrorsIgnorePattern: "^_" },
      ],
    },
  },

  // Vite/Vitest build configs run under Node, not the browser.
  {
    files: ["vite.config.ts", "vite.config.*.ts", "vitest.config.ts"],
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: "module",
      globals: globals.node,
    },
  },
);
