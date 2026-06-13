# Dismiss Alert #42 - False Positive (Test File)

## Repository

`perezjoseph/Gestion-inmobiliaria`

## Action

Dismiss code scanning alert #42 as a false positive because it is in a test file.

## Commands

### 1. Verify the alert (optional, for confirmation)

```powershell
gh api "/repos/perezjoseph/Gestion-inmobiliaria/code-scanning/alerts/42"
```

### 2. Dismiss the alert

```powershell
gh api -X PATCH "/repos/perezjoseph/Gestion-inmobiliaria/code-scanning/alerts/42" `
  -f state=dismissed `
  -f dismissed_reason=used_in_tests `
  -f dismissed_comment="False positive - this alert is in a test file"
```

## Notes

- `dismissed_reason` is set to `used_in_tests` which is the appropriate reason for alerts triggered by code in test files.
- Valid dismiss reasons are: `false_positive`, `won't_fix`, `used_in_tests`.
- The REST API endpoint used is `/repos/{owner}/{repo}/code-scanning/alerts/{alert_number}`.
