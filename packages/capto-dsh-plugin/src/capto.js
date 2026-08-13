// @ts-check
/**
 * Capto CLI runner used by the dsh plugin.
 *
 * Spawns the `capto` binary, parses its JSON envelope, and normalizes every
 * failure into a {@link CaptoError} carrying the process exit code and the
 * envelope's error code — the model branches on those. Infrastructure
 * failures (spawn, timeout, cancellation) also throw with a stable message.
 *
 * Contract reference: docs/CLI.md in the Capto repo.
 */
import { execFile } from 'node:child_process';

export class CaptoError extends Error {
  /**
   * @param {string} message
   * @param {{ exitCode?: number, code?: string }} [meta]
   */
  constructor(message, { exitCode, code } = {}) {
    super(message);
    this.name = 'CaptoError';
    this.exitCode = exitCode;
    this.code = code;
  }
}

/** @returns {Error} the cooperative cancellation error the tool registry expects. */
export function abortError() {
  const error = new Error('tool call aborted');
  error.name = 'AbortError';
  return error;
}

const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/**
 * Parse the CLI's JSON envelope; throws CaptoError on a non-ok envelope or
 * malformed stdout.
 * @param {string} stdout
 * @param {number} exitCode
 */
function parseEnvelope(stdout, exitCode) {
  let envelope;
  try {
    envelope = JSON.parse(stdout);
  } catch {
    throw new CaptoError(`capto produced non-JSON stdout (exit ${exitCode})`);
  }
  if (envelope && envelope.ok === true) return { exitCode, data: envelope.data };
  const code = envelope?.error?.code ?? 'unknown';
  const message = envelope?.error?.message ?? 'unknown capto error';
  throw new CaptoError(`capto exited ${exitCode} (${code}): ${message}`, { exitCode, code });
}

/**
 * One spawn; resolves with `{ exitCode, data }` or rejects with
 * CaptoError / AbortError.
 * @param {{ command: string[], timeoutMs: number }} config
 * @param {string[]} args
 * @param {AbortSignal | undefined} signal
 */
function spawnOnce(config, args, signal) {
  const { command, timeoutMs } = config;
  return new Promise((resolve, reject) => {
    if (signal?.aborted) return reject(abortError());
    execFile(
      command[0],
      [...command.slice(1), ...args],
      {
        encoding: 'utf8',
        timeout: timeoutMs,
        maxBuffer: 16 * 1024 * 1024,
        windowsHide: true,
        signal,
      },
      (error, stdout) => {
        if (error) {
          if (error.name === 'AbortError' || signal?.aborted) return reject(abortError());
          if (error.killed && error.signal === 'SIGTERM') {
            return reject(new CaptoError(`capto timed out after ${timeoutMs}ms`, { code: 'timeout' }));
          }
          // A nonzero process exit still carries the JSON envelope on stdout.
          if (typeof error.code === 'number') {
            try {
              return reject(parseEnvelope(stdout, error.code));
            } catch (e) {
              return reject(e);
            }
          }
          return reject(new CaptoError(`failed to run capto (${error.message})`));
        }
        try {
          return resolve(parseEnvelope(stdout, 0));
        } catch (e) {
          return reject(e);
        }
      },
    );
  });
}

/**
 * Try once; a desktopUnavailable exit (2) comes back as `null` so
 * runCapto can recover from it.
 */
async function attempt(config, args, signal) {
  try {
    return await spawnOnce(config, args, signal);
  } catch (error) {
    if (error instanceof CaptoError && error.exitCode === 2) return null;
    throw error;
  }
}

/**
 * Run `capto <args>` and return `{ exitCode, data }`.
 *
 * With `config.autoOpen`, a desktopUnavailable exit triggers one `capto open`,
 * a ~3s wait, then a single retry of the original call. Otherwise the failure
 * is thrown with guidance the model can act on (run capto_open).
 *
 * @param {{ command: string[], timeoutMs: number, autoOpen: boolean }} config
 * @param {string[]} args CLI arguments AFTER the subcommand-free prefix in
 *   `config.command` (e.g. `['status']` or `['record','start', ...]`).
 * @param {{ signal?: AbortSignal }} [options]
 * @returns {Promise<{ exitCode: number, data: unknown }>}
 */
export async function runCapto(config, args, { signal } = {}) {
  let result = await attempt(config, args, signal);
  if (result !== null) return result;
  if (!config.autoOpen || args[0] === 'open') {
    throw new CaptoError(
      'capto exited 2 (desktopUnavailable) — run capto_open (or ask the user to open Capto), wait a few seconds, then retry',
      { exitCode: 2, code: 'desktopUnavailable' },
    );
  }
  await spawnOnce(config, ['open'], signal);
  await delay(3000);
  result = await attempt(config, args, signal);
  if (result === null) {
    throw new CaptoError(
      'capto exited 2 (desktopUnavailable) even after capto_open — ask the user to open Capto from the Start menu, then retry',
      { exitCode: 2, code: 'desktopUnavailable' },
    );
  }
  return result;
}
