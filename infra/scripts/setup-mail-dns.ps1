# Setup DNS records for docker-mailserver on Cloudflare
# Usage: $env:CF_API_TOKEN = "your-token"; .\infra\scripts\setup-mail-dns.ps1
#
# Prerequisites:
#   - Cloudflare API token with Zone:DNS:Edit and Zone:Zone:Read permissions
#   - Set as $env:CF_API_TOKEN before running

$ErrorActionPreference = "Stop"

if (-not $env:CF_API_TOKEN) {
    Write-Error "Set `$env:CF_API_TOKEN first. Example: `$env:CF_API_TOKEN = 'your-token-here'"
    exit 1
}

$DOMAIN = "myhomeva.us"
$MAIL_HOST = "mail.myhomeva.us"
$PUBLIC_IP = "98.169.121.104"
$DKIM_KEY = "v=DKIM1; k=rsa; p=MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA2tQ0U/oFFdSHh/0T0rsflfSM1XOMA2cPFTbS8DGcpiAJ9jQWnPceEjO9qkUpKOItBrb+gahz+v/h+pgXPqK8a9a417ycvbDGEAXHLKadfO5QeiB9qEbzfm3zeXwTWi2P9MVX9EOab8g+LhnNyqalVj4kdUKLSHMGJS7jO7VjbW1xT5ubvhji2UuxWiLhhEn0RGUu18KlxAajDclxUmBZulygN38Cqv8Y/h7Y9KXFj+EqPYRgTRj7mKwcGF5xTfYYzqfsPU2sdPcUVZL5Qq8I0BsKp7M2c6e1OBQ1tL3MXYezTRkwx6uhV38qmPlJjs5hOVgnWR9ePJGffsH3HDIx3wIDAQAB"

$headers = @{
    "Authorization" = "Bearer $env:CF_API_TOKEN"
    "Content-Type"  = "application/json"
}

# --- Find Zone ID ---
Write-Host "Looking up zone ID for $DOMAIN..." -ForegroundColor Cyan
try {
    $zonesResp = Invoke-RestMethod -Uri "https://api.cloudflare.com/client/v4/zones?name=$DOMAIN" -Headers $headers
} catch {
    Write-Host "HTTP Error: $($_.Exception.Message)" -ForegroundColor Red
    $reader = [System.IO.StreamReader]::new($_.Exception.Response.GetResponseStream())
    Write-Host "Response: $($reader.ReadToEnd())" -ForegroundColor Red
    exit 1
}
if (-not $zonesResp.success -or $zonesResp.result.Count -eq 0) {
    Write-Host "Response: $($zonesResp | ConvertTo-Json -Depth 3)" -ForegroundColor Red
    Write-Error "Could not find zone for $DOMAIN. Check token permissions."
    exit 1
}
$ZONE_ID = $zonesResp.result[0].id
Write-Host "  Zone ID: $ZONE_ID" -ForegroundColor Green

$base = "https://api.cloudflare.com/client/v4/zones/$ZONE_ID/dns_records"

function New-DnsRecord($type, $name, $content, $priority = $null, $proxied = $false) {
    $body = @{ type = $type; name = $name; content = $content; ttl = 300; proxied = $proxied }
    if ($priority -ne $null) { $body.priority = $priority }

    Write-Host "  Creating $type record: $name -> $($content.Substring(0, [Math]::Min(60, $content.Length)))..." -NoNewline
    try {
        $resp = Invoke-RestMethod -Uri $base -Method Post -Headers $headers -Body ($body | ConvertTo-Json -Depth 5)
        if ($resp.success) {
            Write-Host " OK" -ForegroundColor Green
        } else {
            Write-Host " FAILED: $($resp.errors | ConvertTo-Json -Compress)" -ForegroundColor Red
        }
    } catch {
        $err = $_.ErrorDetails.Message | ConvertFrom-Json -ErrorAction SilentlyContinue
        if ($err.errors[0].code -eq 81057) {
            Write-Host " EXISTS (skipped)" -ForegroundColor Yellow
        } else {
            Write-Host " ERROR: $_" -ForegroundColor Red
        }
    }
}

Write-Host "`nCreating DNS records for mail..." -ForegroundColor Cyan

# 1. A record for mail host (NOT proxied — mail needs direct TCP)
New-DnsRecord "A" $MAIL_HOST $PUBLIC_IP -proxied $false

# 2. MX record
New-DnsRecord "MX" $DOMAIN $MAIL_HOST -priority 10

# 3. SPF record
New-DnsRecord "TXT" $DOMAIN "v=spf1 a mx ip4:$PUBLIC_IP ~all"

# 4. DMARC record
New-DnsRecord "TXT" "_dmarc.$DOMAIN" "v=DMARC1;p=quarantine;rua=mailto:postmaster@$DOMAIN"

# 5. DKIM record
New-DnsRecord "TXT" "mail._domainkey.$DOMAIN" $DKIM_KEY

# 6. MTA-STS CNAME (points to mail host for TLS policy)
New-DnsRecord "CNAME" "mta-sts.$DOMAIN" $MAIL_HOST -proxied $false

# 7. Autodiscover for email clients
New-DnsRecord "CNAME" "autoconfig.$DOMAIN" $MAIL_HOST -proxied $false

Write-Host "`nDone! Verify with:" -ForegroundColor Cyan
Write-Host "  nslookup -type=MX $DOMAIN"
Write-Host "  nslookup -type=TXT $DOMAIN"
Write-Host "  nslookup -type=TXT mail._domainkey.$DOMAIN"
Write-Host "`nNote: DNS propagation may take up to 5 minutes." -ForegroundColor Yellow
