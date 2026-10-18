$ErrorActionPreference = 'Stop'
$alzTaskRoot152 = Split-Path -Parent $PSScriptRoot
$alzManifest152 = Join-Path $alzTaskRoot152 'tools/alz-engine/Cargo.toml'
& cargo build --release --locked --manifest-path $alzManifest152
if ($LASTEXITCODE -ne 0) { throw 'ALZ helper build failed' }
$alzResources152 = Join-Path $alzTaskRoot152 'src-tauri/resources/alz-engine'
New-Item -ItemType Directory -Force $alzResources152 | Out-Null
Copy-Item -LiteralPath (Join-Path $alzTaskRoot152 'tools/alz-engine/target/release/long-alz.exe') -Destination $alzResources152
Push-Location $alzTaskRoot152
try { & node scripts/lock-alz-engine.mjs; if ($LASTEXITCODE -ne 0) { throw 'ALZ resource lock failed' } }
finally { Pop-Location }
