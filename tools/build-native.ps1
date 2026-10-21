# HistoryCore 엔진을 짓고 Flutter 플러그인 자리(packages/history_core/native)에 놓는다.
#   ./tools/build-native.ps1                                Windows x64
#   ./tools/build-native.ps1 -Android                       + Android arm64-v8a
#   ./tools/build-native.ps1 -Android -Abi arm64-v8a,x86_64 + 에뮬레이터용
#   ./tools/build-native.ps1 -Android -SkipWindows          Android만(Linux에서는 늘 이렇게 돈다)
param(
  [switch]$Android,
  [ValidateSet('arm64-v8a', 'x86_64')][string[]]$Abi = @('arm64-v8a'),
  [switch]$SkipWindows
)
$ErrorActionPreference = 'Stop'

function Invoke-Cargo {
  # cargo는 진행 글을 stderr로 쓴다. Windows PowerShell 5.1은 출력을 파일 등으로 돌릴 때 그 줄을 오류 기록으로 바꿔
  # 'Stop'에서 멈추므로, 부르는 동안만 Continue로 두고 성패는 종료 코드로 본다.
  $ErrorActionPreference = 'Continue'
  & cargo @args
  if ($LASTEXITCODE -ne 0) { throw "cargo $($args -join ' ') 실패(종료 코드 $LASTEXITCODE)" }
}

$onWindows = $env:OS -eq 'Windows_NT'
$repo = Split-Path $PSScriptRoot -Parent
$manifest = Join-Path $repo 'Cargo.toml'
$native = Join-Path (Join-Path $repo 'packages') (Join-Path 'history_core' 'native')
$release = Join-Path (Join-Path $repo 'target') 'release'
# rustup은 지금 폴더에서 rust-toolchain.toml을 찾는다 — 어디서 불러도 1.94.1로 짓도록 저장소 안에서 돈다.
Push-Location $repo
try {

if ($onWindows -and -not $SkipWindows) {
  Invoke-Cargo build --locked --release -p history_core_ffi --manifest-path $manifest
  $dest = Join-Path $native 'windows-x64'
  New-Item -ItemType Directory -Force -Path $dest | Out-Null
  Copy-Item -LiteralPath (Join-Path $release 'history_core.dll') -Destination $dest -Force
  Write-Output "Windows: $(Join-Path $dest 'history_core.dll')"
}

if ($Android) {
  $sdk = if ($env:ANDROID_HOME) { $env:ANDROID_HOME } elseif ($onWindows) { Join-Path $env:LOCALAPPDATA 'Android\Sdk' } else { throw 'ANDROID_HOME이 필요합니다' }
  $hostTag = if ($onWindows) { 'windows-x86_64' } else { 'linux-x86_64' }
  $ndk = [IO.Path]::Combine($sdk, 'ndk', '28.2.13676358', 'toolchains', 'llvm', 'prebuilt', $hostTag, 'bin')
  if (-not (Test-Path -LiteralPath $ndk)) { throw "Android NDK 28.2.13676358이 필요합니다: $ndk" }
  $exe = if ($onWindows) { '.cmd' } else { '' }
  $arExe = if ($onWindows) { 'llvm-ar.exe' } else { 'llvm-ar' }
  foreach ($a in $Abi) {
    $target = if ($a -eq 'arm64-v8a') { 'aarch64-linux-android' } else { 'x86_64-linux-android' }
    $key = $target.Replace('-', '_')
    $linker = Join-Path $ndk "$($target)24-clang$exe"
    [Environment]::SetEnvironmentVariable("CC_$key", $linker, 'Process')
    [Environment]::SetEnvironmentVariable("AR_$key", (Join-Path $ndk $arExe), 'Process')
    [Environment]::SetEnvironmentVariable("CARGO_TARGET_$($key.ToUpper())_LINKER", $linker, 'Process')
    Invoke-Cargo build --locked --release -p history_core_ffi --target $target --manifest-path $manifest
    $dest = Join-Path (Join-Path $native 'android') $a
    New-Item -ItemType Directory -Force -Path $dest | Out-Null
    Copy-Item -LiteralPath ([IO.Path]::Combine($repo, 'target', $target, 'release', 'libhistory_core.so')) -Destination $dest -Force
    Write-Output "Android: $(Join-Path $dest 'libhistory_core.so')"
  }
}

} finally {
  Pop-Location
}
