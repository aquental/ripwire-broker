## What and why

<!-- One paragraph. Link the decision (D-NNN) or the plan step this implements. -->

## Checklist

From `handoff.md` §"Convenções" and the rules of `AGENTS.md`. Leave a box unticked and say why
rather than delete it; tick one that does not apply and say so.

- [ ] A red test came first, through the public seam in `tests/` (and a mutation shows it bites).
- [ ] Any decision made here has a D-NNN entry, in Portuguese, in `spec/changelog.md` (index row and section).
- [ ] No network crate in the default build (CA-10); any new dependency says why.
- [ ] `scripts/check.sh all` is green locally (or say which target was not run and why).
- [ ] Fixtures in `tests/fixtures/` stay synthetic: no credential, no private source.
- [ ] If this PR touches `integrations/claude-code/` or `.claude-plugin/`, the maintainer was
  asked first (`AGENTS.md`, "Ask before") and it says what changes for plugin users: the
  marketplace serves `master`.

## Verified

What was checked, and how: the commands run and their result, a session tried by hand, a mutation
the new test kills. "Tests pass" alone is not an answer.
