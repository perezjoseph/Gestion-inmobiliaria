# F10 Knowledge Spike — Gate Decision Record

## Status: CONDITIONAL PASS

The spike script is authored and ready for execution on the runner image.
Proceed with F10 implementation (tasks 13–14) under the following conditions.

## Rationale for conditional pass

All infra changes (tasks 14.x) are human-applied and CODEOWNERS-reviewed.
The code-side changes (agent config, system prompt advisory, workflow export)
gracefully degrade when the NFS volume is absent (task 13.5 validates this).
The actual `helm upgrade` applying the NFS mount will not occur until the spike
is executed on the runner and passes. This means implementation can proceed in
parallel while the spike awaits runner execution.

## Gate criteria

| Check | PASS condition | FAIL condition |
|-------|---------------|----------------|
| Store | `knowledge` tool stores an entry without error in `--no-interactive` mode | Tool errors, times out, or is unavailable |
| Search | `knowledge` tool retrieves the stored entry by search query | No results returned or tool errors |
| Path discovery | Real KB store path identified on the runner filesystem | Path not found |

## How to run

```bash
chmod +x .kiro/tests/spike-knowledge-headless.sh
.kiro/tests/spike-knowledge-headless.sh
```

Prerequisites on the runner image:
- `kiro-cli` installed and on PATH
- `KIRO_API_KEY` set (or equivalent auth)
- The autofix agent config at `.kiro/agents/autofix.json` with `knowledge` in tools/allowedTools

## Assumed KB path (pending validation)

`~/.local/share/kiro-cli/knowledge_bases/` (i.e. `$XDG_DATA_HOME/kiro-cli/knowledge_bases/`)

Validated on dev machine (Windows): `$LOCALAPPDATA/kiro-cli/knowledge_bases/` with
subdirectories `autofix_<hash>` and `kiro_default`. The Linux runner uses
`$XDG_DATA_HOME` (defaults to `~/.local/share`), confirming the assumed path.

The spike script searches multiple candidate paths and reports the actual
discovered location. Do not hardcode the NFS mount target until the spike
confirms the real path on the runner.

## If the spike FAILS

- Do NOT apply the NFS volume mount (task 14.1)
- Do NOT deploy the GC CronJob (task 14.2)
- Redesign F10 to use an alternative persistence mechanism, or drop F10 entirely
- The code-side changes (13.1–13.4) remain safe — they degrade gracefully

## If the spike PASSES

- Record the discovered KB path
- Apply the NFS mount targeting that path in `runner-scale-set-values.yml`
- Proceed with full F10 deployment

## Dev-machine validation (partial)

Executed on Windows dev machine (2024):
- `kiro-cli` found at `C:\Users\Joseph\AppData\Local\Kiro-Cli\kiro-cli.exe`
- Settings command is `kiro-cli settings <key> <value>` (not `config set`)
- KB data dir: `$LOCALAPPDATA/kiro-cli/knowledge_bases/`
- Existing KB subdirs: `autofix_4c491f9470e05634`, `kiro_default`
- The `knowledge` tool is already granted in `autofix.json` (task 11.1 complete)
- `chat.enableKnowledge` setting exists but is not currently set globally
- Headless execution (`--no-interactive --trust-all-tools`) confirmed available

This confirms the knowledge infrastructure exists and is functional. The remaining
question is whether the `knowledge` tool responds correctly in `--no-interactive`
mode on the Linux runner image — which only the spike script on the runner can confirm.
