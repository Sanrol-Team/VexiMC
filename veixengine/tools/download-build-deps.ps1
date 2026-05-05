#Requires -Version 5.1
<#
.SYNOPSIS
  联网下载 VeixEngine 构建依赖：ASM（Maven Central）与 Eclipse Temurin JDK（Adoptium）。
.DESCRIPTION
  - ASM 保存到 agent-java/deps/asm-<version>.jar（build.rs 优先使用，免重复下载）
  - JDK 解压到 tools/jdk（build.rs / 启动器可通过 JAVA_HOME 指向此处）
  用法（在 veixengine 目录或任意路径）:
    powershell -ExecutionPolicy Bypass -File tools/download-build-deps.ps1
    powershell -File tools/download-build-deps.ps1 -SkipJdk
    powershell -File tools/download-build-deps.ps1 -JdkMajor 21 -AsmVersion 9.7
#>
param(
    [int]$JdkMajor = 21,
    [string]$AsmVersion = "9.7",
    [switch]$SkipJdk,
    [switch]$SkipAsm
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$ToolsDir = $PSScriptRoot
$VeixRoot = Split-Path -LiteralPath $ToolsDir -Parent
$DepsDir = Join-Path $VeixRoot "agent-java\deps"
$JdkDest = Join-Path $VeixRoot "tools\jdk"

function Get-JdkDownloadUrl {
    param([int]$Major)
    $isWin = $false
    if ($null -ne (Get-Variable -Name IsWindows -ErrorAction SilentlyContinue)) {
        $isWin = $IsWindows
    } else {
        $isWin = $env:OS -match "Windows"
    }
    $plat = if ($isWin) { "windows" } else { "linux" }
    if (-not [Environment]::Is64BitOperatingSystem) {
        throw "仅支持 64 位 JDK 自动下载"
    }
    $arch = "x64"
    return "https://api.adoptium.net/v3/binary/latest/$Major/ga/$plat/$arch/jdk/hotspot/normal/eclipse"
}

New-Item -ItemType Directory -Force -Path $DepsDir | Out-Null

if (-not $SkipAsm) {
    $asmName = "asm-$AsmVersion.jar"
    $asmPath = Join-Path $DepsDir $asmName
    $asmUrl = "https://repo1.maven.org/maven2/org/ow2/asm/asm/$AsmVersion/asm-$AsmVersion.jar"
    Write-Host "[Veix] 下载 ASM $AsmVersion ..."
    Invoke-WebRequest -Uri $asmUrl -OutFile $asmPath -UseBasicParsing
    Write-Host "[Veix] ASM -> $asmPath"
}

if (-not $SkipJdk) {
    $url = Get-JdkDownloadUrl -Major $JdkMajor
    $zipPath = Join-Path ([System.IO.Path]::GetTempPath()) ("veix-temurin-jdk-{0}.zip" -f [Guid]::NewGuid().ToString("N"))
    Write-Host "[Veix] 下载 Temurin JDK $JdkMajor ($url) ..."
    Invoke-WebRequest -Uri $url -OutFile $zipPath -MaximumRedirection 10 -UseBasicParsing

    $stage = Join-Path $ToolsDir ("_jdk_stage_" + [Guid]::NewGuid().ToString("N"))
    if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage -Recurse -Force }
    New-Item -ItemType Directory -Path $stage | Out-Null
    Write-Host "[Veix] 解压 JDK ..."
    Expand-Archive -LiteralPath $zipPath -DestinationPath $stage -Force

    $inner = Get-ChildItem -LiteralPath $stage -Directory | Select-Object -First 1
    if (-not $inner) { throw "JDK 压缩包内未找到目录" }

    if (Test-Path -LiteralPath $JdkDest) {
        Write-Host "[Veix] 移除旧 tools\jdk ..."
        Remove-Item -LiteralPath $JdkDest -Recurse -Force
    }
    Move-Item -LiteralPath $inner.FullName -Destination $JdkDest
    Remove-Item -LiteralPath $stage -Recurse -Force
    Remove-Item -LiteralPath $zipPath -Force -ErrorAction SilentlyContinue
    Write-Host "[Veix] JDK -> $JdkDest"
}

Write-Host ""
Write-Host "后续可选（当前会话）："
Write-Host "  `$env:JAVA_HOME='$JdkDest'"
Write-Host "  `$env:Path='$JdkDest\bin;' + `$env:Path"
Write-Host ""
Write-Host "然后: cd `"$VeixRoot`" ; cargo build -p veix-mc-launch"
