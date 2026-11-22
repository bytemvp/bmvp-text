# PowerShell build and package script for Windows
param (
    [string]$Target = "x86_64-pc-windows-msvc",
    [string]$Version = "0.2.0",
    [string]$ArtifactName = "bmvp-text-windows-x64",
    [switch]$SkipBuild = $false,
    [switch]$BuildInstaller = $false
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

if (-not $SkipBuild) {
    Write-Host "==> [1/3] Compiling release binary for $Target..." -ForegroundColor Cyan
    cargo build --release --target $Target
} else {
    Write-Host "==> [1/3] Using prebuilt release binary for $Target..." -ForegroundColor Cyan
}

$BinaryPath = "target\$Target\release\bmvp-text.exe"
if (-not (Test-Path $BinaryPath)) {
    throw "Release binary not found at: $BinaryPath"
}

$DistDir = "dist\$ArtifactName"
if (Test-Path $DistDir) {
    Remove-Item -Recurse -Force $DistDir
}
New-Item -ItemType Directory -Path $DistDir -Force | Out-Null

Write-Host "==> [2/3] Preparing portable package directory..." -ForegroundColor Cyan
Copy-Item $BinaryPath "$DistDir\"
Copy-Item "README.md" "$DistDir\" -ErrorAction SilentlyContinue
Copy-Item "LICENSE" "$DistDir\" -ErrorAction SilentlyContinue

$ZipFile = "dist\$ArtifactName.zip"
if (Test-Path $ZipFile) {
    Remove-Item -Force $ZipFile
}

# Use tar or .NET ZipFile to avoid PowerShell Write-Progress console buffer issues in headless environments
if (Get-Command tar.exe -ErrorAction SilentlyContinue) {
    tar.exe -a -c -f $ZipFile -C dist $ArtifactName
} else {
    [System.IO.Compression.ZipFile]::CreateFromDirectory((Resolve-Path $DistDir).Path, (Resolve-Path "dist").Path + "\$ArtifactName.zip")
}

Write-Host "==> Portable zip created at: $ZipFile" -ForegroundColor Green

if ($BuildInstaller) {
    Write-Host "==> [3/3] Compiling Inno Setup installer..." -ForegroundColor Cyan
    $InnoCompiler = "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
    if (Test-Path $InnoCompiler) {
        $SourceExe = "..\..\target\$Target\release\bmvp-text.exe"
        & $InnoCompiler "/DSourceExe=$SourceExe" "/DMyAppVersion=$Version" "package\windows\installer.iss"
        Write-Host "==> Setup executable created in dist\" -ForegroundColor Green
    } else {
        Write-Warning "Inno Setup compiler (ISCC.exe) not found. Skipping installer exe creation."
    }
}

Write-Host "==> Windows build completed successfully!" -ForegroundColor Green
