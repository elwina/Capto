// @ts-check
/**
 * Tool definitions for the capto dsh plugin.
 *
 * Every tool is a thin, typed wrapper over the `capto` CLI JSON contract
 * (docs/CLI.md): parameters map onto CLI flags, the envelope's `data` passes
 * through as the canonical tool output, and failures surface as Error
 * messages carrying the exit code + error code so the model can branch
 * (exit 2 → capto_open).
 */
import { runCapto } from './capto.js';

/** One shared output declaration: evolution-proof (Capto may add fields). */
const OUTPUT = {
  schema: { type: 'json' },
  render: (_args, value) => [{ type: 'text', text: JSON.stringify(value, null, 2) }],
};

const str = (description) => ({ type: 'string', description });
const integer = (description) => ({ type: 'integer', description });
const boolean = (description) => ({ type: 'boolean', description });

const SOURCE = {
  type: 'string',
  description: "Capture source: 'display' (default), 'window', or 'region'.",
  enum: ['display', 'window', 'region'],
};

const SOURCE_PARAMS = {
  source: SOURCE,
  display: integer('Display id (from capto_list displays).'),
  window: integer('Window id/hwnd (from capto_list windows).'),
  x: integer('Region left (with y/width/height).'),
  y: integer('Region top (with x/width/height).'),
  width: integer('Region width (with x/y/height).'),
  height: integer('Region height (with x/y/width).'),
};

/** Map source params onto `--flag value` CLI args. */
function sourceArgs(args) {
  const out = ['--source', args.source ?? 'display'];
  for (const key of ['display', 'window', 'x', 'y', 'width', 'height']) {
    if (args[key] !== undefined) out.push(`--${key}`, String(args[key]));
  }
  return out;
}

/** Execute helper: normalize missing args, run, return the envelope `data`. */
function exc(config, argsFn) {
  return async (args, exec) => {
    const a = args ?? {};
    return (await runCapto(config, argsFn(a), { signal: exec.signal })).data;
  };
}

const readOnly = () => true;

/**
 * Build the `capto_*` tool definitions for one configuration.
 * @param {{ command: string[], timeoutMs: number, autoOpen: boolean }} config
 * @returns {import('@deepseek-ai/dsh-tools').ToolDefinition[]} plain tool definitions
 */
