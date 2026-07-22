# Windows 11 primary context menu

Windows 11 only places third-party `IExplorerCommand` actions in the primary
context menu when they are registered through an app identity. This project
keeps its NSIS installer and supplies that identity with a signed sparse MSIX.

Release builds read these secrets:

- `WINDOWS_CODE_SIGNING_PFX_BASE64`: base64-encoded production code-signing PFX
- `WINDOWS_CODE_SIGNING_PFX_PASSWORD`: PFX password
- `WINDOWS_CODE_SIGNING_PUBLISHER`: certificate subject, for example `CN=...`

The publisher must exactly match the signing certificate subject; the build
checks this before packaging. Local builds skip the identity package and retain
the classic Explorer menu fallback when these variables are absent. Release
builds set `REQUIRE_WINDOWS_CONTEXT_MENU_PACKAGE=true`, so a missing or invalid
certificate fails the release instead of silently shipping the fallback.

Two sparse package manifests expose separate top-level COM commands. Each
command uses a distinct package identity because Windows groups multiple verbs
from the same app into an app-attributed flyout:

- supported archives: **一键解压到同名文件夹**
- files and directories: **一键打包为 ZIP**

The full advanced command group remains available through the classic menu.
Each sparse MSIX contains the architecture-matched shell-extension DLL referenced
by `com:Class/@Path`; the main Tauri executable remains in the external NSIS
installation directory.

For a reversible local end-to-end test, first build the release application and
shell extension, then run `npm run test:context-menu-package` from an elevated
PowerShell window. Windows evaluates self-signed package trust from the local
machine's Trusted People store, so the script checks for administrator rights
before changing any state. The test creates a two-day development certificate,
temporarily trusts only that certificate, registers the sparse package,
activates both top-level COM classes, and removes the package, certificate, and
temporary files in `finally` cleanup.

For a manual Explorer UI check, the same script also supports
`-PauseForVisualTest` with ready/continue marker files under the system temp
directory. It keeps the development package installed only while the visual
check is in progress and then uses the same `finally` cleanup path.
