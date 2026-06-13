# Dismiss code scanning alert #42 as false positive (test file)
# Repository: perezjoseph/Gestion-inmobiliaria

# Step 1: Get alert details to confirm it exists and review before dismissing
gh api "/repos/perezjoseph/Gestion-inmobiliaria/code-scanning/alerts/42"

# Step 2: Dismiss alert #42 as false positive with reason that it's in a test file
gh api -X PATCH "/repos/perezjoseph/Gestion-inmobiliaria/code-scanning/alerts/42" `
  -f state=dismissed `
  -f dismissed_reason=used_in_tests `
  -f dismissed_comment="False positive - this alert is in a test file"
