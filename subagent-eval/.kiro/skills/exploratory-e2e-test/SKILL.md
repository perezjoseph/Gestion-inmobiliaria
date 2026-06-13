---
name: exploratory-e2e-test
description: >
  Drive a comprehensive exploratory end-to-end test of a running web application
  using Playwright CLI. Systematically navigates every page, clicks every button,
  link, and interactive element, submits forms with empty/invalid/valid data,
  monitors console and network errors after each interaction, probes edge cases
  (404 routes, unauthenticated access to protected routes, mobile viewport), and
  compiles a severity-ranked bug report. Use whenever the user asks to test a web
  app, do QA, find bugs, click through a site, smoke-test a deployment, verify a
  UI works, run an exploratory or end-to-end test, or check a staging/production
  URL — even if they don't say "Playwright" or "E2E" explicitly.
license: MIT
allowed-tools: Read Write Shell
metadata:
  author: Joseph Perez
  version: "1.0.0"
  domain: testing
  triggers: e2e, end-to-end, exploratory test, QA, smoke test, test the app, find bugs, click through, playwright, UI test, verify the site, check the deployment
  role: specialist
  scope: verification
  output-format: report
---

# Exploratory E2E Test

Drive a real browser through a web app to discover bugs that unit tests miss:
broken navigation, silent form failures, missing error states, console errors,
dead API endpoints, permission glitches, and responsive breakage. The value of
this skill is the *discipline* — visiting every route, exercising every control,
and checking the console after each action — plus a consistent bug-report format
the user can act on.

## Why This Exists

A model asked to "test the app" tends to click a few things and declare success.
Real exploratory testing is exhaustive and skeptical: every interactive element is
a hypothesis ("this button does what its label says"), and your job is to try to
falsify it. Bugs hide in the gaps — a form that submits but shows no error, a nav
link that 404s, an admin page that returns 403 to an admin, a console error nobody
sees. This skill forces you to look in those gaps.

## Tooling: Playwright CLI

Use `playwright-cli` (or `npx playwright-cli` if not global). The core loop is
**act, then snapshot**. The snapshot returns an accessibility tree with element
refs (`e12`, `e44`, …) you use to interact.

```bash
playwright-cli open <url>           # launch + navigate
playwright-cli snapshot             # accessibility tree with refs (your map)
playwright-cli click e12            # click by ref
playwright-cli fill e15 "text"      # fill an input
playwright-cli select e9 "value"    # pick a dropdown option
playwright-cli console error        # console messages (filter by level)
playwright-cli network              # all network requests + status codes
playwright-cli go-back / goto <url> # navigation
playwright-cli resize 375 812       # mobile viewport
playwright-cli state-save auth.json # persist login; state-load to restore
playwright-cli close                # always close when done
```

Key habits that save time and avoid confusion:
- **Snapshots cost tokens.** After clicking into a known layout, snapshot a
  subtree (`playwright-cli snapshot "#main"` or `snapshot --depth=4`) instead of
  the whole page. Re-snapshot only when the DOM changed.
- **Refs go stale** after navigation or DOM updates. If a click fails with "ref
  not found," take a fresh snapshot and use the new ref.
- **Read the console after every meaningful interaction.** A page that looks fine
  can be throwing errors. `playwright-cli network` reveals failed API calls (4xx/5xx)
  that explain empty or broken pages.
- **On Windows `cmd`/PowerShell**, chain with `;` not `&&`, and use
  `Start-Sleep -Seconds N` (not `timeout`) if you need to wait for async loads.

## Workflow

Work through these phases in order. Keep a running list of bugs as you go — note
each one the moment you see it, don't try to remember them all for the end.

### 1. Reconnaissance
Open the app and snapshot the entry point. Read the console immediately — baseline
errors (CSP, analytics) often appear here and you'll want to distinguish them from
bugs you trigger later. Identify the app's surface: public pages, auth flow, and
(after login) the full navigation.

### 2. Public / pre-auth pages
For the landing page and any public routes, click every link and button. For
external links (GitHub, demo, social), verify the `href` and `target` via
`playwright-cli eval` rather than navigating away — opening them wastes time and
loses your place. Confirm each internal link lands on the right route.

