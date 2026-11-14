# AI-Native Readiness Report

**Repo:** ripwire-broker (a git worktree on branch `ai-native/10-skills` @ 4a4458e, clean before this report). GitHub settings (branch protection, rulesets, environments, secret scanning) were not queried: where an item depends on them it says so and marks them unverified. Previous run: `git show 17eddd7:docs/AINE-REPORT.md` (26 passed, 20 failed).
**Date:** 2026-11-14
**Stack:** Rust (edition 2024, toolchain 1.98.1 pinned) on Cargo, Tokio, `rust-mcp-sdk =2.0.0`; tests via `cargo nextest` with proptest; an optional `online` feature (reqwest + rustls). A Claude Code plugin/mod in TypeScript under `integrations/claude-code/` (tested by `claude plugin test`), a Codex integration under `integrations/codex/`. `scripts/check.sh` runs every PR check; CI on GitHub Actions (`rust.yml`, `release.yml`, `supply-chain.yml`) calls it per job. Dependabot, cargo-deny, gitleaks. Agent instructions in `AGENTS.md` (+ `CLAUDE.md`), skills in `.agents/skills/`. Specs and decision log in Portuguese under `spec/` and `docs/`.

**Result:** 44 passed, 2 failed, 0 not applicable

---

**How to read this.** Every item is a numbered question about this repo, answered from evidence in this repo. `Proof` is what was actually found — a path, a file, a command and its output. An item with no proof is not an answer. The italic line under each question says how to check it; it stays in the report so the next run checks the same way.

**FAIL vs N/A.** `FAIL` means the practice would help here and it is missing — including when a whole artifact is missing, so several questions about it fail together with a one-line proof pointing at the item that found it missing. `N/A` means the question could never apply to a project of this kind, and the proof says why. "It does not exist" is a FAIL, never an N/A.

`Priority` is fixed by the template, not by the person filling it in. **High** means an agent working here will hit this on day one, or the risk is real. **Low** means it makes things better but nothing breaks without it.

---

## 1. Context

