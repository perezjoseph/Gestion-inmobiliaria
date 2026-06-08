# Post-build script: extracts Trunk's inline module script into an external file
# so that the CSP can remain strict (no 'unsafe-inline' for scripts).

$DistDir = if ($env:TRUNK_STAGING_DIR) { $env:TRUNK_STAGING_DIR } else { "dist" }

$indexPath = Join-Path $DistDir "index.html"
if (-not (Test-Path $indexPath)) {
    Write-Error "index.html not found at $indexPath"
    exit 1
}

$html = Get-Content $indexPath -Raw

# Match the Trunk-generated inline module script (between <script type="module"> and </script>)
$pattern = '(?s)<script type="module">\s*(import init.*?dispatchEvent\(new CustomEvent\("TrunkApplicationStarted".*?\).*?)\s*</script>'

if ($html -match $pattern) {
    $inlineScript = $Matches[1]
    
    # Write the extracted script to an external file
    $scriptPath = Join-Path $DistDir "trunk-init.js"
    Set-Content -Path $scriptPath -Value $inlineScript -NoNewline
    
    # Replace the inline script tag with an external module script tag
    $html = $html -replace $pattern, '<script type="module" src="/trunk-init.js"></script>'
    
    Set-Content -Path $indexPath -Value $html -NoNewline
    Write-Output "Extracted inline module script to $scriptPath"
} else {
    Write-Warning "No inline module script found to extract"
}
