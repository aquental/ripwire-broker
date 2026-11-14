## What and why

<!-- One paragraph. Link the decision (D-NNN) or the plan step this implements. -->

## Checklist

From `handoff.md` §"Convenções". Leave a box unticked and say why rather than delete it.

- [ ] A red test came first, through the public seam in `tests/` (and a mutation shows it bites).
- [ ] Any decision made here has a D-NNN entry in `spec/changelog.md` (index row and section).
- [ ] No network crate in the default build (CA-10); any new dependency says why.
- [ ] `scripts/check.sh all` is green locally (or say which target was not run and why).
- [ ] Fixtures in `tests/fixtures/` stay synthetic: no credential, no private source.
- [ ] Does this PR change what plugin users run (`integrations/claude-code/`, `.claude-plugin/`)?
  If so, say what changes for them; the marketplace serves `master`.

## Verified

What was checked, and how: the commands run and their result, a session tried by hand, a mutation
the new test kills. "Tests pass" alone is not an answer.
