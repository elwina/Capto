import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Unit tests run in Node against pure logic (hotkey parsing, formatters, etc.).
// Component/DOM tests can be added later by switching a file to environment
// "jsdom" (or happy-dom) and adding the matching dependency.
export default defineConfig({
  plugins: [react()],
  test: {
    include: ["src/**/*.test.{ts,tsx}"],
    environment: "node",
    // Expose afterEach globally so @testing-library/react auto-cleanup runs
    // between component tests (prevents DOM accumulation across cases).
    globals: true,
    coverage: {
      provider: "v8",
      // Coverage gate covers the modules that have unit tests: the hotkey
      // logic/UI and the i18n bootstrap. Add new modules here (with tests)
      // to bring them under the gate; App.tsx and the other views gain
      // coverage as component tests are introduced.
      include: ["src/i18n/index.ts", "src/components/HotkeySettings.tsx"],
      reporter: ["text", "html"],
      reportsDirectory: "coverage",
      thresholds: {
        lines: 80,
        functions: 75,
        branches: 70,
        statements: 80,
      },
    },
  },
});
