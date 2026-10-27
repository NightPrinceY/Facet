---
name: new-spike
description: Create a Phase 0 spike folder with a README (Question, Why it matters, Method, Result) and register it in docs/04-plan/PHASE0-TESTS.md. Use when a new unknown needs testing before building.
argument-hint: "<number> <slug> <question>"
disable-model-invocation: true
---

Create a spike for: $ARGUMENTS

1. Read `docs/04-plan/PHASE0-TESTS.md` and one existing `spikes/*/README.md` to match the style.
2. Create `spikes/<NN>-<slug>/README.md` with sections **Question**, **Why it matters** (link the PRD module or decision), **Method** (exact commands; mark steps that need the user, quota, sudo or closing Claude windows), and **Result** set to `_Not run yet._`
3. Any script goes in the same folder, is read-only by default, prints structure and counts only (never message text), and runs with `python3 -I`.
4. Add a row to the table in `PHASE0-TESTS.md`.
5. Show the user the method and what needs them before running anything.
