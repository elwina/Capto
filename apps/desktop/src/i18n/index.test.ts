import { describe, expect, it } from "vitest";
import i18n, { SUPPORTED_LOCALES } from "./index";

describe("SUPPORTED_LOCALES", () => {
  it("exposes a non-empty list of UI locales", () => {
    expect(SUPPORTED_LOCALES.length).toBeGreaterThan(0);
  });

  it("uses unique BCP-47 ids", () => {
    const ids = SUPPORTED_LOCALES.map((l) => l.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("always includes English as the default locale", () => {
    expect(SUPPORTED_LOCALES.some((l) => l.id === "en")).toBe(true);
  });

  it("gives every locale a non-empty native label", () => {
    for (const locale of SUPPORTED_LOCALES) {
      expect(locale.nativeLabel.length).toBeGreaterThan(0);
    }
  });

  it("registers a translation bundle for every supported locale", () => {
    for (const locale of SUPPORTED_LOCALES) {
      expect(i18n.hasResourceBundle(locale.id, "translation")).toBe(true);
    }
  });
});
