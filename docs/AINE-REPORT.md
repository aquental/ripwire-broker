# AI-Native Readiness Report

**Repo:** ripwire-broker (`/Users/aquental/projects/ai/CECI/ripwire-broker`, branch `master` @ d9aa86e)
**Date:** 2026-11-14
**Stack:** Rust (edition 2024, toolchain 1.98.1 pinned) on Cargo, Tokio, `rust-mcp-sdk =2.0.0`; tests via `cargo test` / `cargo nextest` with proptest; an optional `online` feature (reqwest + rustls). A Claude Code plugin/mod in TypeScript under `integrations/claude-code/` (tested by `claude plugin test`), a Codex integration under `integrations/codex/`. CI on GitHub Actions (`rust.yml`, `release.yml`, `supply-chain.yml`), Dependabot, cargo-deny. Specs and decision log in Portuguese under `spec/` and `docs/`.

**Result:** 26 passed, 20 failed, 0 not applicable

---

**How to read this.** Every item is a numbered question about this repo, answered from evidence in this repo. `Proof` is what was actually found — a path, a file, a command and its output. An item with no proof is not an answer. The italic line under each question says how to check it; it stays in the report so the next run checks the same way.

**FAIL vs N/A.** `FAIL` means the practice would help here and it is missing — including when a whole artifact is missing, so several questions about it fail together with a one-line proof pointing at the item that found it missing. `N/A` means the question could never apply to a project of this kind, and the proof says why. "It does not exist" is a FAIL, never an N/A.

`Priority` is fixed by the template, not by the person filling it in. **High** means an agent working here will hit this on day one, or the risk is real. **Low** means it makes things better but nothing breaks without it.

---

## 1. Context

