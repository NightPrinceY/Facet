# Risks

| Risk | Likelihood | Impact | Mitigation | Signal to watch |
|---|---|---|---|---|
| Anthropic ships Desktop multi-account | High (months) | High for isolation, low for the rest | Build value in Timeline, costs, control, safety; adopt native switching when it lands | Activity on #18435, Desktop release notes, mobile rollout |
| cc-switch adds multiple Claude logins | Medium | High | Differentiate on Desktop support, Timeline depth, safety preview; consider integration | cc-switch releases |
| Anthropic tightens terms on third-party tools | Medium | High | Compliance-by-construction rules; no tokens, no proxying, no rotation | Legal page changes, enforcement news |
| Claude update breaks multi-instance or identity tricks | High (recurring) | Medium | Versioned adapters, Health detection, fast patch releases | Each Claude release |
| OTel or transcript fields change | Medium | Medium | Versioned mappers, tolerant parser, dual sources | Claude Code changelog |
| Too few people pay | Medium | High | Validate with interviews and waitlist first; kill criteria | Waitlist size, interview answers |
| Windows signing and payments from MENA | Medium | Medium | Microsoft Store; Lemon Squeezy where supported | Seller country support |
| Solo developer bandwidth across three OSes | High | Medium | Linux first; strict phase exits; open source contributors | Phase slip |
| Paid Studio features (AI drafting, trigger lab) run on the user's Free/Pro/Max subscription through headless Claude Code; Anthropic's legal page bars third parties "routing requests through Free/Pro/Max credentials" | Medium (unclear) | High | Legal opinion or ask Anthropic before Studio v2; one user click per run, command shown first, "copy command", no background runs; consider making AI-run features free | Legal page changes, Anthropic reply |
| Library or archive leaks secrets (tokens in MCP env, pasted keys in transcripts) | Medium | Very high (trust) | Secret scan on library write/commit/push; archive off by default, redacted, encrypted | Scanner hits in testing |
| A user's account gets flagged while using Facet | Low | Very high (trust) | Never touch tokens or automate; clear docs on what Facet does; no rotation | User reports |
