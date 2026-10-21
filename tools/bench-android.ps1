# 폰에서 hc-bench를 돌린다(USB 디버깅으로 연결된 폰 한 대).
#   ./tools/bench-android.ps1 -Docs 100000
#   ./tools/bench-android.ps1 -Docs 1000 -Quick      # 길 확인만
param([Parameter(Mandatory)][int]$Docs, [switch]$Quick)
$ErrorActionPreference = 'Stop'

function Invoke-Native {
  # cargo · adb는 진행 글을 stderr로 쓴다. Windows PowerShell 5.1은 출력을 파일 등으로 돌릴 때 그 줄을 오류 기록으로 바꿔
  # 'Stop'에서 멈추므로, 부르는 동안만 Continue로 두고 성패는 종료 코드로 본다.
  param([string]$What, [scriptblock]$Run)
  $ErrorActionPreference = 'Continue'
  & $Run
  if ($LASTEXITCODE -ne 0) { throw "$What 실패(종료 코드 $LASTEXITCODE)" }
}

$repo = Split-Path $PSScriptRoot -Parent
$sdk = if ($env:ANDROID_HOME) { $env:ANDROID_HOME } else { Join-Path $env:LOCALAPPDATA 'Android\Sdk' }
$adb = Join-Path $sdk 'platform-tools\adb.exe'
$ndk = Join-Path $sdk 'ndk\28.2.13676358\toolchains\llvm\prebuilt\windows-x86_64\bin'
if (-not (Test-Path -LiteralPath $ndk)) { throw "Android NDK 28.2.13676358이 필요합니다: $ndk" }
$target = 'aarch64-linux-android'
$key = $target.Replace('-', '_')
$linker = Join-Path $ndk "$($target)24-clang.cmd"
[Environment]::SetEnvironmentVariable("CC_$key", $linker, 'Process')
[Environment]::SetEnvironmentVariable("AR_$key", (Join-Path $ndk 'llvm-ar.exe'), 'Process')
[Environment]::SetEnvironmentVariable("CARGO_TARGET_$($key.ToUpper())_LINKER", $linker, 'Process')
Push-Location $repo  # rust-toolchain.toml(1.94.1)을 따르도록 저장소 안에서 짓는다
try { Invoke-Native 'hc-bench(Android) 빌드' { & cargo build --locked --release -p history_bench --bin hc-bench --target $target } } finally { Pop-Location }

$devices = @(Invoke-Native 'adb devices' { & $adb devices } | Select-String -Pattern "`tdevice$")
if ($devices.Count -ne 1) { throw "연결된 폰이 한 대여야 합니다(지금 $($devices.Count)대). USB 디버깅을 켜고 다시." }
$model = ((Invoke-Native '기종 읽기' { & $adb shell getprop ro.product.model }) | Out-String).Trim() -replace '[^\w-]', '_'
Invoke-Native '폰에 올리기' { & $adb push (Join-Path $repo "target\$target\release\hc-bench") /data/local/tmp/hc-bench } | Out-Null
Invoke-Native '실행 권한' { & $adb shell chmod 755 /data/local/tmp/hc-bench }
$quickArg = if ($Quick) { '--quick' } else { '' }
Invoke-Native '폰 측정' { & $adb shell "cd /data/local/tmp && ./hc-bench run --docs $Docs --dir /data/local/tmp/hcb-work --device $model --out /data/local/tmp/hcb-out $quickArg" }
$dest = Join-Path $repo 'bench\results'
New-Item -ItemType Directory -Force -Path $dest | Out-Null
Invoke-Native '결과 받기' { & $adb pull /data/local/tmp/hcb-out/. $dest } | Out-Null
Invoke-Native '폰 정리' { & $adb shell rm -rf /data/local/tmp/hcb-work /data/local/tmp/hcb-out }
Write-Output "결과: $dest"