export function defineCaptoTools(config) {
  return [
    {
      name: 'capto_status',
      description:
        'Capto recording-session status: `{ state, elapsedMs, outputPath, lastError, encoder, hideApp }` with state one of `idle | starting | recording | paused | stopping`. Check this before capto_record_start — do not start twice.',
      parameters: {},
      output: OUTPUT,
      isConcurrencySafe: readOnly,
      execute: exc(config, () => ['status']),
    },
    {
      name: 'capto_doctor',
      description:
        'Capto environment readiness: `{ os, captureBackend, ffmpegPath, ffmpegOk, controlPlane, pid, port, preferredEncoder }`. `ffmpegOk` must be true before recording; if not, tell the user Capto needs its bundled FFmpeg (reinstall).',
      parameters: {},
      output: OUTPUT,
      isConcurrencySafe: readOnly,
      execute: exc(config, () => ['doctor']),
    },
    {
      name: 'capto_open',
      description:
        'Open the Capto desktop window (does not wait for the control plane). Use when another capto_* call errors with exit 2 / `desktopUnavailable`: run this, wait ~3–5 s, then retry. If it still fails, ask the user to open Capto from the Start menu.',
      parameters: {},
      output: OUTPUT,
      execute: exc(config, () => ['open']),
    },
    {
      name: 'capto_list',
      description:
        'Enumerate Capto capture sources. `what` is one of `displays`, `windows`, `audio`, `encoders`. Returns an array of source descriptors; feed ids into capto_shot / capto_record_start.',
      parameters: {
        what: {
          type: 'string',
          required: true,
          enum: ['displays', 'windows', 'audio', 'encoders'],
          description: 'What to enumerate.',
        },
      },
      output: OUTPUT,
      isConcurrencySafe: readOnly,
      execute: exc(config, (a) => ['list', a.what]),
    },
    {
      name: 'capto_shot',
      description:
        'Take a screenshot through the Capto desktop; returns `{ path }` — an absolute PNG path you can read or hand to the user.',
      parameters: SOURCE_PARAMS,
      output: OUTPUT,
      isConcurrencySafe: readOnly,
      execute: exc(config, (a) => ['shot', ...sourceArgs(a)]),
    },
    {
      name: 'capto_record_start',
      description:
        'Start a screen recording in the single Capto desktop session; returns a session snapshot `{ state, outputPath, ... }`. `format` is `mp4` (default), `gif`, or `audio`. Check capto_status first — do not start while state is recording/paused. Always end with capto_record_stop.',
      parameters: {
        ...SOURCE_PARAMS,
        format: {
          type: 'string',
          enum: ['mp4', 'gif', 'audio'],
          description: 'Output container: mp4 (default), gif, or audio-only.',
        },
        fps: integer('Target frames per second (e.g. 30 or 60).'),
        quality: integer('Quality 0–100 when the encoder supports it.'),
        encoder: str(
          'Encoder name: h264_nvenc / h264_qsv / h264_amf / libx264 / hevc_nvenc / hevc_qsv / hevc_amf / libx265 / gif. Omit for the preferred encoder.',
        ),
        mic: str('Microphone device id (from capto_list audio).'),
        loopback: str('Loopback (system audio) device id (from capto_list audio).'),
        noCursor: boolean('Hide the cursor in the recording.'),
      },
      output: OUTPUT,
      execute: exc(config, (a) => {
        const out = ['record', 'start', ...sourceArgs(a)];
        if (a.format !== undefined) out.push('--format', a.format);
        if (a.fps !== undefined) out.push('--fps', String(a.fps));
        if (a.quality !== undefined) out.push('--quality', String(a.quality));
        if (a.encoder !== undefined) out.push('--encoder', a.encoder);
        if (a.mic !== undefined) out.push('--mic', a.mic);
        if (a.loopback !== undefined) out.push('--loopback', a.loopback);
        if (a.noCursor === true) out.push('--no-cursor');
        return out;
      }),
    },
    {
      name: 'capto_record_stop',
      description:
        'Stop the current recording; returns the final session snapshot with `outputPath` (the produced file).',
      parameters: {},
      output: OUTPUT,
      execute: exc(config, () => ['record', 'stop']),
    },
    {
      name: 'capto_record_pause',
      description: 'Pause the current recording; returns a session snapshot.',
      parameters: {},
      output: OUTPUT,
      execute: exc(config, () => ['record', 'pause']),
    },
    {
      name: 'capto_record_resume',
      description: 'Resume a paused recording; returns a session snapshot.',
      parameters: {},
      output: OUTPUT,
      execute: exc(config, () => ['record', 'resume']),
    },
    {
      name: 'capto_config_get',
      description:
        'Read Capto settings: omit `key` for the full settings object, or pass a camelCase key (e.g. `fps`, `outputDir`, `includeCursor`, `micDevice`, `overlays`) for one value.',
      parameters: {
        key: str('Optional camelCase settings key to read.'),
      },
      output: OUTPUT,
      isConcurrencySafe: readOnly,
      execute: exc(config, (a) => ['config', 'get', ...(a.key !== undefined ? [a.key] : [])]),
    },
    {
      name: 'capto_config_set',
      description:
        'Patch Capto settings and return the updated settings object. Keys are camelCase. Provide `json` (an object string, e.g. `{"fps":60,"includeCursor":true}`) and/or `pairs` (e.g. `["fps=60","micDevice=..."]`); at least one is required.',
      parameters: {
        json: str('JSON patch object as a string, e.g. {"fps":60}'),
        pairs: {
          type: 'array',
          items: { type: 'string' },
          description: 'camelCase key=value pairs, e.g. ["fps=60","includeCursor=true"]',
        },
      },
      output: OUTPUT,
      execute: exc(config, (a) => {
        if (a.json === undefined && (!a.pairs || a.pairs.length === 0)) {
          throw new Error('capto_config_set: provide `json` or at least one `pairs` entry');
        }
        const out = ['config', 'set'];
        if (a.json !== undefined) out.push('--json', a.json);
        if (a.pairs) out.push(...a.pairs);
        return out;
      }),
    },
    {
      name: 'capto_config_path',
      description: 'Absolute path of the Capto settings.json file.',
      parameters: {},
      output: OUTPUT,
      isConcurrencySafe: readOnly,
      execute: exc(config, () => ['config', 'path']),
    },
    {
      name: 'capto_outputs_recent',
      description:
        'Recent Capto output files: `{ outputDir, items: [{ path, name, bytes, modifiedMs }] }`. Use to find a finished recording or screenshot.',
      parameters: {
        limit: integer('Max entries (default 20).'),
      },
      output: OUTPUT,
      isConcurrencySafe: readOnly,
      execute: exc(config, (a) => ['outputs', 'recent', '--limit', String(a.limit ?? 20)]),
    },
    {
      name: 'capto_outputs_open',
      description:
        'Open a Capto output file or folder in the OS (Explorer). Pass `path` for a specific file, `last: true` for the newest output, and/or `folder: true` to open the output directory.',
      parameters: {
        path: str('Absolute path of the file to open.'),
        last: boolean('Open the most recent output file.'),
        folder: boolean('Open the output folder instead of a file.'),
      },
      output: OUTPUT,
      execute: exc(config, (a) => {
        const out = ['outputs', 'open'];
        if (a.path !== undefined) out.push(a.path);
        if (a.last === true) out.push('--last');
        if (a.folder === true) out.push('--folder');
        return out;
      }),
    },
  ];
}
