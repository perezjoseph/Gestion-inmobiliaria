# Agent Learnings

> Shared notepad for all background agents. Append-only — never overwrite.
> Format: ## YYYY-MM-DD [agent-name] Task: description


## 2026-04-09 [orchestrator] Task: Frontend perPage=1000 optimization

Replaced all 5 instances of `perPage=1000` across 3 frontend files:
- `contratos.rs`: Added proper server-side pagination (page/per_page state, prev/next buttons, total count). Capped dropdown fetches (propiedades, inquilinos) at perPage=100.
- `pagos.rs`: Added proper server-side pagination with filter-aware page reset. Capped contratos dropdown at perPage=100.
- `dashboard.rs`: Reduced contratos fetch from perPage=1000 to perPage=20 (only needs 5 active for display).

Also corrected stale optimization memory: backend pagination for inquilinos, contratos, pagos was already implemented but marked UNRESOLVED.
