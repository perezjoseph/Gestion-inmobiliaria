---
inclusion: fileMatch
fileMatchPattern: ["android/**/*.kt", "android/**/*.kts", "android/gradle/**/*"]
---

# Android

## Architecture
- Multi-module: `app`, `core:*`, `feature:*`. No feature-to-feature deps. `core:model` is pure Kotlin JVM.
- Hilt DI. `StateFlow` (never LiveData). `sealed class` for state, `data class` for DTOs.
- Screen -> ViewModel -> Repository -> Network/Database.
- `collectAsStateWithLifecycle()` in Compose. `stringResource()` for text (Spanish only).
- `BigDecimal` for currency. DD/MM/YYYY dates. Version catalog only. Pure Compose, no Fragments.
- JUnit 5 + Kotest + Turbine + Google Truth for testing.

## Compose Anti-Patterns
- Never pass `List<T>` as composable params. Standard collections are unstable (Compose can't prove they haven't changed), forcing recomposition even when data is identical. Use `ImmutableList` from kotlinx-collections-immutable.
- Never read rapidly-changing state (scroll position, animation progress) high in the composable tree. It forces the entire subtree to recompose on every frame. Push reads to the smallest composable that needs them, use `derivedStateOf` to convert rapid changes into discrete signals.
- Never create lambdas inline in composables without `remember`. Each recomposition creates a new lambda instance (new object), triggering child recomposition. Stabilize with `remember { { viewModel.onAction() } }` or method references.
- Never use multiple booleans for UI state (`isLoading`, `isError`, `isSuccess`). N booleans = 2^N combinations, most invalid. Use `sealed interface` state machines where only valid states compile.
- Never mutate collections in-place (`list.add()`). Compose uses reference equality; same reference = no recomposition. Always create new: `list = list + newItem` or use `mutableStateListOf()`.
- Never change `LaunchedEffect` keys from rapidly-toggling state. Key change cancels the running coroutine at the next frame boundary. Use `Unit` key + `snapshotFlow` to decouple observation from lifecycle.
- Never profile performance on debug builds (10x slower due to disabled compiler optimizations). Always `assembleRelease` on a real device.
