---
name: decision-entry
description: >-
  Record a decision of this repository in spec/changelog.md: one index row at the end of the
  index and one `## D-NNN — …` section at the end of the file, in Portuguese, dated with `date`.
  Use whenever a choice is made while working here (a design, a scope cut, a rejected option, a
  changed plan), before committing the work it explains.
---

# Decision entry

Every decision gets a D-NNN entry (`AGENTS.md`, Rules; `handoff.md` §"Convenções"). The
changelog is the source of truth on why things are the way they are, so an entry says what was
decided, why, what was rejected and how it is tested.

## Steps

1. **The number.** The highest `## D-` section plus one:
   `grep -o '^## D-[0-9]*' spec/changelog.md | tail -1`.
2. **The date and time, now:** `date '+%Y-%m-%d %H:%M'`. Never invent or round it.
3. **The title,** in Portuguese, as a sentence that states the decision:
   `## D-NNN — <título>`. Its anchor is what GitHub makes of it: lowercase, punctuation
   dropped, each space a hyphen, so `## D-201 — Um título, curto` is `#d-201--um-título-curto`
   (the dash between number and title leaves two hyphens).
4. **The index row,** at the end of the index: after the last `| 20…` row, before the first
   `## ` heading. One line, in Portuguese:
   `| <data e hora> | <o que foi decidido e por quê, em uma ou duas frases> | [D-NNN](#anchor) |`.
   The index runs D-001 to the last in ascending order, one row per section; a pipe inside the
   summary is written `\|`.
5. **The section,** at the end of the file, in Portuguese: what changes; why, with the evidence
   (a measurement, a test, a review finding); what was rejected and why; how it is tested; what
   is left pending, and for whom. Cite other decisions by link,
   `[D-NNN](#anchor)`.
6. **The handoff.** If `handoff.md` now cites the new number, its header ("Estado em …, até o
   D-NNN") cites it too.
7. **Check:** `cargo nextest run --locked --test skills --test agents_md`. The first holds the
   index (one row per section, in order); the second the handoff header and its links.

## Where it stops

This skill only edits `spec/changelog.md` and, when step 6 applies, `handoff.md`. It runs nothing
from the list in `AGENTS.md` "Ask before": it never runs `git push`, never creates a `git tag` or a
release, never merges. Committing the entry with the work it explains is fine; what happens to the
commit afterwards is the maintainer's.