- [ ] 1. Does an agent instruction file exist (AGENTS.md, CLAUDE.md, .cursorrules, .github/copilot-instructions.md, or this ecosystem's equivalent), and what does it point at?
  *Look for a file whose purpose is to be loaded into a coding agent's context at the start of every session. That file is the **entry point**. Then follow its pointers: instructions are commonly spread across several files, with the entry point routing to them — "commands are in docs/development.md", "conventions in .claude/rules/". The entry point plus every file it points at is the **instruction set**, and every other agent-instruction question is answered against that whole set, so list it here in the proof. Follow pointers as deep as they go, but a pointer only counts when it names a target and says what is in it: "test commands are in CONTRIBUTING.md" counts, a bare "see the docs" does not, and a file nothing points at is outside the set however good it is. A README is not an entry point, however good — it gets its credit at the README item and the specs-and-architecture item — but a README the entry point explicitly routes to is inside the set. Vendored skill libraries are not instructions either.*
  - Status: FAIL
  - Proof: No AGENTS.md, CLAUDE.md, .cursorrules or .github/copilot-instructions.md at the root or anywhere in `git ls-files`. `.claude/` is untracked (`git status`: `?? .claude/`) and holds only `skills/aine-readiness-analyzer`. The one tracked AGENTS.md, `integrations/codex/AGENTS.md` (2,242 B), is product content shipped to Codex users ("This repository has the `ripwire-broker` MCP server configured..."), not instructions for working on this repo. The real onboarding doc is `handoff.md` (38 KB, sections "Como verificar", "Onde está o quê", "Armadilhas já pisadas", "Convenções"), but no entry point routes to it, so the instruction set is empty.
  - Recommendation: Add a root `AGENTS.md` (plus a `CLAUDE.md` containing `@AGENTS.md`) that routes to `handoff.md` §"Como verificar", §"Onde está o quê", §"Armadilhas já pisadas" and §"Convenções", and to `spec/changelog.md` for decisions.
  - Priority: High

- [ ] 2. Do the agent instructions name the commands to build, test and check this project?
  *Answer this against the instruction set mapped at the entry-point item and nothing outside it. If no entry point exists, FAIL with a one-line proof pointing at that item. The commands may sit in a file the entry point routes to rather than in the entry point itself — that is progressive disclosure working as intended, and it passes. What fails is a command an agent would have to guess its way to: if the commands live only in the README or the manifest and nothing in the set points at them, that is a FAIL, because the question measures what an agent can reach without being told where to look. Name the file each command was found in.*
  - Status: FAIL
  - Proof: No agent instruction file exists (see the agent-instruction-file item). The commands exist in `README.md` §Tests and `handoff.md` §"Como verificar", but nothing an agent loads routes to them.
  - Recommendation: Blocked by the missing agent instruction file.
  - Priority: High

- [ ] 3. Does everything the agent instructions name still exist — the commands, the paths, the libraries, and the files they point at?
  *If no entry point exists, FAIL with a one-line proof pointing at the entry-point item. Verify, do not trust: check every named command against the manifest or build file, spot-check the paths, run the cheap read-only ones. Then resolve every pointer in every file of the set — a link to a moved or deleted file is the most common rot in a multi-file instruction set, and it fails silently: the agent reads the entry point, follows nothing, and carries on without the rules.*
  - Status: FAIL
  - Proof: No agent instruction file exists (see the agent-instruction-file item).
  - Recommendation: Blocked by the missing agent instruction file.
  - Priority: High

- [ ] 4. Are the agent instructions specific to this repo, rather than advice that would read the same in any codebase?
  *If no entry point exists, FAIL with a one-line proof pointing at the entry-point item. The test: could this be pasted into another project unchanged? "Write clean code" and "add tests for new features" would fit anywhere and count for nothing. Judge the whole set, but weigh the files differently: an entry point that is mostly a routing table is fine, even good, when what it routes to is specific — while generic filler in the entry point costs more than generic filler three hops down, because it is loaded into every session whether it is needed or not. Say which files carried the specifics.*
  - Status: FAIL
  - Proof: No agent instruction file exists (see the agent-instruction-file item).
  - Recommendation: Blocked by the missing agent instruction file.
  - Priority: High

- [ ] 5. Do the agent instructions explain where things live and how this project is laid out?
  *If no entry point exists, FAIL with a one-line proof pointing at the entry-point item. The map may live in a routed-to file. Judge coverage against the real tree, not against what the files mention: if the set maps one package well but is silent about sibling packages or directories an agent would land in, that is a FAIL with the omission named. In a multi-file set, check the routing too — a layout document nothing points at is a document the agent never opens.*
  - Status: FAIL
  - Proof: No agent instruction file exists (see the agent-instruction-file item). A layout map exists in `handoff.md` §"Onde está o quê", but nothing points at it.
  - Recommendation: Blocked by the missing agent instruction file.
  - Priority: High

- [ ] 6. Do the agent instructions state the rules that are not obvious from the code — the things never to do here?
  *If no entry point exists, FAIL with a one-line proof pointing at the entry-point item. These are the tribal-knowledge traps: the flag that must be exactly this string, the import that breaks the build, the directory that is generated and must not be edited. Rules in a routed-to file count. Two extra checks in a multi-file set: that the entry point signposts the rules clearly enough for an agent to open them *before* it needs them, since a trap found afterwards has already been sprung; and that the files do not contradict each other, because nothing tells the agent which one wins.*
  - Status: FAIL
  - Proof: No agent instruction file exists (see the agent-instruction-file item). The traps are written down in `handoff.md` §"Armadilhas já pisadas" and §"Convenções" (e.g. "Toda decisão vira uma entrada `D-NNN` em `spec/changelog.md`", "Nada de dependência nova sem motivo: o grafo do build padrão é uma promessa (CA-10)"), and in workflow comments ("Renaming one silently removes the gate", `.github/workflows/rust.yml`), but no entry point signposts them.
  - Recommendation: Blocked by the missing agent instruction file.
  - Priority: Low

- [ ] 7. Has the agent instruction file been updated recently enough to still be true, given how active the repo is?
  *If no agent instruction file exists, FAIL with a one-line proof pointing at the entry-point item. Compare the last commit touching the file against the repo's tempo, then spot-check two or three of its claims against the code — a recently touched file can still lie.*
  - Status: FAIL
  - Proof: No agent instruction file exists (see the agent-instruction-file item).
  - Recommendation: Blocked by the missing agent instruction file.
  - Priority: Low

- [ ] 8. Is the agent instruction file small enough to load into every session without crowding out the actual task?
  *If no agent instruction file exists, FAIL with a one-line proof pointing at the entry-point item. Measure it: `wc -c`, and bytes divided by four is a fair token estimate. Under ~5k tokens is comfortable; past ~10k it is eating the context window.*
  - Status: FAIL
  - Proof: No agent instruction file exists (see the agent-instruction-file item).
  - Recommendation: Blocked by the missing agent instruction file.
  - Priority: Low

- [x] 9. Can a fresh session pick up a half-finished task — is there a file or convention where progress, decisions and what is left get written down?
  *This is not about the agent instruction file. Any durable place in-flight state gets written counts: a TODO.md, a plans or notes directory, task files, linked issues, a specs folder whose entries carry progress and open questions, an agent memory file or directory, a scratchpad or working-notes convention. The test is whether a fresh session could read it and know what was decided and what is left — not what the place is called. Git history alone does not count: it records what happened, not what was decided or what remains.*
  - Status: PASS
  - Proof: `handoff.md` gives the state "até o D-169", a phase table, "Em andamento" sections with numbered next steps ("O que fazer, em ordem") and "Pendências conhecidas"; `spec/plan/roteiro-testes-pendentes.md` has the step-by-step for the pending tests; `spec/changelog.md` records every decision (D-001 to D-188). Caveat: `handoff.md` contradicts itself on freshness. Its header says "até o D-169", while its T5.3 section already covers D-185 to D-187; only D-188 (2026-10-22) is missing.
  - Recommendation: —
  - Priority: Low

- [x] 10. Is there a README that says what this project is and how to run it?
  *This is where a good README earns its credit. It needs three things: what the project is, how to run it, and how to check a change. Judge what is on the page, not the file's existence.*
  - Status: PASS
  - Proof: `README.md` (74 KB): the first paragraph says what it is ("A local MCP server that turns Ripwire's wide surface (33 verbs) into three task-moment tools..."); §Requirements, §"Installing ripwire" and §"Build and run" (`cargo build --release`, `./target/release/ripwire-broker --workspace ...`) cover running it; §Tests (`cargo test`, `cargo test --features online`, the ignored opt-ins) covers checking a change.
  - Recommendation: —
  - Priority: High

## 2. Specs

- [x] 11. Is the thinking behind this system written down somewhere durable — a specs, RFC, proposals, design-doc or ADR directory, or architecture notes that record not just what but why?
  *Look for the place and for the convention: `specs/`, `docs/adr/`, `rfcs/`, `proposals/`, `.specify/`, a docs or design folder, architecture notes anywhere in the tree, or this ecosystem's equivalent. Judge substance, not location: "we use X because Y" is a decision, a list of technologies is not, an essay about specs is not a spec, and a docs folder of usage guides with no reasoning is a FAIL whose proof says what was in there instead. Other items are answered against whatever this item finds, so name it precisely — and where forward-looking specs and after-the-fact architecture records live in different places, name both, since a decision log cannot answer a question about acceptance criteria.*
  - Status: PASS
  - Proof: Forward-looking: `spec/ripwire-broker-mcp.md` (PRD, 184 KB: problem, principles, §4.2 non-goals, §7 architecture, CA-01..CA-10), `docs/jev-mem-prd.md` (memory PRD v0.5), plans in `spec/plan/` and `spec/plan/xtd/`. After the fact: `spec/changelog.md`, D-001 to D-188, each with an index row and a section of reasoning (e.g. D-107: "o `master` não tinha proteção de branch nenhuma ... A porta que o D-102 descreve como frágil não era frágil: não existia"). Plus `spec/diagrams/ripwire-broker.architecture.html`.
  - Recommendation: —
  - Priority: High

- [x] 12. Does the specs directory hold recent entries, or is it an archive nobody has touched?
  *If no specs directory exists, FAIL with a one-line proof pointing at the specs-and-architecture item. Otherwise compare the newest entry's date against the repo's recent activity.*
  - Status: PASS
  - Proof: `git log -1 -- spec/` = 2026-10-23, the same as the repo's last commit (2026-10-23). Newest spec documents: `docs/jev-mem-prd.md`, `spec/plan/jev-mem-plan.md`, `spec/plan/roteiro-testes-pendentes.md`, all committed 2026-10-11; the changelog was extended through D-188 on 2026-10-22.
  - Recommendation: —
  - Priority: Low

- [ ] 13. Is there a spec template, or an SDD framework, so every spec comes out the same shape?
  *Scaffolding can exist even where no specs directory does — look for a TEMPLATE.md, a `.specify/` directory, or framework config. If neither a directory nor any scaffolding exists, FAIL.*
  - Status: FAIL
  - Proof: `find` for `*template*` and `.specify` outside `target/` returns nothing (only this skill's own template). PRDs and plans share a house style (Dado/quando/então criteria, numbered sections), and the changelog has a fixed index-row-plus-section format described in `handoff.md` §Convenções, but no spec or plan template exists.
  - Recommendation: Add `spec/TEMPLATE.md` capturing the shape the PRDs already use: context, objectives and non-goals, Dado/quando/então acceptance criteria, failure behaviour, rollback, and the D-NNN entries it creates.
  - Priority: Low

- [x] 14. Do the specs state acceptance criteria a machine could check?
  *If no specs exist, FAIL with a one-line proof pointing at the specs-and-architecture item. Otherwise open the two newest specs and quote a criterion: "the endpoint returns 403 for expired tokens" is checkable; "the feature works well" is not.*
  - Status: PASS
  - Proof: `spec/ripwire-broker-mcp.md` CA-02: "**Dado** um pedido conceitual e orçamento de 2.500 tokens, **quando** `context_for_task` é chamado, **então** a resposta apresenta símbolos, contexto e testes relevantes sem ultrapassar o limite e informa qualquer corte." `docs/jev-mem-prd.md` §14 "Critérios verificáveis" #1: "No modo normal, sem `--online` e sem `--memory`, nenhuma chamada Jev ou cliente HTTP é iniciado." CA-10 is also enforced as a CI step (`cargo tree -e normal | grep reqwest|rustls|hyper`).
  - Recommendation: —
  - Priority: High

- [x] 15. Open the newest spec: do its criteria go past the happy path — what happens when a step fails, and how the change gets undone?
  *If no specs exist, FAIL with a one-line proof pointing at the specs-and-architecture item. Look for error cases, edge inputs, and a rollback or undo story, not just the success flow.*
  - Status: PASS
  - Proof: `docs/jev-mem-prd.md` §12 "Falhas e comportamento obrigatório" covers timeouts mid-pair, worktree/hash changes, clock regression and rebase, death before consolidation, and provider 401/403/429. §14 criterion #7: "Fault injection em spool/rename/commit/write 19/retry/forget demonstra idempotência e não ressurreição"; #11: "Retenção e forget removem pais/descendentes/cache/jobs". The undo path is `memory forget` / `memory resume`.
  - Recommendation: —
  - Priority: Low

- [x] 16. Do the specs state non-goals, so an agent knows where to stop?
  *If no specs exist, FAIL with a one-line proof pointing at the specs-and-architecture item. Non-goals written elsewhere (a README's "what this is not" list) are worth naming in the proof, but they do not turn this into a PASS — the question is whether specs carry them.*
  - Status: PASS
  - Proof: `spec/ripwire-broker-mcp.md` §4.2 "Não objetivos do MVP": "Modificar arquivos do repositório", "Gerar resumos por serviço LLM remoto", "Oferecer Streamable HTTP público", "Suportar múltiplos workspaces em uma mesma instância", and six more.
  - Recommendation: —
  - Priority: Low

- [x] 17. Can recent shipped work be traced back to a spec?
  *If no specs exist, FAIL with a one-line proof pointing at the specs-and-architecture item. Otherwise take the last few substantial commits or PRs and look for a reference to a spec, an issue, or a design doc in the message or description.*
  - Status: PASS
  - Proof: Last commits: 7e2216c "ci: install the latest ripwire release ... (D-188)"; a5854b0 "test: make the two timing-sensitive tests deterministic (#120, #122)"; PR #119's body names the RustSec advisories; plans in `spec/plan/` cite D-numbers and task IDs (e.g. `release.yml`: "spec/plan/mod-plan.md, T2.1").
  - Recommendation: —
  - Priority: Low

## 3. Verification

- [x] 18. Does this project have an automated test suite, in whatever form this ecosystem uses?
  *Work out this ecosystem's convention before concluding anything is missing — check the manifest, the build file, the CI config, the README. A shell script that diffs output files is a test suite. If you find one, run it and record the result.*
  - Status: PASS
  - Proof: Cargo integration tests in `tests/` (33 targets, `tests/common/` fakes, fixtures recorded from ripwire 0.6.4 and real Claude Code/Codex hook payloads), proptest targets `props`/`props_fs`, and the plugin's TS tests in `integrations/claude-code/tests/` (`claude plugin test`). Ran `cargo test --locked`: the first run stopped with `error: test failed, to rerun pass --test plugin`; `cargo test --locked --test plugin` then passed 20/20, and `cargo test --locked --no-fail-fast` passed 886, failed 0, ignored 5. So one test in `tests/plugin.rs` is intermittent (it was not captured on the failing run). The local run also included the untracked `tests/routing_budget.rs`.
  - Recommendation: —
  - Priority: High

- [x] 19. Can the test command be discovered without guessing — is it written down where an agent reads?
  *If no test suite exists, FAIL with a one-line proof pointing at the test-suite item. Otherwise check the places an agent looks: the agent instruction file, the README, the manifest's scripts or targets.*
  - Status: PASS
  - Proof: `README.md` §Tests lists `cargo test`, `cargo test --features online`, and the `--ignored` opt-ins with their env vars; `handoff.md` §"Como verificar" lists the same plus clippy, fmt and the three `claude plugin` lines. Both are standard places to look, though no agent file routes to them (see the agent-instruction-file item).
  - Recommendation: —
  - Priority: High

- [x] 20. Do the tests assert real behaviour, rather than asserting that a mock was called?
  *If no test suite exists, FAIL with a one-line proof pointing at the test-suite item. Otherwise open the largest test files and read the assertions: calling real code on real inputs passes; `expect(mock).toHaveBeenCalled()` as the main dish fails.*
  - Status: PASS
  - Proof: `tests/broker.rs` `a_quality_regression_blocks_ready` drives the public `Broker` seam against a scripted upstream that answers with a recorded fixture (`quality_delta_regression`) and asserts on the real output: `regression["symbol"] == "classify"`, `regression["path"] == "src/auth.py"`, message contains `"0 -> 42"`. `tests/online_units.rs` builds real files on disk (`.env`, `id_rsa`, `config/deploy.py` with a key header) and asserts what the reader excludes. `handoff.md` §Convenções: "TDD por costura pública ... nada de testar função privada" and "Mutação para provar que um teste morde".
  - Recommendation: —
  - Priority: High

- [x] 21. Is there a linter or static analysis configured for this language, and does it pass on a clean checkout?
  *Configured is not enough — run it. A linter that exits non-zero on an untouched checkout is a FAIL with the error count in the proof, because an agent cannot tell its own damage from the baseline noise.*
  - Status: PASS
  - Proof: Clippy with `-D warnings` in CI (`rust.yml`), plus `forbid(unsafe_code)` / `deny(print_stdout)` (D-107). Ran `cargo clippy --all-targets --locked -- -D warnings` on this checkout: `Finished`, exit 0.
  - Recommendation: —
  - Priority: High

- [x] 22. Is there a formatter, so an agent's diffs do not churn on style?
  *Look for the config file and the dependency in this ecosystem's form — .prettierrc, rustfmt, gofmt, black, an .editorconfig doing real work. If the language ships one formatting standard with the toolchain, that is a PASS and the proof says so.*
  - Status: PASS
  - Proof: rustfmt is the Rust toolchain standard; `rust-toolchain.toml` pins `components = ["clippy", "rustfmt"]`. Ran `cargo fmt --all --check`: exit 0.
  - Recommendation: —
  - Priority: Low

- [x] 23. Is there a compile-time or type-level gate, if this language offers one?
  *N/A only when the language genuinely has no such gate. If the language offers one and the repo does not use it — no strict mode, no typecheck script, no compiler step — that is a FAIL. Run the gate if it exists and record the result.*
  - Status: PASS
  - Proof: Rust's compiler is the type gate, on a pinned toolchain (`channel = "1.98.1"`, edition 2024). The clippy and test runs above compiled every target without errors. The plugin's TS has a `tsconfig.json` and `types/index.d.ts`; it is typechecked through `claude plugin test` in the CI `plugin` job.
  - Recommendation: —
  - Priority: Low

- [ ] 24. Can an agent prove its own work before it pushes — one command, task-runner target or commit hook that runs every check this project has?
  *One command, not a list to remember: a `check` or `verify` target, a `precommit` script, a Makefile target that chains them. Separate commands documented side by side are close but FAIL — the question is whether the agent can run the whole gauntlet without knowing its parts.*
  - Status: FAIL
  - Proof: No Makefile, justfile, `xtask`, `.cargo/config.toml` alias or git hook (`.git/hooks` has only samples; no `.pre-commit-config.yaml`). The checks sit as separate lines in `handoff.md` §"Como verificar" (seven commands) and as separate CI steps, two of them inline shell (the CA-10 `cargo tree | grep` and the fixture credential grep), which exist only in `rust.yml`.
  - Recommendation: Add a `scripts/check.sh` (or `cargo xtask check`) that runs fmt, clippy for both feature sets, nextest for both, the `props` run with `PROPTEST_CASES=4096`, the CA-10 grep and the fixture guard, and have `rust.yml` call it so local and CI cannot drift.
  - Priority: High

- [x] 25. Is there continuous integration, and does it run before a change lands rather than after?
  *A deploy pipeline that runs on push to the default branch runs after the change has landed — that is a FAIL with the trigger quoted, not a PASS with a caveat. Look for a pre-merge trigger: pull_request events, merge trains, or this forge's equivalent.*
  - Status: PASS
  - Proof: `.github/workflows/rust.yml`: `on: pull_request: branches: [ "master" ]`. Branch protection (`gh api .../branches/master/protection`) requires the `default` and `online` checks, with `enforce_admins: true`.
  - Recommendation: —
  - Priority: High

- [x] 26. Does CI actually run the tests and the checks that exist in this repo?
  *If no pipeline of any kind exists, FAIL with a one-line proof pointing at the CI item. Otherwise list what the pipeline runs and diff it against every check found in the Verification section — name each check that exists in the repo but is missing from the pipeline.*
  - Status: PASS
  - Proof: `rust.yml` runs `cargo fmt --all --check`, clippy `-D warnings` (default and `--features online`), `cargo nextest run --locked` (both feature sets), `props` with `PROPTEST_CASES=4096`, `props_fs`, the CA-10 network-crate gate, the synthetic-fixture guard, and in the `plugin` job `claude plugin validate --strict` (plugin and marketplace) plus `claude plugin test`. Real-binary tests run against the latest ripwire release (D-188). `cargo deny` runs in `supply-chain.yml` on a weekly schedule and on pushes that touch the lockfile, deliberately not on PRs (D-108). No check in the repo is missing from CI.
  - Recommendation: —
  - Priority: High

- [x] 27. Does anything here get an LLM to read a change and go looking for problems — a review skill, a saved review prompt, or an AI reviewer on the pull requests?
  *Look for a committed review skill or slash command, a CI step calling an AI reviewer, or bot config. Human review rules are a Safety item; this one is specifically about machine review.*
  - Status: PASS
  - Proof: `spec/prompt/auditoria-rust.md` is a saved review prompt ("Você é um revisor de código Rust sênior. Não elogie... Todo achado precisa de arquivo, símbolo e evidência"), and its results show up in history (D-143/D-144 audit, commit 00784e3 "test: address the audit of the flaky-test fix"). No AI reviewer runs on PRs; this is on demand.
  - Recommendation: —
  - Priority: Low

## 4. Tooling

- [x] 28. Can an agent reach the outside systems this project depends on, with that access committed to the repo rather than set up per laptop?
  *MCP servers are one form: look for a committed `.mcp.json` or this ecosystem's equivalent. A CLI is another and often the better one — `aws`, `gh`, `psql`, `kubectl`, `stripe`, a vendor's own tool — and it counts when the repo names which tools the work needs and how to authenticate, so an agent is not guessing at a tool it cannot see. Judge the axis, not the mechanism: access that exists only in someone's shell history or laptop config is a FAIL, and so is a committed config that turns out to be gitignored, with the distinction in the proof. Project task scripts have their own item; this one is about reaching past the repo's edge. If this project genuinely talks to nothing outside itself, N/A with that as the reason.*
  - Status: PASS
  - Proof: The project talks to `ripwire` (CLI; `README.md` §"Installing ripwire" gives the install script and PATH line, and `ripwire-broker doctor` checks it), the Jev / Cloudflare Workers AI classifier (`README.md` §Secrets: key only from `RIPWIRE_BROKER_JEV_API_KEY`; `--jev-account-id` for Cloudflare), GitHub via `gh` (`release.yml`; `handoff.md` publishing steps), and the `claude` CLI pinned at `@anthropic-ai/claude-code@2.1.292` (`rust.yml`). All of these are named in committed files, with how to authenticate.
  - Recommendation: —
  - Priority: Low

- [ ] 29. Is there a skills, commands or reusable-prompt library in the repo?
  *Look for .claude/skills, .agents/skills, .claude/commands, or a prompts directory. Committed and pinned beats committed; note whether anything ties the copies to a source.*
  - Status: FAIL
  - Proof: Committed: only `integrations/claude-code/skills/ripwire-broker/SKILL.md`, a skill the product ships to its users. `.claude/skills/`, `.agents/skills/`, `.grok/skills/` (each holding only `aine-readiness-analyzer`) and `skills-lock.json` (source `alfonsograziano/skills`, pinned by `computedHash`) are all untracked per `git status`, so a fresh clone has none of them. `spec/prompt/` holds three saved prompts (`auditoria-rust.md`, `ci-cd.md`, `xtd/jev-provider-cloudflare.md`) but not as invocable skills or commands.
  - Recommendation: Commit `.claude/skills/` together with `skills-lock.json` (or delete them), and turn `spec/prompt/auditoria-rust.md` into a `.claude/skills/audit-rust/SKILL.md` so it can be invoked by name.
  - Priority: Low

- [ ] 30. Do the skills, servers and tools that exist cover the work this team plainly repeats?
  *First identify the repeated work from the README and the commit history. If skills, servers or documented tooling exist but miss it, FAIL naming the gap. If none exist at all and the repo plainly repeats work, FAIL pointing at the outside-systems item and the skills-library item. If the project is too small to repeat anything, N/A.*
  - Status: FAIL
  - Proof: Repeated work, from the history and `handoff.md`: writing a D-NNN entry (index row plus section, with the split ordering rule in §Convenções) for every decision; the release dance (tag → `release.yml` → paste SHA-256 lines into `integrations/claude-code/scripts/checksums.txt` in a PR); Dependabot PR triage against the pin policy in `dependabot.yml`; audit-then-fix rounds. No skill, command or script covers any of them: the only repo-local skill is the readiness analyzer, and it is untracked (see the skills-library item).
  - Recommendation: Add committed skills for the two most frequent: `decision-entry` (append a D-NNN row and section to `spec/changelog.md` following the §Convenções ordering rule) and `release` (tag, wait for `release.yml`, write `checksums.txt`, open the PR).
  - Priority: Low

- [ ] 31. Are there project CLI scripts or task-runner targets for the common jobs?
  *Check the manifest's scripts, the Makefile, the justfile, or this ecosystem's equivalent. The test: does routine work need a raw multi-flag command that someone has to remember?*
  - Status: FAIL
  - Proof: Cargo is the only runner: no `[alias]` in a `.cargo/config.toml`, no Makefile/justfile/xtask, no `scripts/` at the root (`integrations/claude-code/scripts/` is the plugin's runtime launcher and installer). Routine work needs remembered multi-flag commands: `cargo clippy --all-targets --locked --features online -- -D warnings`, `PROPTEST_CASES=4096 cargo nextest run --locked -E 'binary(props)'`, `RIPWIRE_BROKER_JEV_API_KEY=... cargo test --features online --test online_live -- --ignored`, plus the CA-10 pipeline, which exists only inline in `rust.yml`.
  - Recommendation: Add `.cargo/config.toml` aliases (e.g. `lint`, `lint-online`, `props`, `live`) or a justfile with the same targets, and the `check` target from the one-command-gate item.
  - Priority: High

- [ ] 32. Are those scripts named somewhere the agent will actually read them?
  *If no scripts exist, FAIL with a one-line proof pointing at the task-scripts item. Otherwise check the agent instruction file, the README, and whether the manifest itself is self-explanatory.*
  - Status: FAIL
  - Proof: No task scripts exist (see the task-scripts item).
  - Recommendation: Blocked by the missing task scripts.
  - Priority: High

- [x] 33. Can an agent get this project running — is there a reproducible environment or a documented setup path?
  *Look for a pinned runtime (.nvmrc, .tool-versions, rust-toolchain), a lockfile, a container or nix file, and written setup steps. Try the first step if it is cheap and safe. This one bites hardest the moment work happens in a fresh git worktree or a new clone — the normal way to run agents in parallel. A new worktree has no installed dependencies, no `.env`, no build cache, so anything that works today only because of untracked state sitting on someone's machine simply does not run there. The test: would a bare checkout plus the written steps get this project up? Name any prerequisite nothing creates — an env file someone hand-made, a seeded database, a manual login — because each one is a wall a worktree hits on its first command.*
  - Status: PASS
  - Proof: Pinned toolchain (`rust-toolchain.toml` 1.98.1 with clippy/rustfmt), committed `Cargo.lock`, `--locked` in CI; `README.md` §Requirements and §"Installing ripwire" give the one external prerequisite. Ran `ripwire --version` → `ripwire 0.6.4`. No `.env` needed (README: "There is no `.env` file and none is needed"). A fresh worktree needs only `ripwire` on PATH. Without it, the e2e tests return early and pass (`require_ripwire!`, README: "The e2e and upstream tests are skipped when `ripwire` is not on `PATH`"), so a bare worktree goes green without testing them. The plugin checks need the `claude` CLI, whose pinned version is named only in `rust.yml` and `handoff.md`.
  - Recommendation: —
  - Priority: High

- [x] 34. Can an agent see the results of a failed run — do the tools here produce output it can read and act on?
  *Judge from the runs you already did in Verification: does a failure print a path, a line, a name — something actionable — or a wall of noise? If nothing could be run, FAIL saying why.*
  - Status: PASS
  - Proof: The failing run printed `error: test failed, to rerun pass --test plugin`, naming the target and the rerun command; Rust panics print `file:line` and left/right values. `.config/nextest.toml` sets `slow-timeout = { period = "60s", terminate-after = 3 }`, so a hung test is ended and named (D-144), and CI steps get their own timeouts so logs survive. CI errors use `::error::` annotations with a reason ("a network crate reached the default build (CA-10)").
  - Recommendation: —
  - Priority: Low

## 5. Safety

- [x] 35. Are credentials kept out of the repo — nothing secret committed, ignore rules in place, an example env file for the shape?
  *Three checks: grep tracked files for key-shaped strings, read the ignore rules for env and key patterns, and look for an example env file. Report each of the three separately.*
  - Status: PASS
  - Proof: (1) Tracked-file grep for `sk-…`, `AKIA…`, `ghp_…`, `xox[bp]-`, `BEGIN … PRIVATE KEY` hits only `tests/online_units.rs:449` and `tests/props_fs.rs:169-170`. Both are synthetic stubs the tests write to a temp dir to check the reader excludes them (`b"KEY = '''\n-----BEGIN OPENSSH PRIVATE KEY-----\nabc\n'''\n"`). (2) `.gitignore` ignores `.env` and `.envrc` (comment cites D-089). (3) No example env file, by design: config is CLI flags plus one variable, and `README.md` §Secrets documents it (`RIPWIRE_BROKER_JEV_API_KEY`, "never accepted as an argument, never written by `install`").
  - Recommendation: —
  - Priority: High

- [ ] 36. Does anything scan for secrets automatically?
  *Look for gitleaks, trufflehog, detect-secrets or this ecosystem's equivalent, wherever it is wired in — a CI step, a pre-commit hook, or forge-level push protection visible from the repo. CI is where this normally lives, and that is a PASS; a local hook on top is better, because it catches the key before it is pushed rather than after, but its absence is a line in the proof, not a FAIL. Say where the scan runs. "Nothing secret exists today" does not make this N/A — the scan is for the day that changes.*
  - Status: FAIL
  - Proof: The only automated scan is the `rust.yml` step "Fixtures stay synthetic", which greps `tests/fixtures/` alone for `Bearer |api_key|authorization|sk-[A-Za-z0-9]{10}`. No gitleaks/trufflehog/detect-secrets anywhere; no pre-commit hook; `gh api repos/aquental/ripwire-broker --jq .security_and_analysis` shows `secret_scanning`, `secret_scanning_push_protection` and `secret_scanning_non_provider_patterns` all `disabled`.
  - Recommendation: Turn on GitHub secret scanning with push protection for the repo (free on public repos), and add a `gitleaks` job to `rust.yml` that scans the whole tree on `pull_request`, pinned by SHA like the other actions.
  - Priority: High

- [x] 37. Are dependencies pinned, so a build is reproducible?
  *Look for lockfiles in every package of the repo, exact versions for load-bearing dependencies, a pinned runtime, and an install command that respects the lock (npm ci, not npm install).*
  - Status: PASS
  - Proof: `Cargo.lock` committed and every CI cargo command uses `--locked`; `rust-toolchain.toml` pins 1.98.1; the load-bearing SDK is exact (`rust-mcp-sdk = "=2.0.0"`, D-002/D-105); actions are pinned by full SHA; the `claude` CLI is pinned to `2.1.292`; the plugin pins its release binaries by SHA-256 in `integrations/claude-code/scripts/checksums.txt`. The TS plugin has no npm dependencies, so it has no lockfile to pin. One deliberate exception: CI installs the latest ripwire release, unpinned (D-188), to catch upstream breakage.
  - Recommendation: —
  - Priority: High

- [x] 38. Is anything watching those dependencies for known vulnerabilities?
  *Look for dependabot or renovate config, an audit step in CI, or this ecosystem's equivalent. Check every lockfile in the repo is covered, not just the root one.*
  - Status: PASS
  - Proof: `.github/dependabot.yml`: weekly `cargo` and `github-actions` updates (with `rust-mcp-sdk` and `sha2 >= 0.11` ignored for documented reasons). `supply-chain.yml` runs `cargo deny --all-features check` (advisories, bans, licenses, sources) weekly and on lockfile/manifest pushes; `deny.toml` has `ignore = []` for advisories. `Cargo.lock` is the only lockfile, so coverage is complete. (`dependabot_security_updates` is `disabled` at the forge; the weekly version updates plus cargo-deny cover it.)
  - Recommendation: —
  - Priority: Low

- [ ] 39. Are the review rules written down — who reads a change, and what they check?
  *Look for CONTRIBUTING, a PR template, or a review checklist in the agent instruction set. On a solo repo the "who" is N/A-shaped but the "what gets checked before it lands" still matters — judge that half.*
  - Status: FAIL
  - Proof: No CONTRIBUTING, no `.github/pull_request_template.md`, no CODEOWNERS (`ls .github` → `dependabot.yml workflows`). Branch protection requires only the `default` and `online` checks (`required_pull_request_reviews: null`). The "what gets checked" half is scattered: TDD-first, mutation testing, D-NNN entry per decision, English commits, no new dependency without reason (`handoff.md` §Convenções), and the required-check names (`rust.yml` comment, D-107). Nothing assembles it into a checklist a PR author or agent sees.
  - Recommendation: Add `.github/pull_request_template.md` with the `handoff.md` §Convenções checklist (red test first, D-NNN entry in `spec/changelog.md`, no new crate in the default graph, `checksums.txt` updated on release PRs).
  - Priority: Low

- [ ] 40. Are the operations that need a human named somewhere an agent will read them?
  *Look for a "never without asking" list in the agent instruction file or the README. Docs that hand out production commands with no fence around them count against, and the proof should quote one.*
  - Status: FAIL
  - Proof: No agent instruction file (see the agent-instruction-file item), and `README.md` hands out destructive and outward-facing commands with no "ask first" fence. Examples: `memory forget --workspace DIR (--all | --id ID)` ("Forgets ... everything (which also revokes collection)"); `ripwire-broker install claude-code --workspace /repo --online --write`, which writes host config; `memory drain ... --online`, which spends classifier calls; and the release flow (push a `v*` tag) in `handoff.md` §"Como verificar".
  - Recommendation: In the new root `AGENTS.md`, add a "never without asking" list: pushing a `v*` tag, editing `checksums.txt`, `install … --write`, `memory forget --all`, any `--online` run or `--ignored` live test that spends API credit, and changes to the required-check job names in `rust.yml`.
  - Priority: High

- [ ] 41. Does every action that spends money, destroys data or changes production have a human in the way?
  *Start from the damage, not from the tooling: list what in reach of this repo could charge a card, drop or overwrite data, or alter what users are running. Then trace the shortest route an agent could take to each one — a push that auto-deploys, a script carrying live credentials, a migration that runs on merge, an infrastructure apply with no plan-and-approve step. PASS when every route meets a human first, whether that is a review, a manual trigger or a protected environment. FAIL when even one route runs start to finish unattended, and quote that route in the proof so the fix is obvious.*
  - Status: FAIL
  - Proof: Damage in reach: (a) publishing binaries to users (GitHub releases; plugin users via `checksums.txt`); (b) spending classifier credit (`--online`, live tests, `memory drain --online`); (c) deleting the local memory store (`memory forget --all`). Unattended route for (a): `git push origin vX.Y.Z` → `release.yml` (`on: push: tags: ["v*"]`) → gates → draft → build → `gh release edit "$tag" --draft=false`, with no `environment:` protection (`gh api .../environments` returns none) and no tag ruleset (the only ruleset, `protect`, covers `~DEFAULT_BRANCH` with `deletion`/`non_fast_forward`). Second route: master requires checks but `required_pull_request_reviews: null`, so a PR that bumps `checksums.txt` can be merged with no human reading it. (b) and (c) need the key or a local command and do meet a human today.
  - Recommendation: Put the `publish` job of `release.yml` behind a GitHub `environment: release` with a required reviewer, and add a tag ruleset restricting `v*` creation; require one approving review on `master` for changes to `integrations/claude-code/scripts/checksums.txt` (CODEOWNERS on that path).
  - Priority: High

- [ ] 42. If a prompt injection landed tonight, how far would it reach — are the credentials an agent can get to here scoped to the job, with nothing production-grade in reach?
  *Inventory what an agent in this repo can reach: env files, cloud CLI profiles, tokens named in docs or config, deploy commands that work from a laptop. Scoped-or-absent passes; production-grade reach fails with the item named.*
  - Status: FAIL
  - Proof: In this shell, `printenv` shows no `RIPWIRE_BROKER_JEV_API_KEY`, Cloudflare or GitHub token variables (names only, values not read). `gh auth status`: logged in as the repo owner via keyring with scopes `'admin:public_key', 'gist', 'read:org', 'repo'`. `repo` scope from the owner's account is enough to push a `v*` tag (an unattended release, see the human-gate item) and to merge PRs, since no review is required. That is production-grade reach for any agent that can run `gh` or `git push` here.
  - Recommendation: Run agents with a fine-grained GitHub token scoped to this repo with `contents:read` and `pull-requests:write` (no tag push, no merge), and keep the owner's `repo`-scoped token out of agent sessions.
  - Priority: High

- [x] 43. Can a change reach production a slice at a time — a feature flag that defaults to off, a canary, a staged rollout — rather than everyone at once?
  *Look for a flag system and check the default, or canary and staged-rollout config in the deploy pipeline. Flags that need a rebuild to flip are worth naming in the proof — they gate exposure but they are not a kill switch.*
  - Status: PASS
  - Proof: New behaviour ships off by default and is enabled per run without a rebuild: `--online` (runtime flag, and the `online` Cargo feature is off in the default build, CA-10), `--memory` ("experimental"), `--incremental`; the plugin exposes these as `user_config` toggles (`integrations/claude-code/.mcp.json`: `RIPWIRE_BROKER_PLUGIN_ONLINE`, `_MEMORY`, `_INCREMENTAL`). Binaries reach plugin users only when `checksums.txt` is updated in a later PR, which is a natural stage. There is no canary, which suits a locally installed tool.
  - Recommendation: —
  - Priority: High

- [x] 44. Once a change is live, can anyone see what it is doing — logs, metrics, traces, alerts that fire on their own, and can an agent read them too?
  *Look for logging setup, an error tracker, analytics, alerting config — and then ask the second half: could an agent reach any of it (a CLI, an MCP server, an API named in the docs), or does observability stop at a dashboard behind a login?*
  - Status: PASS
  - Proof: No hosted service and no telemetry, by design (local, offline, CA-10). Everything that does exist is agent-readable: the `ripwire-broker://status` MCP resource (counts, durations, error kinds, D-015), `hook-log --session`, `hook-stats --json`, `doctor --json`, `memory status --json`, `--memory-debug-log` (one line per event in `debug.log`, for `tail -F`) and `jev.log`. CI failures and the weekly cargo-deny run reach the owner through GitHub notifications and `gh run list`.
  - Recommendation: —
  - Priority: Low

- [ ] 45. Is there a way back — can a bad change be undone without a rebuild and a redeploy, including the ones that touched a database or a queue?
  *Look for a documented rollback path, a revert-and-redeploy story, down-migrations, or a flag that can turn the change off at runtime. Deployment docs that only say how to go forward are worth quoting.*
  - Status: FAIL
  - Proof: Mechanisms exist but nothing documents a way back. A bad release could be undone by reverting `checksums.txt` to the previous tag (the old release assets stay), with no rebuild. The memory store refuses an unknown `schema_version` (`src/memory/store.rs:1341`, `Unavailable::UnknownSchema`) rather than misreading it, and `memory forget` exists. But `grep -i 'rollback|roll back|revert|previous version|yank'` across `README.md`, `integrations/claude-code/README.md` and `handoff.md` returns nothing: "Publishing a version" only goes forward. There is no down-migration story for a future `SCHEMA_VERSION` 2.
  - Recommendation: Add a "Rolling back a version" subsection under "Publishing a version" in `integrations/claude-code/README.md`: revert `checksums.txt` to the prior tag, mark the bad GitHub release as pre-release, and what an older binary does with a newer memory store (it refuses it; `memory forget --all` is the escape).
  - Priority: Low

- [x] 46. Does a past failure leave a check behind — is there a test or a rule in here that exists because something broke once?
  *Look for tests, lint rules or comments that name the incident they guard against — "regression", "this broke when", a linked issue. On a very young repo with no failures yet, N/A with the age as the reason.*
  - Status: PASS
  - Proof: `tests/eval.rs:1977,2012` cite `#122`; commit a5854b0 rewrote `a_hung_upstream_times_out_and_is_restarted` after it flaked (#120); `tests/eval.rs`/`tests/hook_durability.rs` cite D-141/D-143/D-174; `.config/nextest.toml` exists "because a test that hangs ... leaves no log at all (audit fixes, D-144)"; the CI fixture guard exists because "a future recording taken straight from the provider is the likeliest way a secret ... enters"; `handoff.md` keeps an "Armadilhas já pisadas" section.
  - Recommendation: —
  - Priority: Low

---

Learn more: **https://ainativesoftware.engineering/**
