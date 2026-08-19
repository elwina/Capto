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
  },
});
