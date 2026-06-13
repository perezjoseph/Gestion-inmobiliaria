# Fowler Harness Engineering Patterns

Distilled from "Harness engineering for coding agent users" (Martin Fowler / Birgitta Böckeler, April 2026).

## Feedforward and Feedback

To harness a coding agent, combine both control types:

- **Guides (feedforward):** Anticipate behavior and steer before the agent acts. Increase probability of good results on first attempt.
- **Sensors (feedback):** Observe after the agent acts and enable self-correction. Powerful when they produce signals optimized for LLM consumption (e.g., custom linter messages with fix instructions).

Separately, you get either an agent that keeps repeating mistakes (feedback-only) or one that encodes rules but never verifies them (feedforward-only).

## Computational vs Inferential

Two execution types for both guides and sensors:

- **Computational:** Deterministic, fast, run by CPU. Tests, linters, type checkers, structural analysis. Milliseconds to seconds; results are reliable.
- **Inferential:** Semantic analysis, AI code review, LLM-as-judge. Slower, more expensive, non-deterministic. But allows rich guidance and additional semantic judgment.

Computational sensors are cheap enough to run on every change. Inferential controls add trust when used with a strong model but cannot replace deterministic checks.

## The Steering Loop

The human's job: iterate on the harness. When an issue happens multiple times:

1. Improve feedforward controls to make the issue less probable
2. Improve feedback sensors to catch it earlier
3. Use AI to help build custom controls (structural tests, linters, how-to guides)

## Keep Quality Left

Distribute checks across the development timeline by cost, speed, and criticality:

- **Before commit (fast):** LSP, linters, fast test suites, basic code review
- **Post-integration (expensive):** Mutation testing, broad architecture review, detailed code review

The earlier you find issues, the cheaper they are to fix.

## Regulation Categories

### Maintainability Harness

Regulates internal code quality. Easiest to build — lots of pre-existing tooling.

Computational sensors catch: duplicate code, cyclomatic complexity, missing coverage, architectural drift, style violations.

LLMs partially address: semantically duplicate code, redundant tests, brute-force fixes, over-engineering. But expensively and probabilistically.

Neither catches reliably: misdiagnosis, overengineering, misunderstood instructions.

### Architecture Fitness Harness

Guides and sensors that define and check architecture characteristics (fitness functions):

- Performance requirements + performance tests
- Observability conventions + debugging instructions
- Module boundary enforcement via structural tests

### Behaviour Harness

The hardest category. How do we verify the application functionally behaves correctly?

Current state of the art: functional spec (feedforward) + AI-generated test suite (feedback) + manual testing. Puts a lot of faith in AI-generated tests — not good enough yet.

## Harnessability

Not every codebase is equally amenable to harnessing:

- Strongly typed languages → type-checking as a sensor
- Clear module boundaries → architectural constraint rules
- Frameworks that abstract details → implicitly increase agent success

Greenfield teams can bake harnessability in from day one. Legacy teams face the harder problem: the harness is most needed where it is hardest to build.

## Ashby's Law of Requisite Variety

A regulator must have at least as much variety as the system it governs. An LLM can produce almost anything, but committing to a topology (Rust workspace, TypeScript sidecar, K8s manifests) narrows the output space, making a comprehensive harness achievable.

Defining topologies is a variety-reduction move. Pre-defined sensor suites per topology make governance tractable.

## Harness Templates

Common service topologies (CRUD API, event processor, data dashboard) can be codified as harness templates: a bundle of guides and sensors that leash an agent to the structure, conventions, and tech stack of a topology.

Teams may start picking tech stacks partly based on what harnesses are already available.
