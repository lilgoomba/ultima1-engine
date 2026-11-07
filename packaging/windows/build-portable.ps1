# Builds build\ultima1-engine-windows-portable-x86_64\ (and its .zip): the
# release program with SDL2 linked in, the enhanced assets, the licences and
# a portable.txt marker so config and saves stay beside the program.
# No game files: the player brings their own (see README.txt).
# Run on Windows with the MSVC Rust toolchain and cmake (the GitHub runner).
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..\..')
$name = 'ultima1-engine-windows-portable-x86_64'
$env:CMAKE_POLICY_VERSION_MINIMUM = '3.5'
cargo build --release --locked -p u1 --features u1/bundled,u1/windows-gui
if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }
$target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'target' }

$out = Join-Path 'build' $name
if (Test-Path $out) { Remove-Item -Recurse -Force $out }
New-Item -ItemType Directory -Force (Join-Path $out 'ULTIMA1'), (Join-Path $out 'assets') | Out-Null
Copy-Item (Join-Path $target 'release\u1.exe') (Join-Path $out 'ultima1-engine.exe')
Copy-Item -Recurse 'assets\enhanced' (Join-Path $out 'assets\enhanced')
Copy-Item 'packaging\windows\README.txt' $out
Copy-Item 'packaging\windows\portable.txt' $out
Copy-Item 'packaging\windows\ULTIMA1-README.txt' (Join-Path $out 'ULTIMA1\README.txt')
Copy-Item 'LICENSE' (Join-Path $out 'LICENSE.txt')
Copy-Item 'CREDITS' (Join-Path $out 'CREDITS.txt')
Copy-Item 'PLAYING.md' $out

# DLLs the program needs that are not part of Windows itself are shipped
# beside it (only the Visual C++ runtime, if it is linked dynamically).
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$vs = & $vswhere -latest -products * -property installationPath
$dumpbin = Get-ChildItem -Recurse "$vs\VC\Tools\MSVC" -Filter dumpbin.exe | Where-Object { $_.FullName -match 'Hostx64\\x64' } | Select-Object -First 1
$deps = & $dumpbin.FullName /nologo /dependents (Join-Path $out 'ultima1-engine.exe') | ForEach-Object { $_.Trim() } | Where-Object { $_ -match '\.dll$' }
Write-Host "imports: $($deps -join ', ')"
$system = '^(kernel32|user32|gdi32|winmm|imm32|ole32|oleaut32|version|setupapi|shell32|advapi32|ws2_32|bcrypt|bcryptprimitives|ntdll|userenv|cfgmgr32|hid|dinput8|dxgi|d3d9|d3d11|opengl32|uxtheme|dwmapi|secur32|crypt32|api-ms-win-[a-z0-9-]+)\.dll$'
$crt = '^(vcruntime140|vcruntime140_1|msvcp140)\.dll$'
foreach ($d in $deps) {
    if ($d -imatch $system) { continue }
    if ($d -imatch $crt) {
        # The newest redistributable (never older than the toolset that linked us).
        $src = Get-ChildItem -Recurse "$vs\VC\Redist\MSVC" -Filter $d |
            Where-Object { $_.FullName -match '\\MSVC\\(\d+\.\d+\.\d+)\\x64\\Microsoft\.VC\d+\.CRT\\' -and $_.FullName -notmatch 'onecore|debug' } |
            Sort-Object { [version]([regex]::Match($_.FullName, '\\MSVC\\(\d+\.\d+\.\d+)\\').Groups[1].Value) } -Descending |
            Select-Object -First 1
        if (-not $src) { throw "no redistributable $d found" }
        Copy-Item $src.FullName $out
        Write-Host "shipped $d from $($src.FullName)"
        continue
    }
    throw "unexpected DLL dependency $d"
}

# Safety net: nothing that looks like original game data goes in.
$bad = Get-ChildItem -Recurse -File $out | Where-Object {
    ($_.Extension -imatch '^\.(bin|u1|16|4|com)$') -or ($_.Extension -ieq '.exe' -and $_.Name -ne 'ultima1-engine.exe')
}
if ($bad) { $bad | ForEach-Object { Write-Host $_.FullName }; throw 'refusing to package game data' }

$zip = Join-Path 'build' "$name.zip"
if (Test-Path $zip) { Remove-Item $zip }
Compress-Archive -Path $out -DestinationPath $zip
Write-Host "built $zip"
