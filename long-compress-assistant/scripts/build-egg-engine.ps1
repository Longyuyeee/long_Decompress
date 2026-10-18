$ErrorActionPreference = 'Stop'
$eggTaskRoot153 = Split-Path -Parent $PSScriptRoot
& cargo build --release --locked --manifest-path (Join-Path $eggTaskRoot153 'tools/egg-engine/Cargo.toml')
if ($LASTEXITCODE -ne 0) { throw 'EGG helper build failed' }
$eggResources153 = Join-Path $eggTaskRoot153 'src-tauri/resources/egg-engine'
New-Item -ItemType Directory -Force $eggResources153 | Out-Null
Copy-Item -LiteralPath (Join-Path $eggTaskRoot153 'tools/egg-engine/target/release/long-egg.exe') -Destination $eggResources153
Push-Location $eggTaskRoot153
try { & node scripts/lock-egg-engine.mjs; if ($LASTEXITCODE -ne 0) { throw 'EGG resource lock failed' } }
finally { Pop-Location }
