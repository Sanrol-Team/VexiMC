# VeixEngine：从工作区启动 Minecraft + Veix Agent（需已下载客户端与依赖）
# 用法（在 F:\VeixEngine\26 下）:
#   .\veixengine\tools\run_game_test.ps1 -GameDir "D:\Games\MC\.minecraft"
#   .\veixengine\tools\run_game_test.ps1 -GameDir "..." -Version "1.21.4" -DryRun

param(
    [Parameter(Mandatory = $true)]
    [string] $GameDir,
    [string] $Version = "",
    [switch] $DryRun,
    [switch] $VeixItems,
    [switch] $VeixTestMod
)

$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$Exe = Join-Path $Root "target\debug\veix-mc-launch.exe"
if (-not (Test-Path $Exe)) {
    Write-Host "Missing builder output: $Exe  (run: cargo build -p veix-mc-launch)" -ForegroundColor Red
    exit 1
}

$gd = (Resolve-Path -LiteralPath $GameDir).Path
if (-not $Version) {
    $Version = "26.1.2"
}

$json = Join-Path $gd "$Version.json"
if (-not (Test-Path $json)) {
    Write-Host "Missing version JSON: $json" -ForegroundColor Red
    Write-Host 'Use HMCL/PCL to download the version first. game dir must contain VERSION.json matching --version.' -ForegroundColor Yellow
    exit 1
}

$launchArgs = @("--game-dir", $gd, "--version", $Version)
if ($DryRun) { $launchArgs += "--dry-run" }
if ($VeixItems) { $launchArgs += "--veix-items" }
if ($VeixTestMod) { $launchArgs += "--veix-test-mod" }

Write-Host ("Run: " + $Exe + " " + ($launchArgs -join " ")) -ForegroundColor Cyan
& $Exe @launchArgs
exit $LASTEXITCODE
