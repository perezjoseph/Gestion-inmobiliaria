$ErrorActionPreference = 'Stop'

# Pattern: matches both "realestate_backend::config::ChatbotEnvConfig {" and "ChatbotEnvConfig {"
# variants, captures the leading qualifier (if any), and matches every body shape we use.
# All current test fixtures share the same 5-field body verbatim — only whitespace differs.
$pattern = '(?ms)(realestate_backend::config::)?ChatbotEnvConfig\s*\{\s*baileys_service_url:\s*"http://baileys:3100"\.to_string\(\),\s*baileys_internal_token:\s*"a\]3kF9#mP7vL2nQ8wR5xT0yU4zA1bC6dE"\.to_string\(\),\s*ovms_endpoint:\s*"http://ovms:8000/v3"\.to_string\(\),\s*ovms_chat_model:\s*"Qwen3-30B-A3B-Instruct-2507-int4-ov"\.to_string\(\),\s*ai_chat_timeout_secs:\s*30,\s*\}'

$replacement = '${1}ChatbotEnvConfig::for_testing()'

$updated = 0
Get-ChildItem -Recurse -Path 'backend\tests' -Include *.rs | ForEach-Object {
    $p = $_.FullName
    $c = Get-Content -Raw -LiteralPath $p
    $n = [regex]::Replace($c, $pattern, $replacement)
    if ($n -ne $c) {
        Set-Content -LiteralPath $p -Value $n -NoNewline
        Write-Host "Updated: $p"
        $updated++
    }
}
Write-Host ""
Write-Host "Total files updated: $updated"
