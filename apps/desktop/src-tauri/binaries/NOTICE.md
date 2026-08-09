# Third-party notices (bundled FFmpeg)

Capto (MIT) may ship a bundled `ffmpeg.exe` sidecar for encoding.

That binary is a separate program. Its copyright and license terms are those of
FFmpeg and of any libraries it was linked with (often GPL when including
libx264, plus other components depending on the build).

When you redistribute Capto with a bundled FFmpeg:

1. Keep this NOTICE (or an updated copy) next to the app or in the installer.
2. Include the license texts required by your FFmpeg build (typically under
   FFmpeg's `COPYING.*` / component licenses).
3. Do not claim Capto's MIT license covers the FFmpeg binary.

Developer copies created by `scripts/copy-ffmpeg.ps1` inherit whatever license
applies to the FFmpeg build on that machine. Replace this file with the
upstream notices for the exact build you ship.