- [x] 1. Does an agent instruction file exist (AGENTS.md, CLAUDE.md, .cursorrules, .github/copilot-instructions.md, or this ecosystem's equivalent), and what does it point at?
  *Look for a file whose purpose is to be loaded into a coding agent's context at the start of every session. That file is the **entry point**. Then follow its pointers: instructions are commonly spread across several files, with the entry point routing to them — "commands are in docs/development.md", "conventions in .claude/rules/". The entry point plus every file it points at is the **instruction set**, and every other agent-instruction question is answered against that whole set, so list it here in the proof. Follow pointers as deep as they go, but a pointer only counts when it names a target and says what is in it: "test commands are in CONTRIBUTING.md" counts, a bare "see the docs" does not, and a file nothing points at is outside the set however good it is. A README is not an entry point, however good — it gets its credit at the README item and the specs-and-architecture item — but a README the entry point explicitly routes to is inside the set. Vendored skill libraries are not instructions either.*
  - Status: PASS
  - Proof: Entry point: root `AGENTS.md` (5,285 B, tracked), loaded by Claude Code through `CLAUDE.md` (11 B, a single line `@AGENTS.md`). `AGENTS.md:6-7` says `integrations/codex/AGENTS.md` is product content, not instructions for this repo. Instruction set, from the pointers in `AGENTS.md` §"Where things are" (lines 74-85) and §Verify (line 56), each naming its target and what is in it: `handoff.md` §"Como verificar" (line 48), §"Onde está o quê" (line 88), §"Armadilhas já pisadas" (line 417), §"Convenções" (line 479); `spec/changelog.md` (decisions D-NNN); `spec/ripwire-broker-mcp.md` (PRD, CA-01..CA-10); `spec/plan/ai-native-plan.md` (plan in progress); `README.md` §"Tests" (line 965, opt-in suites); `.agents/skills/` (`decision-entry`, `release`, `audit-rust`, symlinked from `.claude/skills/` and `.grok/skills/`). `handoff.md:4` points back at `AGENTS.md`.
  - Recommendation: —
  - Priority: High

- [x] 2. Do the agent instructions name the commands to build, test and check this project?
  *Answer this against the instruction set mapped at the entry-point item and nothing outside it. If no entry point exists, FAIL with a one-line proof pointing at that item. The commands may sit in a file the entry point routes to rather than in the entry point itself — that is progressive disclosure working as intended, and it passes. What fails is a command an agent would have to guess its way to: if the commands live only in the README or the manifest and nothing in the set points at them, that is a FAIL, because the question measures what an agent can reach without being told where to look. Name the file each command was found in.*
  - Status: PASS
  - Proof: `AGENTS.md:47-51` names `scripts/check.sh` (every PR check, CI order), `scripts/check.sh default|online|plugin|secrets` (one target) and `cargo lint` / `cargo lint-online`; `AGENTS.md:53-56` names the tools `all` needs and routes the opt-in suites to `README.md` §"Tests", which lists `cargo test`, `cargo test --features online` and the `--ignored` runs with their env vars (`README.md:977-983`). `handoff.md` §"Como verificar" (`handoff.md:50-54`) repeats the three commands and explains CI and release. Build: `cargo build` is implied by the toolchain and appears in `README.md` §"Build and run" (line 33), which nothing in the set points at, but `scripts/check.sh` compiles every target, so no build command has to be guessed to check a change.
  - Recommendation: —
  - Priority: High

- [x] 3. Does everything the agent instructions name still exist — the commands, the paths, the libraries, and the files they point at?
  *If no entry point exists, FAIL with a one-line proof pointing at the entry-point item. Verify, do not trust: check every named command against the manifest or build file, spot-check the paths, run the cheap read-only ones. Then resolve every pointer in every file of the set — a link to a moved or deleted file is the most common rot in a multi-file instruction set, and it fails silently: the agent reads the entry point, follows nothing, and carries on without the rules.*
  - Status: PASS
  - Proof: Every repo path cited in `AGENTS.md` exists (checked one by one: the five `integrations/claude-code/...` files, `.claude-plugin/marketplace.json`, `tests/online_live.rs`, both workflows, `scripts/check.sh`, `.cargo/config.toml`, `deny.toml`, `.gitleaks.toml`, `.gitleaksignore`, `tests/fixtures/`, `spec/plan/ai-native-plan.md`, the three skills and their `.claude/skills/` / `.grok/skills/` links). The four `handoff.md` sections it names exist under those exact titles (`handoff.md:48,88,417,479`), as does `README.md` §"Tests" (line 965). The link `spec/changelog.md#d-192--credencial-do-agente-risco-aceito` resolves to `## D-192 — Credencial do agente: risco aceito` (`spec/changelog.md:8972`). Commands: the `lint`/`lint-online` aliases exist (`.cargo/config.toml:4-5`); the targets `default online plugin secrets` exist (`scripts/check.sh:18`); `scripts/check.sh all` was run on this checkout and ended `all PR checks passed` (see the test-suite item). The jobs `default` and `online` the rules forbid renaming exist (`.github/workflows/rust.yml:27-28,79-80`). `tests/agents_md.rs` (`every_path_agents_md_cites_exists`, `every_section_agents_md_points_to_has_that_title`, `every_command_agents_md_shows_runs`) holds these in CI.
  - Recommendation: —
  - Priority: High

- [x] 4. Are the agent instructions specific to this repo, rather than advice that would read the same in any codebase?
  *If no entry point exists, FAIL with a one-line proof pointing at the entry-point item. The test: could this be pasted into another project unchanged? "Write clean code" and "add tests for new features" would fit anywhere and count for nothing. Judge the whole set, but weigh the files differently: an entry point that is mostly a routing table is fine, even good, when what it routes to is specific — while generic filler in the entry point costs more than generic filler three hops down, because it is loaded into every session whether it is needed or not. Say which files carried the specifics.*
  - Status: PASS
  - Proof: `AGENTS.md` could not be pasted elsewhere: it names this repo's effects and files ("pushing or creating a `v*` tag ... because a tag runs `.github/workflows/release.yml`", "the marketplace serves `master`", `AGENTS.md:14-22`), its rules (CA-10 "No network crate in the default build", "Never rename the CI jobs `default` and `online`" (D-107), synthetic fixtures, `AGENTS.md:60-69`), the D-192 credential situation (`AGENTS.md:37-43`) and the Portuguese/English split. The routed files carry more specifics: `handoff.md` §"Armadilhas já pisadas" (zsh word splitting in mutation loops, mtime fooling cargo, worktree `CARGO_MANIFEST_DIR`), §"Convenções". No generic filler found in the entry point.
  - Recommendation: —
  - Priority: High

- [x] 5. Do the agent instructions explain where things live and how this project is laid out?
  *If no entry point exists, FAIL with a one-line proof pointing at the entry-point item. The map may live in a routed-to file. Judge coverage against the real tree, not against what the files mention: if the set maps one package well but is silent about sibling packages or directories an agent would land in, that is a FAIL with the omission named. In a multi-file set, check the routing too — a layout document nothing points at is a document the agent never opens.*
  - Status: PASS
  - Proof: `AGENTS.md:74-85` routes to `handoff.md` §"Onde está o quê" (`handoff.md:88-137`), which maps `src/` module by module (`broker.rs`, `mcp.rs`, `upstream.rs`/`supervise.rs`, `hook.rs`/`state.rs`, `worktree.rs`, `online/`, `memory/`, `eval/` + `bin/ripwire-eval.rs`, `usage.rs`, `statusline*.rs`), `tests/` (one file per public seam, `common/`, fixtures), `spec/` (PRD, changelog, `plan/`, `diagrams/`), `integrations/` (the plugin's files one by one, Codex), `scripts/`, `.cargo/config.toml`, `.agents/skills/`, `AGENTS.md`/`CLAUDE.md`, and the marketplace root. Omissions, judged minor: `examples/` (two Cargo examples, `spike.rs` and `jev_record.rs`, compiled by `cargo lint --all-targets`) is not in the map, only in `README.md:983`; `docs/` is not in the map either, though its one spec, `docs/jev-mem-prd.md`, is linked from the state table (`handoff.md:43`); `spec/old/`, `spec/prompt/` and `.config/nextest.toml` are unmapped.
  - Recommendation: —
  - Priority: High

- [x] 6. Do the agent instructions state the rules that are not obvious from the code — the things never to do here?
  *If no entry point exists, FAIL with a one-line proof pointing at the entry-point item. These are the tribal-knowledge traps: the flag that must be exactly this string, the import that breaks the build, the directory that is generated and must not be edited. Rules in a routed-to file count. Two extra checks in a multi-file set: that the entry point signposts the rules clearly enough for an agent to open them *before* it needs them, since a trap found afterwards has already been sprung; and that the files do not contradict each other, because nothing tells the agent which one wins.*
  - Status: PASS
  - Proof: The entry point carries the traps itself, before any task: §"Ask before" (`AGENTS.md:9-35`, six effect-based categories: publishing, what plugin users run, spending money, writing outside the repo, deleting real state, touching the gates) and §Rules (`AGENTS.md:58-72`: D-NNN per decision, CA-10, the required job names, TDD through the public seam, synthetic fixtures, public repo). `AGENTS.md:78-80` signposts `handoff.md` §"Armadilhas já pisadas" with what is in it ("flaky memory tests under load, pipes hiding exit codes, protected `master`"). No contradiction found between the files: `AGENTS.md:39-43` ("Commit locally; the maintainer pushes and opens the PR") matches `handoff.md` §"Convenções" (last bullet) and the `release` / `decision-entry` skills' "Where it stops"; the check commands in `AGENTS.md:47-51` and `handoff.md:50-54` agree; `AGENTS.md:81` states that `spec/changelog.md` wins over any other document.
  - Recommendation: —
  - Priority: Low

- [x] 7. Has the agent instruction file been updated recently enough to still be true, given how active the repo is?
  *If no agent instruction file exists, FAIL with a one-line proof pointing at the entry-point item. Compare the last commit touching the file against the repo's tempo, then spot-check two or three of its claims against the code — a recently touched file can still lie.*
  - Status: PASS
  - Proof: `git log -- AGENTS.md`: created 40fa8bd (2026-11-14, D-195), last touched 480945d (2026-11-14, D-201), the same day as HEAD; the repo committed 26 times on 2026-11-14. Spot-checks against the code: the check targets and aliases it shows exist and run (`scripts/check.sh:18`, `.cargo/config.toml:4-5`); `scripts/check.sh default` does fail on a network crate (`no_network_stack`, `scripts/check.sh:123-129`); the job names `default`/`online` are as stated (`rust.yml:27-28,79-80`); the three skills it lists exist. `handoff.md`'s header is current too ("até o D-201", `handoff.md:2-3`; the last changelog section is D-201, `spec/changelog.md:9660`).
  - Recommendation: —
  - Priority: Low

- [x] 8. Is the agent instruction file small enough to load into every session without crowding out the actual task?
  *If no agent instruction file exists, FAIL with a one-line proof pointing at the entry-point item. Measure it: `wc -c`, and bytes divided by four is a fair token estimate. Under ~5k tokens is comfortable; past ~10k it is eating the context window.*
  - Status: PASS
  - Proof: `wc -c AGENTS.md CLAUDE.md` → 5,285 + 11 bytes, about 1.3k tokens loaded per session; `tests/agents_md.rs:211-215` fails the build past 6,000 bytes. The large routed file, `handoff.md` (43,251 B, ~10.8k tokens), is not imported; `AGENTS.md` points at its sections to be opened on demand.
  - Recommendation: —
  - Priority: Low

- [x] 9. Can a fresh session pick up a half-finished task — is there a file or convention where progress, decisions and what is left get written down?
  *This is not about the agent instruction file. Any durable place in-flight state gets written counts: a TODO.md, a plans or notes directory, task files, linked issues, a specs folder whose entries carry progress and open questions, an agent memory file or directory, a scratchpad or working-notes convention. The test is whether a fresh session could read it and know what was decided and what is left — not what the place is called. Git history alone does not count: it records what happened, not what was decided or what remains.*
  - Status: PASS
  - Proof: `handoff.md` gives the state "Estado em 2026-11-14, até o D-201" (`handoff.md:2-3`), a phase table whose AI-native row lists what is done per step and "Faltam as partes do mantenedor das etapas 3, 5, 7 e 9" (`handoff.md:33`), "Em andamento" sections with "O que fazer, em ordem" (`handoff.md:210`) and "Pendências conhecidas" (`handoff.md:285`); `spec/plan/ai-native-plan.md` carries the steps, acceptance and "Verificação final" (line 406); `spec/changelog.md` records D-001 to D-201. The header and body now agree (the previous report's D-169-vs-D-187 contradiction is gone).
  - Recommendation: —
  - Priority: Low

- [x] 10. Is there a README that says what this project is and how to run it?
  *This is where a good README earns its credit. It needs three things: what the project is, how to run it, and how to check a change. Judge what is on the page, not the file's existence.*
  - Status: PASS
  - Proof: `README.md`: what it is (`README.md:3-9`, "A local MCP server that turns Ripwire's wide surface (33 verbs) into three task-moment tools with a token budget..."); how to run it (§Requirements line 15, §"Installing ripwire" line 22, §"Build and run" line 33); how to check a change (§"Tests" line 965, which now opens with `scripts/check.sh` and its targets, then the underlying `cargo test` commands and opt-ins).
  - Recommendation: —
  - Priority: High

## 2. Specs

- [x] 11. Is the thinking behind this system written down somewhere durable — a specs, RFC, proposals, design-doc or ADR directory, or architecture notes that record not just what but why?
  *Look for the place and for the convention: `specs/`, `docs/adr/`, `rfcs/`, `proposals/`, `.specify/`, a docs or design folder, architecture notes anywhere in the tree, or this ecosystem's equivalent. Judge substance, not location: "we use X because Y" is a decision, a list of technologies is not, an essay about specs is not a spec, and a docs folder of usage guides with no reasoning is a FAIL whose proof says what was in there instead. Other items are answered against whatever this item finds, so name it precisely — and where forward-looking specs and after-the-fact architecture records live in different places, name both, since a decision log cannot answer a question about acceptance criteria.*
  - Status: PASS
  - Proof: Forward-looking: `spec/ripwire-broker-mcp.md` (PRD: problem, principles, §4.2 non-goals, §7 architecture, CA-01..CA-10), `docs/jev-mem-prd.md` (memory PRD v0.5), one plan per phase in `spec/plan/` (newest `spec/plan/ai-native-plan.md`) and `spec/plan/xtd/`. After the fact: `spec/changelog.md`, D-001 to D-201, an index row plus a reasoned section each (e.g. D-192, `spec/changelog.md:8972`: what stays, the accepted risk, consequence, "Reabrir se"). Plus `spec/diagrams/ripwire-broker.architecture.html`.
  - Recommendation: —
  - Priority: High

- [x] 12. Does the specs directory hold recent entries, or is it an archive nobody has touched?
  *If no specs directory exists, FAIL with a one-line proof pointing at the specs-and-architecture item. Otherwise compare the newest entry's date against the repo's recent activity.*
  - Status: PASS
  - Proof: `git log -1 -- spec/` → 91000e7, 2026-11-14, the same day as HEAD. Newest entries: `spec/plan/ai-native-plan.md`, `spec/TEMPLATE.md` and changelog sections D-189 to D-201, all 2026-11-14.
  - Recommendation: —
  - Priority: Low

- [x] 13. Is there a spec template, or an SDD framework, so every spec comes out the same shape?
  *Scaffolding can exist even where no specs directory does — look for a TEMPLATE.md, a `.specify/` directory, or framework config. If neither a directory nor any scaffolding exists, FAIL.*
  - Status: PASS
  - Proof: `spec/TEMPLATE.md` (64 lines, added in 9c8fe89, D-199): title/status/version/date header, "Contexto e problema", "Objetivos e não objetivos", numbered "Critérios de aceitação" in Dado/quando/então (`spec/TEMPLATE.md:32-43`), "Falhas e comportamento obrigatório", "Como desfazer", "Decisões relacionadas", with the instruction that a section that does not apply stays with a line saying why (`spec/TEMPLATE.md:10-11`). Signposted from the instruction set: `handoff.md` §"Convenções" ("Spec nova parte do `spec/TEMPLATE.md`"). No spec has been written from it yet; it is one day old.
  - Recommendation: —
  - Priority: Low

- [x] 14. Do the specs state acceptance criteria a machine could check?
  *If no specs exist, FAIL with a one-line proof pointing at the specs-and-architecture item. Otherwise open the two newest specs and quote a criterion: "the endpoint returns 403 for expired tokens" is checkable; "the feature works well" is not.*
  - Status: PASS
  - Proof: `spec/ripwire-broker-mcp.md` CA-02: "**Dado** um pedido conceitual e orçamento de 2.500 tokens, **quando** `context_for_task` é chamado, **então** a resposta apresenta símbolos, contexto e testes relevantes sem ultrapassar o limite e informa qualquer corte." `docs/jev-mem-prd.md` §14 #1: "No modo normal, sem `--online` e sem `--memory`, nenhuma chamada Jev ou cliente HTTP é iniciado." The newest plan states checkable acceptance per step, e.g. `spec/plan/ai-native-plan.md:91`: "`wc -c AGENTS.md` ≤ 6.000; todo caminho citado existe e todo comando citado...", now held by `tests/agents_md.rs:211`.
  - Recommendation: —
  - Priority: High

- [x] 15. Open the newest spec: do its criteria go past the happy path — what happens when a step fails, and how the change gets undone?
  *If no specs exist, FAIL with a one-line proof pointing at the specs-and-architecture item. Look for error cases, edge inputs, and a rollback or undo story, not just the success flow.*
  - Status: PASS
  - Proof: Newest spec, `spec/plan/ai-native-plan.md`: step 3's acceptance has two separate rehearsals because "um push recusado nunca chega a virar PR" and names the false-positive risk and its handling (`spec/plan/ai-native-plan.md:151-165`); step 9 is the documented way back (line 339); the adversarial-review table (line 415 on) records failure cases found and fixed (e.g. #5, "`memory forget --all` falha num store de esquema desconhecido"). The newest PRD, `docs/jev-mem-prd.md` §12 "Falhas e comportamento obrigatório" and §15 "Rollback operacional", covers timeouts, clock regression, provider 401/403/429 and undo.
  - Recommendation: —
  - Priority: Low

- [x] 16. Do the specs state non-goals, so an agent knows where to stop?
  *If no specs exist, FAIL with a one-line proof pointing at the specs-and-architecture item. Non-goals written elsewhere (a README's "what this is not" list) are worth naming in the proof, but they do not turn this into a PASS — the question is whether specs carry them.*
  - Status: PASS
  - Proof: `spec/ripwire-broker-mcp.md` §4.2 "Não objetivos do MVP" ("Modificar arquivos do repositório", "Oferecer Streamable HTTP público", ...). The newest plan states its limits: "Este plano não altera código de produto (`src/`), o contrato MCP, dependências Cargo nem os nomes dos jobs obrigatórios" (`spec/plan/ai-native-plan.md:20-22`). `spec/TEMPLATE.md:28-30` makes "Não objetivos" a required part of every new spec.
  - Recommendation: —
  - Priority: Low

- [x] 17. Can recent shipped work be traced back to a spec?
  *If no specs exist, FAIL with a one-line proof pointing at the specs-and-architecture item. Otherwise take the last few substantial commits or PRs and look for a reference to a spec, an issue, or a design doc in the message or description.*
  - Status: PASS
  - Proof: Every commit since 17eddd7 cites its decision: 2e34a98 "build: one command runs every PR check (D-194)", 40fa8bd (D-195), d59a372 (D-196), 99da01b (D-197), a5c4de1 (D-198), 9c8fe89 (D-199), 625853c (D-200), 480945d (D-201); each D-NNN section names its plan step (e.g. `scripts/check.sh:2` "spec/plan/ai-native-plan.md, step 6"). `.github/pull_request_template.md:3` now asks every PR to link its D-NNN or plan step.
  - Recommendation: —
  - Priority: Low

## 3. Verification

- [x] 18. Does this project have an automated test suite, in whatever form this ecosystem uses?
  *Work out this ecosystem's convention before concluding anything is missing — check the manifest, the build file, the CI config, the README. A shell script that diffs output files is a test suite. If you find one, run it and record the result.*
  - Status: PASS
  - Proof: Cargo integration tests in `tests/` (36 nextest binaries, `tests/common/` fakes, recorded fixtures), proptest targets `props`/`props_fs`, and the plugin's TS tests in `integrations/claude-code/tests/`. Ran `scripts/check.sh all` on this checkout (3 min 08 s, exit 0, ends `all PR checks passed`): default `955 tests run: 955 passed, 5 skipped`; `props` at 4096 cases `44 passed`; `props_fs` `10 passed`; online `993 tests run: 993 passed, 8 skipped`; the mod `Ran 47 tests across 6 files`, 0 fail. `ripwire` was on PATH, so the e2e tests ran for real. New since the last audit: `tests/agents_md.rs`, `tests/check_script.rs`, `tests/skills.rs`, `tests/plugin_surface.rs` test the harness itself.
  - Recommendation: —
  - Priority: High

- [x] 19. Can the test command be discovered without guessing — is it written down where an agent reads?
  *If no test suite exists, FAIL with a one-line proof pointing at the test-suite item. Otherwise check the places an agent looks: the agent instruction file, the README, the manifest's scripts or targets.*
  - Status: PASS
  - Proof: `AGENTS.md:47-51` (`scripts/check.sh`, one target, `cargo lint`), `README.md` §"Tests" (`README.md:965-983`, the script first, then `cargo test` and the opt-ins), `handoff.md` §"Como verificar" (`handoff.md:50-54`).
  - Recommendation: —
  - Priority: High

- [x] 20. Do the tests assert real behaviour, rather than asserting that a mock was called?
  *If no test suite exists, FAIL with a one-line proof pointing at the test-suite item. Otherwise open the largest test files and read the assertions: calling real code on real inputs passes; `expect(mock).toHaveBeenCalled()` as the main dish fails.*
  - Status: PASS
  - Proof: Unchanged from the last audit and still true: `tests/broker.rs` `a_quality_regression_blocks_ready` drives the public `Broker` against a scripted upstream with a recorded fixture and asserts the real output (`regression["symbol"] == "classify"`, message contains `"0 -> 42"`); `tests/online_units.rs` writes real files and asserts what the reader excludes. The new harness tests do the same: `tests/check_script.rs` runs the real `scripts/check.sh` against fake tools on PATH; `tests/agents_md.rs` reads the real `AGENTS.md` and runs the commands it shows.
  - Recommendation: —
  - Priority: High

- [x] 21. Is there a linter or static analysis configured for this language, and does it pass on a clean checkout?
  *Configured is not enough — run it. A linter that exits non-zero on an untouched checkout is a FAIL with the error count in the proof, because an agent cannot tell its own damage from the baseline noise.*
  - Status: PASS
  - Proof: Clippy with `-D warnings`, now behind the aliases `cargo lint` / `cargo lint-online` (`.cargo/config.toml:4-5`), run by `scripts/check.sh` (`scripts/check.sh:143,155`). Both ran clean in the `scripts/check.sh all` run: `Finished`, no warning, exit 0.
  - Recommendation: —
  - Priority: High

- [x] 22. Is there a formatter, so an agent's diffs do not churn on style?
  *Look for the config file and the dependency in this ecosystem's form — .prettierrc, rustfmt, gofmt, black, an .editorconfig doing real work. If the language ships one formatting standard with the toolchain, that is a PASS and the proof says so.*
  - Status: PASS
  - Proof: rustfmt, the Rust toolchain standard, pinned in `rust-toolchain.toml` (`components = ["clippy", "rustfmt"]`). `cargo fmt --all --check` is the first step of `scripts/check.sh default` (`scripts/check.sh:142`) and passed in the run.
  - Recommendation: —
  - Priority: Low

- [x] 23. Is there a compile-time or type-level gate, if this language offers one?
  *N/A only when the language genuinely has no such gate. If the language offers one and the repo does not use it — no strict mode, no typecheck script, no compiler step — that is a FAIL. Run the gate if it exists and record the result.*
  - Status: PASS
  - Proof: Rust's compiler on a pinned toolchain (1.98.1, edition 2024); every target compiled in the clippy and test steps of the run. The plugin's TS (`integrations/claude-code/tsconfig.json`, `types/index.d.ts`) is checked through `claude plugin test`, which passed.
  - Recommendation: —
  - Priority: Low

- [x] 24. Can an agent prove its own work before it pushes — one command, task-runner target or commit hook that runs every check this project has?
  *One command, not a list to remember: a `check` or `verify` target, a `precommit` script, a Makefile target that chains them. Separate commands documented side by side are close but FAIL — the question is whether the agent can run the whole gauntlet without knowing its parts.*
  - Status: PASS
  - Proof: `scripts/check.sh` (no argument = `all`) runs every check CI runs on a PR, four targets in CI's order: fmt, `cargo lint`, nextest, `props` at `PROPTEST_CASES=4096`, `props_fs`, the CA-10 gate and the fixture guard (`default`, `scripts/check.sh:141-152`); `cargo lint-online` and the online tests (`online`, :154-158); `plugin validate --strict` for plugin and marketplace plus `plugin test` (`plugin`, :161-170); gitleaks over the whole history (`secrets`, :176-202). `all` refuses to start if a tool is missing, including `ripwire`, so empty e2e passes do not count (:57-75). Ran it: exit 0, `all PR checks passed`. Not in it on purpose, and said so (`scripts/check.sh:10`): `cargo deny` (scheduled, D-108) and the paid live tests; the PR-only "Plugin surface" step needs the PR diff and network (`rust.yml:66-77`). No git hook runs it; it is one command an agent runs by hand.
  - Recommendation: —
  - Priority: High

- [x] 25. Is there continuous integration, and does it run before a change lands rather than after?
  *A deploy pipeline that runs on push to the default branch runs after the change has landed — that is a FAIL with the trigger quoted, not a PASS with a caveat. Look for a pre-merge trigger: pull_request events, merge trains, or this forge's equivalent.*
  - Status: PASS
  - Proof: `.github/workflows/rust.yml:3-7`: `on: pull_request: branches: [ "master" ]` (plus push to master and `workflow_call` from `release.yml`). The comment `rust.yml:25-26` says `default` and `online` are required checks on `master`; branch protection itself is a GitHub setting and is unverified from this checkout.
  - Recommendation: —
  - Priority: High

- [x] 26. Does CI actually run the tests and the checks that exist in this repo?
  *If no pipeline of any kind exists, FAIL with a one-line proof pointing at the CI item. Otherwise list what the pipeline runs and diff it against every check found in the Verification section — name each check that exists in the repo but is missing from the pipeline.*
  - Status: PASS
  - Proof: Each CI job calls `scripts/check.sh` with its own target (`rust.yml:65` default, `:107` online, `:125` plugin, `:157` secrets), so CI runs the same steps as the local command and cannot drift from it; `default` adds the PR-only `scripts/plugin-surface.sh` (`rust.yml:70-77`). `cargo deny` runs in `.github/workflows/supply-chain.yml` weekly and on manifest/lockfile pushes to `master`, deliberately not on PRs (D-108). No check in the repo is missing from CI. Of the four jobs, only `default` and `online` are described as required; `plugin` and `secrets` are not (`rust.yml:109`, `:127-128`).
  - Recommendation: —
  - Priority: High

- [x] 27. Does anything here get an LLM to read a change and go looking for problems — a review skill, a saved review prompt, or an AI reviewer on the pull requests?
  *Look for a committed review skill or slash command, a CI step calling an AI reviewer, or bot config. Human review rules are a Safety item; this one is specifically about machine review.*
  - Status: PASS
  - Proof: `.agents/skills/audit-rust/SKILL.md` turns the saved prompt into an invocable skill ("Você é um revisor de código Rust sênior. Não elogie...", "Todo achado precisa de arquivo, símbolo e evidência"), and the vendored `.agents/skills/rust-review/SKILL.md` reviews a diff; both are linked into `.claude/skills/` and `.grok/skills/`. Review rounds by other models show up in history (f72c264 "address the adversarial review of AGENTS.md", f558a5b "address the Antigravity review", 4a4458e "address Grok's review"). No AI reviewer runs on PRs automatically.
  - Recommendation: —
  - Priority: Low

## 4. Tooling

- [x] 28. Can an agent reach the outside systems this project depends on, with that access committed to the repo rather than set up per laptop?
  *MCP servers are one form: look for a committed `.mcp.json` or this ecosystem's equivalent. A CLI is another and often the better one — `aws`, `gh`, `psql`, `kubectl`, `stripe`, a vendor's own tool — and it counts when the repo names which tools the work needs and how to authenticate, so an agent is not guessing at a tool it cannot see. Judge the axis, not the mechanism: access that exists only in someone's shell history or laptop config is a FAIL, and so is a committed config that turns out to be gitignored, with the distinction in the proof. Project task scripts have their own item; this one is about reaching past the repo's edge. If this project genuinely talks to nothing outside itself, N/A with that as the reason.*
  - Status: PASS
  - Proof: Named in committed files with how to authenticate: `ripwire` (`README.md` §"Installing ripwire"; `ripwire-broker doctor`), the Jev / Cloudflare classifier (`README.md` §Secrets: key only from `RIPWIRE_BROKER_JEV_API_KEY`), GitHub via `gh` (`release.yml`, the `release` skill's read-only `gh release view` / `gh release download`), the `claude` CLI pinned at 2.1.292 and fetched by `npx` (`scripts/check.sh:17,162`), and `gitleaks` (`scripts/check.sh:51`, pinned by version and SHA-256 in `rust.yml:136-137`).
  - Recommendation: —
  - Priority: Low

- [x] 29. Is there a skills, commands or reusable-prompt library in the repo?
  *Look for .claude/skills, .agents/skills, .claude/commands, or a prompts directory. Committed and pinned beats committed; note whether anything ties the copies to a source.*
  - Status: PASS
  - Proof: Committed now: `.agents/skills/` holds three skills born here (`decision-entry`, `release`, `audit-rust`) and four vendored ones (`aine-readiness-analyzer`, `rust-best-practices`, `rust-port`, `rust-review`); `.claude/skills/*` and `.grok/skills/*` are tracked symlinks to them (`git ls-files` lists them; `ls -la .claude/skills` shows `-> ../../.agents/skills/...`). The vendored four are tied to their source by `skills-lock.json` (`alfonsograziano/skills`, `morganlinton/BeautifulRust`, each with a `computedHash`). `tests/skills.rs` treats every unlocked directory as the repo's own and checks each one says where it stops.
  - Recommendation: —
  - Priority: Low

- [x] 30. Do the skills, servers and tools that exist cover the work this team plainly repeats?
  *First identify the repeated work from the README and the commit history. If skills, servers or documented tooling exist but miss it, FAIL naming the gap. If none exist at all and the repo plainly repeats work, FAIL pointing at the outside-systems item and the skills-library item. If the project is too small to repeat anything, N/A.*
  - Status: PASS
  - Proof: Repeated work from the history (499 commits): a D-NNN entry for every decision (128 commit subjects cite one) → `decision-entry` skill; review/audit-then-fix rounds (43 subjects) → `audit-rust` and `rust-review`; the release dance (tag, `release.yml`, pins in `checksums.txt`, PR) → `release` skill, which stops at the tag, the approval and the merge; running the PR gauntlet → `scripts/check.sh`. One repeated job left uncovered: Dependabot PR triage (18 merged Dependabot PRs), whose pin policy lives only in the comments of `.github/dependabot.yml`; merging is the maintainer's anyway (`AGENTS.md:14`).
  - Recommendation: —
  - Priority: Low

- [x] 31. Are there project CLI scripts or task-runner targets for the common jobs?
  *Check the manifest's scripts, the Makefile, the justfile, or this ecosystem's equivalent. The test: does routine work need a raw multi-flag command that someone has to remember?*
  - Status: PASS
  - Proof: `scripts/check.sh [all|default|online|plugin|secrets]` and the Cargo aliases `cargo lint` / `cargo lint-online` (`.cargo/config.toml:4-5`) replace the multi-flag commands the last audit listed (clippy with features and `-D warnings`, `PROPTEST_CASES=4096 ... -E 'binary(props)'`, the inline CA-10 pipeline, now `no_network_stack` in `scripts/check.sh:123-129`). Still raw by design: the paid live run (`RIPWIRE_BROKER_JEV_API_KEY=... cargo test --features online --test online_live -- --ignored`, `README.md:982`), which is on the "Ask before" list.
  - Recommendation: —
  - Priority: High

- [x] 32. Are those scripts named somewhere the agent will actually read them?
  *If no scripts exist, FAIL with a one-line proof pointing at the task-scripts item. Otherwise check the agent instruction file, the README, and whether the manifest itself is self-explanatory.*
  - Status: PASS
  - Proof: `AGENTS.md` §Verify (`AGENTS.md:47-56`), loaded every session; also `README.md` §"Tests" (`README.md:967-975`) and `handoff.md` §"Como verificar" (`handoff.md:50-54`). The aliases carry a comment saying what they are (`.cargo/config.toml:2-3`).
  - Recommendation: —
  - Priority: High

- [x] 33. Can an agent get this project running — is there a reproducible environment or a documented setup path?
  *Look for a pinned runtime (.nvmrc, .tool-versions, rust-toolchain), a lockfile, a container or nix file, and written setup steps. Try the first step if it is cheap and safe. This one bites hardest the moment work happens in a fresh git worktree or a new clone — the normal way to run agents in parallel. A new worktree has no installed dependencies, no `.env`, no build cache, so anything that works today only because of untracked state sitting on someone's machine simply does not run there. The test: would a bare checkout plus the written steps get this project up? Name any prerequisite nothing creates — an env file someone hand-made, a seeded database, a manual login — because each one is a wall a worktree hits on its first command.*
  - Status: PASS
  - Proof: Pinned toolchain (`rust-toolchain.toml` 1.98.1 with clippy/rustfmt), committed `Cargo.lock`, `--locked` everywhere; `README.md` §Requirements and §"Installing ripwire"; no `.env` needed. This audit ran in a fresh git worktree with an empty `target/`: `scripts/check.sh all` built and passed everything in 3 min 08 s. The tools a full run needs are now named with install hints: `AGENTS.md:53-55` and the script itself, which refuses to start without them (run with a bare PATH, it printed `cargo: install Rust with rustup (https://rustup.rs)`, `gitleaks: brew install gitleaks, ...`, `ripwire is not on PATH; the e2e tests pass empty without it`, exit 1). The Claude Code needed by the `plugin` target is fetched by `npx` at the pinned version, so only Node.js is a prerequisite.
  - Recommendation: —
  - Priority: High

- [x] 34. Can an agent see the results of a failed run — do the tools here produce output it can read and act on?
  *Judge from the runs you already did in Verification: does a failure print a path, a line, a name — something actionable — or a wall of noise? If nothing could be run, FAIL saying why.*
  - Status: PASS
  - Proof: From this audit's runs: a missing tool prints which one and how to install it, exit 1; a bad target prints the usage, exit 2; every check is a named `==> <title>` step (`::group::` in CI) and the gates fail with a reason (`::error::a network crate reached the default build (CA-10)`, `::error::gitleaks scanned no commit, so the scan proved nothing`, `scripts/check.sh:126,195`); nextest prints per-test PASS/FAIL and `.config/nextest.toml` ends and names a hung test (D-144). Cosmetic: with both `default` and `online` selected, the missing-tool list prints `cargo` and `cargo-nextest` twice.
  - Recommendation: —
  - Priority: Low

## 5. Safety

- [x] 35. Are credentials kept out of the repo — nothing secret committed, ignore rules in place, an example env file for the shape?
  *Three checks: grep tracked files for key-shaped strings, read the ignore rules for env and key patterns, and look for an example env file. Report each of the three separately.*
  - Status: PASS
  - Proof: (1) `git grep` of tracked files for `sk-…`, `AKIA…`, `ghp_…`, `xox[bp]-`, `BEGIN … PRIVATE KEY` hits only the synthetic stubs `tests/online_units.rs:449` and `tests/props_fs.rs:169-170` (header-only keys written to a temp dir to prove the reader refuses them), plus the previous report quoting them; gitleaks over 394 commits in this audit's run: `no leaks found`. (2) `.gitignore` ignores `.env` and `.envrc` (comment cites D-089). (3) No example env file, by design: configuration is CLI flags plus one variable, documented in `README.md` §Secrets (`RIPWIRE_BROKER_JEV_API_KEY`, never an argument).
  - Recommendation: —
  - Priority: High

- [x] 36. Does anything scan for secrets automatically?
  *Look for gitleaks, trufflehog, detect-secrets or this ecosystem's equivalent, wherever it is wired in — a CI step, a pre-commit hook, or forge-level push protection visible from the repo. CI is where this normally lives, and that is a PASS; a local hook on top is better, because it catches the key before it is pushed rather than after, but its absence is a line in the proof, not a FAIL. Say where the scan runs. "Nothing secret exists today" does not make this N/A — the scan is for the day that changes.*
  - Status: PASS
  - Proof: CI: the `secrets` job of `.github/workflows/rust.yml:129-157` runs on every `pull_request` to `master`, downloads gitleaks 8.30.1 checked against a pinned SHA-256, checks out `fetch-depth: 0`, and runs `scripts/check.sh secrets`, which scans the whole history (`--all --full-history --diff-merges=remerge`), refuses a shallow clone, and fails when gitleaks logs an error or scans no commit (`scripts/check.sh:176-198`); exceptions are per-finding fingerprints in `.gitleaksignore` with reasons, no allowlist (`.gitleaks.toml:1-8`). Ran here: `394 commits scanned`, `no leaks found`. Locally the same scan is part of `scripts/check.sh all`; there is no pre-commit hook (`core.hooksPath` unset, no hooks but samples, no `.pre-commit-config.yaml`). Not verified from the checkout: whether `secrets` is a required check (the workflow says it is not yet, `rust.yml:127-128`) and whether GitHub secret scanning / push protection are on (`handoff.md:33` leaves them to the maintainer).
  - Recommendation: —
  - Priority: High

- [x] 37. Are dependencies pinned, so a build is reproducible?
  *Look for lockfiles in every package of the repo, exact versions for load-bearing dependencies, a pinned runtime, and an install command that respects the lock (npm ci, not npm install).*
  - Status: PASS
  - Proof: `Cargo.lock` committed, every cargo command `--locked` (`scripts/check.sh:139`, `release.yml:90`); `rust-toolchain.toml` 1.98.1; `rust-mcp-sdk = "=2.0.0"` exact (D-002/D-105); actions pinned by full SHA; gitleaks pinned by version and SHA-256 (`rust.yml:136-137`); Claude Code pinned at 2.1.292 in `scripts/check.sh:17`; plugin release binaries pinned by SHA-256 in `integrations/claude-code/scripts/checksums.txt`, now checked against the release's `SHA256SUMS` on PRs (`scripts/plugin-surface.sh:57-80`). The TS plugin has no npm dependencies. Deliberate exception: CI installs the latest ripwire (D-188, `rust.yml:44-46`).
  - Recommendation: —
  - Priority: High

- [x] 38. Is anything watching those dependencies for known vulnerabilities?
  *Look for dependabot or renovate config, an audit step in CI, or this ecosystem's equivalent. Check every lockfile in the repo is covered, not just the root one.*
  - Status: PASS
  - Proof: `.github/dependabot.yml`: weekly `cargo` and `github-actions` (with `rust-mcp-sdk` and `sha2 >= 0.11` ignored for written reasons); `.github/workflows/supply-chain.yml` runs `cargo deny` weekly and on manifest/lockfile pushes. `Cargo.lock` is the only lockfile. Not watched: the gitleaks binary and the Claude Code version, pinned in `rust.yml`/`scripts/check.sh` env vars that Dependabot does not read; both say raising them is a decision.
  - Recommendation: —
  - Priority: Low

- [x] 39. Are the review rules written down — who reads a change, and what they check?
  *Look for CONTRIBUTING, a PR template, or a review checklist in the agent instruction set. On a solo repo the "who" is N/A-shaped but the "what gets checked before it lands" still matters — judge that half.*
  - Status: PASS
  - Proof: `.github/pull_request_template.md` (D-198): "What and why" (link the D-NNN or plan step), a checklist taken from `handoff.md` §"Convenções" and `AGENTS.md` (red test first through `tests/` plus a mutation; D-NNN entry in Portuguese; no network crate in the default build; `scripts/check.sh all` green or say which target was not run; synthetic fixtures; plugin changes asked first and described), and "Verified" ("'Tests pass' alone is not an answer"). The "who": a solo repo; `AGENTS.md:39-43` makes pushing, opening and merging PRs the maintainer's. No CODEOWNERS; whether a review is required on `master` is a GitHub setting, unverified here.
  - Recommendation: —
  - Priority: Low

- [x] 40. Are the operations that need a human named somewhere an agent will read them?
  *Look for a "never without asking" list in the agent instruction file or the README. Docs that hand out production commands with no fence around them count against, and the proof should quote one.*
  - Status: PASS
  - Proof: `AGENTS.md` §"Ask before" (`AGENTS.md:9-35`), in the file loaded every session, lists by effect, with the reason for each: publishing (`v*` tags, releases, `gh pr merge`, `git push` to `master`, approving the `release` environment); any edit under `integrations/claude-code/` or to `.claude-plugin/marketplace.json`; anything that reaches the paid classifier (`--online`, `serve --memory`, `memory drain --online`, `doctor --jev-probe`, the `--ignored` live tests, paid `ripwire-eval` runs); `install … --write` outside a temp dir; `memory forget --all` on real state; the gates (workflow steps, `scripts/check.sh`, `.cargo/config.toml`, rulesets, environments, `deny.toml`, gitleaks files). `AGENTS.md:37-43` repeats the publishing set under Credentials. The skills that come near these stop and hand over (`release` and `decision-entry` §"Where it stops"). `README.md` still shows `memory forget ... --all` and `install ... --write` without a fence, but the fence now sits in the entry point.
  - Recommendation: —
  - Priority: High

- [ ] 41. Does every action that spends money, destroys data or changes production have a human in the way?
  *Start from the damage, not from the tooling: list what in reach of this repo could charge a card, drop or overwrite data, or alter what users are running. Then trace the shortest route an agent could take to each one — a push that auto-deploys, a script carrying live credentials, a migration that runs on merge, an infrastructure apply with no plan-and-approve step. PASS when every route meets a human first, whether that is a review, a manual trigger or a protected environment. FAIL when even one route runs start to finish unattended, and quote that route in the proof so the fix is obvious.*
  - Status: FAIL
  - Proof: Damage in reach: (a) publishing binaries and plugin code to users; (b) spending classifier credit; (c) deleting the local memory store. (b) and (c) need a key or a local command and the "Ask before" list covers them. (a) still has unattended routes for an agent holding the maintainer's login, which D-192 accepts (`spec/changelog.md:8972-8995`: "consegue sem humano no caminho empurrar uma tag `v*`, criar um release, mesclar uma PR, aprovar o environment `release` e mudar o plugin entregue pelo marketplace"). What the checkout now adds: `release.yml:30` puts `environment: release` on `draft`, the first job that writes, with `build` and `publish` chained behind it by `needs` (`release.yml:57,109`); but the file itself says the environment's rules "are set on GitHub by the maintainer; until then GitHub creates it unprotected" (`release.yml:27-29`), and `handoff.md:33` lists creating and configuring it as still pending. Environment protection, a tag ruleset and required reviews on `master` are GitHub settings, unverified from this checkout. Even when set, the agent's `repo`-scoped login can approve the environment, run `gh release create` through the API, or `gh pr merge` a PR that changes `integrations/claude-code/` (`scripts/plugin-surface.sh` only warns, "informative, never blocks", line 7). Fails by decision (D-192).
  - Recommendation: Accepted risk (D-192); reopen it on the triggers D-192 names (a second maintainer, a relevant plugin user base, or unsupervised sessions) by running agents without the maintainer's `repo`-scoped login and then configuring the `release` environment with a required reviewer, a `v*` tag ruleset and a required review on `master`.
  - Priority: High

- [ ] 42. If a prompt injection landed tonight, how far would it reach — are the credentials an agent can get to here scoped to the job, with nothing production-grade in reach?
  *Inventory what an agent in this repo can reach: env files, cloud CLI profiles, tokens named in docs or config, deploy commands that work from a laptop. Scoped-or-absent passes; production-grade reach fails with the item named.*
  - Status: FAIL
  - Proof: `AGENTS.md:37-43`: "The session runs as the maintainer, with their GitHub login (keyring, `repo` scope) in reach (D-192: accepted risk, no isolation)". D-192 (`spec/changelog.md:8972-8995`) rules out a restricted token, a machine account and session isolation, and states the consequence: this item stays failed. `repo` scope is production-grade reach here: tags trigger `release.yml`, releases are what plugin users download, and the marketplace serves `master`. The only remaining fence is the agent's obedience to `AGENTS.md` and the host's permission modes. This audit did not query the keyring or `gh auth status`; the finding rests on the committed decision.
  - Recommendation: Accepted risk (D-192); when a reopen trigger in D-192 fires, run agent sessions under a separate OS user or machine account with no write credential to `aquental/ripwire-broker`, the maintainer pushing branches.
  - Priority: High

- [x] 43. Can a change reach production a slice at a time — a feature flag that defaults to off, a canary, a staged rollout — rather than everyone at once?
  *Look for a flag system and check the default, or canary and staged-rollout config in the deploy pipeline. Flags that need a rebuild to flip are worth naming in the proof — they gate exposure but they are not a kill switch.*
  - Status: PASS
  - Proof: Unchanged: new behaviour ships off by default and is switched at runtime, no rebuild: `--online` (and the `online` Cargo feature, off in the default build, CA-10, gated in `scripts/check.sh:123-129`), `--memory` (experimental), `--incremental`; the plugin exposes them as `user_config` toggles (`integrations/claude-code/.mcp.json`). Binaries reach plugin users only when a later PR pins them in `checksums.txt`, a natural stage. No canary, which suits a locally installed tool.
  - Recommendation: —
  - Priority: High

- [x] 44. Once a change is live, can anyone see what it is doing — logs, metrics, traces, alerts that fire on their own, and can an agent read them too?
  *Look for logging setup, an error tracker, analytics, alerting config — and then ask the second half: could an agent reach any of it (a CLI, an MCP server, an API named in the docs), or does observability stop at a dashboard behind a login?*
  - Status: PASS
  - Proof: Unchanged: no hosted service and no telemetry, by design (local, offline, CA-10). What exists is agent-readable: the `ripwire-broker://status` MCP resource, `hook-log --session`, `hook-stats --json`, `doctor --json`, `memory status --json`, `--memory-debug-log` (`debug.log`, for `tail -F`), `jev.log`; CI results via `gh run list`.
  - Recommendation: —
  - Priority: Low

- [x] 45. Is there a way back — can a bad change be undone without a rebuild and a redeploy, including the ones that touched a database or a queue?
  *Look for a documented rollback path, a revert-and-redeploy story, down-migrations, or a flag that can turn the change off at runtime. Deployment docs that only say how to go forward are worth quoting.*
  - Status: PASS
  - Proof: `integrations/claude-code/README.md` §"Rolling back a version" (lines 167-228; D-200, `spec/changelog.md:9527`, records an end-to-end rehearsal with Claude Code 2.1.292 that published nothing): the maintainer puts the previous tag and its `SHA256SUMS` lines back in `scripts/checksums.txt` and the matching `version` in `plugin.json`, marks the bad release `--prerelease`, merges; no rebuild, the old assets stay. The user side: `claude plugin update`, clear the `binary` option, `sh install-binary.sh`, restart. It names what does not go back on its own (a set `binary` option, an old `ripwire-broker` on PATH) and how to see what runs. The data side: an older binary refuses a newer memory store (`store unavailable: unknown_schema`), `memory forget --all` does not get out of it, and the way out is rolling forward or moving `<state-dir>/memory/<workspace id>/` aside with the processes stopped (lines 215-228). The `release` skill points at the section. Its runtime switch is the flags off (`docs/jev-mem-prd.md:474`, "Rollback operacional").
  - Recommendation: —
  - Priority: Low

- [x] 46. Does a past failure leave a check behind — is there a test or a rule in here that exists because something broke once?
  *Look for tests, lint rules or comments that name the incident they guard against — "regression", "this broke when", a linked issue. On a very young repo with no failures yet, N/A with the age as the reason.*
  - Status: PASS
  - Proof: `tests/eval.rs` cites `#122`; a5854b0 rewrote a flaky test (#120); `.config/nextest.toml` exists because a hung test left no log (D-144); `scripts/check.sh:191` "gitleaks 8.30 exits 0 when git writes anything to stderr, having scanned nothing" and the `must_not_match` guard against grep's exit 2 (`scripts/check.sh:102-119`) came out of the D-196 review rounds; `handoff.md` §"Armadilhas já pisadas" adds the worktree `CARGO_MANIFEST_DIR` trap found in D-200.
  - Recommendation: —
  - Priority: Low

---

Learn more: **https://ainativesoftware.engineering/**
