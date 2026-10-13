# PentaKuru + Sally

This combined application includes Sally by cpbrr:
https://github.com/cpbrr/Sally

Vendored revision: 93aaec6361c127847e3a145881190c56ef4a5b5f (version 1.5.0).
The upstream LICENSE contains GPL-3.0. It is retained at vendor/Sally/LICENSE;
the combined application is distributed under GPL-3.0-only, in LICENSE.
The original PentaKuru MIT notice is retained in LICENSE-PENTEYE-MIT.
The vendored Cargo manifest's license field was corrected to match LICENSE.

Local modifications: reusable Tauri builder, embedded Windows runtime,
PentaKuru window configuration, and a frontend event to collect a stopped
meeting when the parent disables translation. The React UI and all upstream
audio, Gemini, storage, review, export and cleanup commands are included.
Audio fixes: microphone-off video capture by default, atomic mute at Start,
release/reopen the microphone on mute/unmute, skip Windows mic warm-up,
clear buffered mic audio on mute, and acknowledge stream startup errors.
Feedback fixes: self-excluding Windows process loopback, process-tree checks,
and a readout guard requiring isolated capture and a muted microphone.

Noto Sans licensing is in assets/fonts/OFL.txt. Other dependencies retain
their respective licenses. Corresponding combined source is in the
source ZIP supplied beside the Windows executable ZIP.

Optional integrations added in October 2026:
- LibreOffice is controlled through its official UNO API. Its runtime is reused
  from an existing installation or downloaded on demand from The Document Foundation.
  https://api.libreoffice.org/
- DeepTutor by HKUDS (Apache-2.0) is installed on demand from PyPI, pinned to 1.6.14.
  Its official runtime and frontend are not bundled into this executable.
  https://github.com/HKUDS/DeepTutor/tree/v1.6.14
- Penta Music and Penta Clip load the user's supplied services in isolated WebView2
  windows. The Music adaptation script is local; upstream site assets are loaded
  from their respective origins and are not copied into this source archive.
  https://music.pentasys.id.vn/ and https://clip.pentasys.id.vn/

Optional DeepTutor prerequisites are downloaded on demand: Python 3.13.12 (Python Software Foundation license, official NuGet package) and Node.js 24.21.0 LTS (MIT and bundled third-party notices). Their license files remain in the downloaded runtime distribution. They are not embedded in the PentaKuru executable.
