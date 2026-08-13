// Fake `capto` CLI for offline smoke tests. Mirrors the JSON envelope
// contract from docs/CLI.md: `{ ok, data }` / `{ ok:false, error:{code,message} }`
// on stdout, exit codes 0 / 2.
//
// Modes (environment, read at spawn time):
//   FAKE_CAPTO_BOOM=1        → every call fails with exit 2 desktopUnavailable
//   FAKE_CAPTO_SLEEP_MS=<ms> → sleep before answering (timeout tests)
//   FAKE_CAPTO_MARKER=<path> → `status` fails exit 2 while the marker exists;
//                              `open` removes the marker (autoOpen tests)
//   FAKE_CAPTO_ECHO=1        → echo back argv as data (arg-mapping tests)
import { existsSync, rmSync } from 'node:fs';

const args = process.argv.slice(2);

const sleepMs = Number(process.env.FAKE_CAPTO_SLEEP_MS ?? 0);
if (sleepMs > 0) await new Promise((resolve) => setTimeout(resolve, sleepMs));

const fail = (code, message) => {
  process.stdout.write(JSON.stringify({ ok: false, error: { code, message } }, null, 2));
  process.exit(code === 'desktopUnavailable' ? 2 : 1);
};

if (process.env.FAKE_CAPTO_BOOM === '1') {
  fail('desktopUnavailable', 'no fake desktop');
}

const marker = process.env.FAKE_CAPTO_MARKER;
if (marker && existsSync(marker)) {
  if (args[0] === 'open') {
    rmSync(marker);
    process.stdout.write(
      JSON.stringify({ ok: true, data: { path: 'C:\\fake\\capto-app.exe', hint: 'wait a bit' } }, null, 2),
    );
    process.exit(0);
  }
  fail('desktopUnavailable', `fake desktop down (marker ${marker})`);
}

if (process.env.FAKE_CAPTO_ECHO === '1') {
  process.stdout.write(JSON.stringify({ ok: true, data: { args } }, null, 2));
  process.exit(0);
}

switch (args[0]) {
  case 'open':
    process.stdout.write(
      JSON.stringify({ ok: true, data: { path: 'C:\\fake\\capto-app.exe', hint: 'wait a bit' } }, null, 2),
    );
    break;
  case 'status':
    process.stdout.write(
      JSON.stringify(
        {
          ok: true,
          data: { state: 'idle', elapsedMs: 0, outputPath: null, lastError: null, encoder: null, hideApp: false },
        },
        null,
        2,
      ),
    );
    break;
  case 'doctor':
    process.stdout.write(
      JSON.stringify(
        {
          ok: true,
          data: {
            os: 'windows',
            captureBackend: 'wgc',
            ffmpegPath: 'C:\\fake\\ffmpeg.exe',
            ffmpegOk: true,
            controlPlane: true,
            pid: 1234,
            port: 7343,
            preferredEncoder: 'h264_nvenc',
          },
        },
        null,
        2,
      ),
    );
    break;
  case 'record':
    if (args[1] === 'start') {
      process.stdout.write(
        JSON.stringify(
          {
            ok: true,
            data: {
              state: 'recording',
              elapsedMs: 120,
              outputPath: 'C:\\fake\\capto-2026-08-10.mp4',
              lastError: null,
              encoder: 'libx264',
              hideApp: true,
            },
          },
          null,
          2,
        ),
      );
    } else {
      process.stdout.write(
        JSON.stringify(
          {
            ok: true,
            data: {
              state: 'idle',
              elapsedMs: 5000,
              outputPath: 'C:\\fake\\capto-2026-08-10.mp4',
              lastError: null,
              encoder: 'libx264',
              hideApp: false,
            },
          },
          null,
          2,
        ),
      );
    }
    break;
  default:
    // echo back argv so tests can assert arg mapping
    process.stdout.write(JSON.stringify({ ok: true, data: { args } }, null, 2));
    break;
}
process.exit(0);