### 3. Authentication
Test the login and registration forms as an adversary:
- **Empty submit** → expect inline validation errors on required fields.
- **Invalid formats** (bad email, short password) → expect specific messages.
- **Valid-but-wrong credentials** → expect a clear error *on the form*. Watch for
  the silent-failure bug: the API returns 401 but the UI redirects or does nothing,
  leaving the user with no feedback. Confirm with `playwright-cli network`.
- **Valid credentials** → expect redirect to the authenticated area.
After a successful login, run `playwright-cli state-save auth.json` so you can
restore the session quickly if it's lost.

### 4. Every navigation destination
Snapshot the nav and enumerate every link — sidebar, header, nested menus, footer.
Visit each one. For every page:
- Snapshot and note the visible buttons and controls.
- Click each **Create / New / Add** button → verify the form or modal opens.
- **Submit the creation form empty** → verify validation fires.
- Click **Edit** on an existing record → verify the form loads pre-filled.
- Click **Delete** → verify a confirmation dialog appears (then cancel; don't
  destroy data unless the user asked you to).
- Exercise filters, search, sort headers, and pagination → verify they respond.
- **Check the console and network after each page.** Record any 4xx/5xx or JS errors.

Don't destroy or mutate real data unless the user explicitly approves it. Prefer
opening forms, triggering validation, and cancelling out of destructive dialogs.
If you do create test records to verify a flow, note them so the user can clean up.

### 5. Feature-specific probes
Go deeper on data-driven features:
- **Dropdowns that depend on other data** (e.g. a contract form referencing
  properties and tenants) → verify the options actually loaded, not just that the
  control exists. Use `playwright-cli eval` to dump `<select>` options if the
  snapshot is ambiguous.
- **Status badges, totals, and computed fields** → sanity-check the values.
- **Reports / exports / charts** → generate them and confirm they render with data,
  not a blank screen.

### 6. Edge cases and error states
- **Non-existent route** (`/nonexistent`) → expect a 404 page, not a crash.
- **Protected route without auth** → clear cookies and localStorage, navigate to a
  private route, expect a redirect to login.
- **Mobile viewport** → `resize 375 812`, reload, and verify the layout adapts.
  Test the hamburger/menu toggle actually opens navigation — overlay/z-index bugs
  that make controls unclickable are common here and Playwright will report
  "intercepts pointer events" when you hit one.

### 7. Report
Compile findings into the format below. Restore the viewport, close the browser
(`playwright-cli close`), and clean up any artifacts you created (e.g. `auth.json`,
test records).

## Bug Report Format

Lead with a one-line summary (how many bugs, rough severity spread). Then one table
per bug, ordered by severity (Critical → High → Medium → Low). Use this template:

```markdown
### Bug #N — <short title>
| Field | Value |
|---|---|
| **Page/URL** | <where it happened> |
| **Action** | <exact steps / what was clicked or submitted> |
| **Expected** | <what should have happened> |
| **Actual** | <what happened: error text, blank screen, console/network error, status code> |
| **Severity** | **<Critical/High/Medium/Low>** — <one-line justification> |
```

Severity guidance:
- **Critical** — data loss, security hole, app unusable, can't log in at all.
- **High** — a core feature is broken or a whole page fails (e.g. admin gets 403 on
  an admin page, mobile nav unreachable, login fails silently with no feedback).
- **Medium** — a feature is degraded but has workarounds (e.g. a dropdown can't load
  its data, a backend returns a malformed empty response).
- **Low** — cosmetic or non-blocking (duplicate UI element, a single broken image,
  analytics blocked by CSP).

Close with a **"Tested & Working"** table listing every page/flow you verified that
behaved correctly. This is as valuable as the bug list — it tells the user exactly
what coverage you achieved and what they can trust.

## Anti-patterns

- **Declaring success after a few clicks.** Coverage is the point. If there are 25
  nav destinations, visit all 25.
- **Ignoring the console.** A green-looking page throwing red console errors is a
  finding, not a pass.
- **Snapshotting the whole page every step.** Scope your snapshots; refs are cheap
  to refresh when you actually need them.
- **Navigating to external links.** Verify hrefs in place instead.
- **Destroying real data to "test delete."** Confirm the dialog appears, then cancel.
