# Windows counterpart of build-engine.sh: builds yue2.cpp's yue-server.exe at
# the pinned commit with the Vulkan backend (NVIDIA, AMD and Intel cards) and
# copies it with its DLLs and the Visual C++ runtime into
# src-tauri\resources\yue2-cpp. Needs Visual Studio C++ Build Tools, CMake,
# Ninja and the Vulkan SDK (VULKAN_SDK set, glslc on PATH).
param([ValidateSet('vulkan', 'cuda', 'cpu')][string]$Backend = 'vulkan')

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$repository = 'https://github.com/timoncool/yue2.cpp.git'
$commit = 'ec43393ef2c7f604cf0b054911eb6172eec5a6ee'
# The ggml tree is deep; a short root keeps it under MAX_PATH.
$buildRoot = if ($env:ENGINE_BUILD_ROOT) { $env:ENGINE_BUILD_ROOT } else { Join-Path $root 'engine-build' }
$worktree = Join-Path $buildRoot 'yue2.cpp'
$output = Join-Path $root 'src-tauri\resources\yue2-cpp'

function Invoke-Native([string]$what, [scriptblock]$block) {
    & $block
    if ($LASTEXITCODE -ne 0) { throw "$what failed ($LASTEXITCODE)" }
}

if (-not (Test-Path (Join-Path $worktree '.git'))) {
    New-Item -ItemType Directory -Force -Path $buildRoot | Out-Null
    Invoke-Native 'clone' { git clone $repository $worktree }
}
Invoke-Native 'fetch' { git -C $worktree fetch --quiet origin $commit }
Invoke-Native 'checkout' { git -C $worktree checkout --quiet --detach $commit }
Invoke-Native 'submodules' { git -C $worktree submodule update --init --recursive }

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$vsPath = (& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath).Trim()
if (-not $vsPath) { throw 'Visual Studio C++ Build Tools are required.' }
$vcvars = Join-Path $vsPath 'VC\Auxiliary\Build\vcvars64.bat'

$backendFlags = switch ($Backend) {
    'vulkan' { '-DGGML_VULKAN=ON' }
    'cuda' { '-DGGML_CUDA=ON "-DCMAKE_CUDA_ARCHITECTURES=75-real;80-real;86-real;89-real;90-real"' }
    'cpu' { '' }
}
$build = 'build'
$parallel = [Environment]::ProcessorCount
# VSLANG=1033 keeps cl's /showIncludes in English, which Ninja parses.
$command = "set `"VSLANG=1033`" && call `"$vcvars`" >nul && cmake -S . -B $build -G Ninja -DCMAKE_BUILD_TYPE=Release -DGGML_NATIVE=OFF $backendFlags && cmake --build $build --target yue-server --parallel $parallel"
Push-Location $worktree
try { & cmd.exe /d /s /c $command | Out-Host } finally { Pop-Location }
if ($LASTEXITCODE -ne 0) { throw 'yue2.cpp build failed.' }

$server = Get-ChildItem -Path (Join-Path $worktree $build) -Recurse -Filter 'yue-server.exe' -File | Select-Object -First 1
if (-not $server) { throw 'The build finished without yue-server.exe.' }

New-Item -ItemType Directory -Force -Path $output | Out-Null
Get-ChildItem $output -Exclude '.keep' | Remove-Item -Recurse -Force
Copy-Item $server.FullName $output
Get-ChildItem -Path (Join-Path $worktree $build) -Recurse -Filter '*.dll' -File | Copy-Item -Destination $output -Force

# The Visual C++ runtime travels with the engine (app-local, as Microsoft's
# redistribution terms allow) so a clean machine needs no system install.
$redist = Get-ChildItem -Path (Join-Path $vsPath 'VC\Redist\MSVC') -Directory | Where-Object { $_.Name -match '^\d' } | Sort-Object { [version]$_.Name } | Select-Object -Last 1
if ($redist) {
    Get-ChildItem -Path (Join-Path $redist.FullName 'x64') -Directory | Where-Object { $_.Name -match 'CRT$|OpenMP$' } |
        ForEach-Object { Get-ChildItem $_.FullName -Filter '*.dll' } | Copy-Item -Destination $output -Force
}

Set-Content -Path (Join-Path $output 'runtime.commit') -Value $commit -NoNewline
Write-Host "yue-server ($commit, $Backend) is in $output"
Get-ChildItem $output | Format-Table Name, Length
