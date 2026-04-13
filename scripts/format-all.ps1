Set-StrictMode -Off
$ErrorActionPreference = "Continue"

Write-Host "Formatting Rust files..."
cargo fmt --all 2>&1 | Out-Host

Write-Host "Formatting Kotlin files..."
$env:JAVA_HOME = Join-Path $PSScriptRoot "..\android\.jdk\jdk-17.0.18+8"
$gradlew = Join-Path $PSScriptRoot "..\android\gradlew.bat"
& $gradlew -p (Join-Path $PSScriptRoot "..\android") spotlessApply --quiet 2>&1 | Out-Host

Write-Host "Done."
