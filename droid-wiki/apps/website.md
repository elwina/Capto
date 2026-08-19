# Website

Active contributors: elwina

Capto's product landing page is a single static HTML file, `website/index.html`, with no build step. The same folder is deployed to both GitHub Pages and Cloudflare Pages, so one page serves the full marketing surface.

## Purpose

The page tells a first-time visitor what Capto is (an ultra-light, purely local Windows screen recorder) and points them at downloads and sources. Content is bilingual (English and a simplified-Chinese default) with a small inline language switcher, the brand colors are intentionally Capto purple.

## How it works

The whole site lives in one file: CSS and the tiny inline JavaScript (a language toggle and an architecture-aware download link, `website/index.html`) are embedded in `index.html`. There is no bundler, no framework, and no generated `dist` folder. The page is served as-is by whatever host it lands on.

Content themes:

- **Product**, capture modes (display / window / region), MP4 with NVENC / QSV / AMF / libx264 fallback, GIF, overlays (click highlights, keystrokes, webcam PiP, cursor toggle).
- **Download links**, a `#download` section with x64 and ARM64 NSIS installer buttons, calling `github.com/elwina/Capto/releases/download/v1.0.0/Capto_1.0.0_(_arch_)-setup.exe`. A small `RELEASES` object in the inline script lets the page pick the ARM64 asset based on `userAgent`.
- **Agent integrations**, a feature block highlighting the JSON control plane, stable exit codes, and `capto-agent-skill`, so AI agents can run a `doctor → record → stop` loop.

## Deployment and integration

Two active hosts with the same `website/` content, detailed in [Deployment](../deployment.md) and `docs/CI.md`:

- **GitHub Pages** (`https://elwina.github.io/Capto/`), driven by `.github/workflows/pages.yml` on every push to `main` (or manually). It deploys one artifact: the landing page at the site root plus the workspace rustdoc under `/docs/`. `pages.yml` is the only workflow allowed to write the `github-pages` environment.
- **Cloudflare Pages** (`https://capto.elwina.work/` primary, `capto.pages.dev` fallback), driven by the Cloudflare dashboard Git integration, not a workflow, so no Cloudflare token secret is needed. The project's root directory is set to `website/` so the monorepo is not deployed as a whole.

Cloudflare Pages is the primary entry; GitHub Pages stays as a fallback if Cloudflare is unreachable.

## Key source files

| File | Purpose |
|------|---------|
| `website/index.html` | The entire landing page (structure, styles, inline JS) |
| `website/README.md` | How to open the page locally and where it deploys |
| `.github/workflows/pages.yml` | GitHub Pages deployment + rustdoc |
