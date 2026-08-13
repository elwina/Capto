// @ts-check
/**
 * capto-dsh-plugin — DeepSeek Harness (dsh) tool plugin.
 *
 * Registers first-class `capto_*` tools that drive the local-only Capto
 * screen recorder through its `capto` CLI control plane (JSON over
 * localhost). Every tool is a typed wrapper around the CLI contract in
 * docs/CLI.md; failures surface as Error messages carrying the CLI exit
 * code and error code so the model can branch (exit 2 → capto_open).
 *
 * The package root exports exactly the Cordis plugin contract:
 * `{ name, inject, Config, apply }`.
 */
import z from '@deepseek-ai/schemastery';
import { defineTool } from '@deepseek-ai/dsh-tools';
import { defineCaptoTools } from './tools.js';

export const name = 'capto';

export const inject = ['tools', 'systemPrompt'];

/** Plugin config: how to invoke the capto CLI and how to behave on failure. */
export const Config = z.object({
  /** argv prefix of the CLI, e.g. ['capto'] (on PATH) or ['D:\\...\\capto.exe']. */
  command: z.array(z.string()).min(1).default(['capto']),
  /** Per-call timeout in ms (kills the CLI child). Must exceed the CLI's
   * 45s auto-launch wait (crates/capto-cli client.rs wait_for_ready), so a
   * cold desktop start can finish before the tool gives up. */
  timeoutMs: z.number().min(1).default(120000),
  /** Always pass --no-launch (never auto-start the Capto desktop). */
  noLaunch: z.boolean().default(false),
  /** On exit 2 (desktopUnavailable): run `capto open`, wait ~3s, retry once. */
  autoOpen: z.boolean().default(false),
});

const PROMPT = [
  'Drive the local-only Capto screen recorder through the `capto_*` tools. They talk to the single Capto desktop process on this machine over localhost; never spawn FFmpeg or shell out to the `capto` binary for capture work.',
  '- Before `capto_record_start`, check `capto_status`; do not start twice while state is `recording`/`paused`.',
  '- A call failing with `desktopUnavailable` (exit 2): run `capto_open`, wait ~3–5 s, then retry; if it still fails, ask the user to open Capto from the Start menu.',
  '- End recordings with `capto_record_stop`; find the file with `capto_outputs_recent`.',
  '- Nonzero exits are reported in the error message as `(exit <code>: <errorCode>)`.',
].join('\n');

/**
 * @param {import('@deepseek-ai/cordis').Context} ctx
 * @param {Partial<import('@deepseek-ai/schemastery').Infer<typeof Config>>} [config]
 */
export function apply(ctx, config = {}) {
  const cfg = {
    command: config.command ?? ['capto'],
    timeoutMs: config.timeoutMs ?? 120000,
    noLaunch: config.noLaunch ?? false,
    autoOpen: config.autoOpen ?? false,
  };
  // Global CLI flags precede the subcommand: `capto --no-launch <command>`.
  const runtime = cfg.noLaunch
    ? { ...cfg, command: [...cfg.command, '--no-launch'] }
    : cfg;

  ctx.systemPrompt.section({
    name: 'tool:capto',
    order: 110,
    text: PROMPT,
  });

  for (const tool of defineCaptoTools(runtime)) {
    ctx.tools.register(defineTool(tool));
  }
}
