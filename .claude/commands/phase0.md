---
description: Run or continue the Phase 0 validation tests
---

Read `docs/04-plan/PHASE0-TESTS.md` and the README in each folder under `spikes/`.

1. List the ten tests with their status (not started, in progress, done) based on the Result section of each spike README.
2. Pick the next test that is not done, in order, unless the user named one: $ARGUMENTS
3. Before running anything, state exactly what the test will do on this machine, which accounts it touches, and anything that needs the user (signing in, sudo, closing Claude windows). Wait for approval if it needs the user.
4. Run it. Never read credential files, cookies or keychain items, even to "check" them. Inspect transcripts for structure and field names only, not message content.
5. Write the result into that spike's README under "Result", with date, Claude Code version, commands run and raw evidence.
6. If the result changes a plan, add an entry with `/decide`.
