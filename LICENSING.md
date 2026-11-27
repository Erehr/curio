# Curio licensing

Copyright (c) 2026 Erehr

Curio is free software: you may redistribute it and/or modify it under the terms of the GNU General Public License, version 3 only, as published by the Free Software Foundation. That covers the whole repository: the program, the API client in `src/core/api/client/`, the plugin templates and the code examples in the documentation.

Curio is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.

The GNU GPL version 3 is in [LICENSE](LICENSE); if it is missing, see https://www.gnu.org/licenses/gpl-3.0.html.

## Third-party material

Dependencies, vendored components and reference projects keep their own licenses and notices. Among those shipped with Curio:

- FFmpeg on Windows (`resources/ffmpeg/`, with its `LICENSE.txt`): FFmpeg (https://ffmpeg.org) in BtbN's GPL build with shared libraries (https://github.com/BtbN/FFmpeg-Builds), the newest release build when Curio was built, under the GPL version 3. It is a separate program Curio runs, and plugins may run it too.
- FFmpeg on macOS (`resources/ffmpeg/` in the app's `Contents/Resources`): FFmpeg (https://ffmpeg.org) in Martin Riedl's static GPL builds (https://ffmpeg.martin-riedl.de), the newest release build when Curio was built, under the GPL version 3. It is a separate program Curio runs, and plugins may run it too.
- The release carrying a Windows or macOS download also carries `Curio-<version>-ffmpeg-source.tar`: which FFmpeg builds it bundles, with their addresses and SHA-256, FFmpeg's corresponding source for each, and the list of libraries each build compiles in. `ffmpeg -version` in `resources/ffmpeg/` names the version a copy carries.
- Vue, OpenVue and Tabler Icons (MIT) in the window.

No trademark rights are granted. Copies received earlier under other terms keep the permissions given with them.
