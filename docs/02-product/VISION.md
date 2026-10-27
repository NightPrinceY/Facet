# Vision

## One line
Facet is the local control layer for everyone who uses more than one Claude account: keep them apart, see what each costs to the second, control every setting, and install third-party tools safely.

## The problem
Claude Desktop holds one account at a time. Claude Code supports one account per config folder, but nothing ties the two together. People with an employer seat plus a personal plan, and consultants with one account per client, juggle sign-outs, browser profiles and shell aliases. They cannot answer simple questions:

- Which account am I typing into right now?
- What did each account, project, client and conversation cost this week?
- Which conversation, task or single tool run burned the most?
- Are my accounts set up the same way, and are they allowed to do the same things?
- What will this skill pack, plugin or MCP server actually do on my machine?

## Who it is for
1. **Consultants, freelancers and agencies** with an account per client. They bill clients, so costs per client and invoice export are worth paying for. *Paying core.*
2. **People with an employer seat and a personal plan**, especially on Linux and Windows where nothing serves them. *Largest group, the way in.*
3. **Power users** who run many parallel sessions and want to see and steer everything. *Advocates.*

Not for: people stacking subscriptions to stretch usage limits.

## What makes it different
- **Desktop and Claude Code accounts in one place, on all three systems.** Others cover one OS or only the terminal.
- **The Timeline.** Every prompt, model call and tool run across all accounts on one zoomable axis, from a month to a millisecond, with replay.
- **Costs that answer real questions.** By account, project, client, conversation, task, tool call, file and MCP server.
- **Control with a diff.** Every Claude Code setting per account, presets, and the exact file change before saving.
- **Safety before install.** A plain-language preview of hooks, permissions, network access and files for any skill, plugin or MCP server.
- **Compliant by construction.** Official binaries only, no token handling, no calls to Anthropic, no rotation, no UI automation.

## Principles
1. **Local first.** All data stays on the computer. No account, no cloud, no telemetry about the user.
2. **Never between the user and Anthropic.** Facet launches Claude and listens to telemetry Claude already exports. It never proxies, intercepts or impersonates.
3. **Show before change.** Every write shows a diff and makes a backup.
4. **Identity everywhere.** The active account's colour is visible wherever the user can type or act.
5. **Survive the platform.** Anthropic will ship native features. Facet's value must survive native multi-account: costs, control, safety, Linux.

## Where it goes (beyond v1)
- **Team edition:** shared policy packs, an approved catalogue of skills and MCP servers, audit of who changed what.
- **Phone companion:** see which sessions need you and what they cost, answer from anywhere.
- **Workspace memory:** per-project notes and context that follow the right account.
- **Anomaly alerts:** retry loops, runaway tool output, a sudden cost spike, a session stuck waiting.
- **Plan advisor:** given real usage, which plan each account should be on.
