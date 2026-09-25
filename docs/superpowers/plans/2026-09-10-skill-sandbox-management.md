# Skill Sandbox Management Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add independently sandboxed Skill registration, dependency management, progressive MCP loading, and a desktop management UI with a sandbox terminal, while keeping the memory Skill unregistered.

**Architecture:** Skills are indexed from source directories but run from per-Skill application-data sandboxes. A Rust Skill Registry owns metadata, dependency state, sandbox paths, and safe reads; MCP exposes only `skill_list`, `skill_search`, and `skill_load`. Tauri commands expose registration, dependency inspection, sandbox lifecycle, and a constrained interactive terminal to Svelte.

**Tech Stack:** Rust, Tauri 2, serde/serde_json, existing process policy, Svelte 5, TypeScript, existing Tailwind-style utility classes.

---

### Task 1: Skill domain model and isolated storage

**Files:**
- Create: `src-tauri/src/skills/mod.rs`
- Create: `src-tauri/src/skills/model.rs`
- Create: `src-tauri/src/skills/parser.rs`
- Create: `src-tauri/src/skills/storage.rs`
- Create: `src-tauri/src/skills/sandbox.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] Define serializable Skill metadata, dependency, sandbox, permission, and availability states.
- [ ] Parse `SKILL.md`, optional `skill.yaml`, and common dependency manifests without executing content.
- [ ] Persist registrations under application data and never delete source directories on unregister.
- [ ] Create per-Skill `source`, `runtime`, `workspace`, `cache`, `tmp`, `logs`, and `state` directories.
- [ ] Exclude memory-related Skills from default discovery and registration candidates.
- [ ] Add unit tests for parsing, path containment, persistence round trips, and sandbox layout.

### Task 2: Dependency planning and safe installation

**Files:**
- Create: `src-tauri/src/skills/dependency.rs`
- Create: `src-tauri/src/skills/resolver.rs`
- Create: `src-tauri/src/skills/installer.rs`
- Create: `src-tauri/src/skills/health.rs`
- Modify: `src-tauri/src/skills/mod.rs`

- [ ] Detect Node/Python manifests and system command requirements.
- [ ] Generate explicit install plans; never execute natural-language commands from `SKILL.md`.
- [ ] Install Node/Python dependencies only into a staging directory inside the Skill sandbox.
- [ ] Enforce no global installs, no sudo, no host filesystem scope, and no project manifest mutation.
- [ ] Atomically activate only after dependency and health checks pass; retain prior environment on failure.
- [ ] Persist redacted install records and dependency status.

### Task 3: MCP progressive Skill tools

**Files:**
- Modify: `src-tauri/src/tools/registry.rs`
- Modify: `src-tauri/src/tools/dispatch.rs`
- Modify: `src-tauri/src/tools/mod.rs`
- Modify: `src-tauri/src/mcp/server.rs`
- Create/modify: `src-tauri/tests/skill_tools.rs`

- [ ] Add fixed `skill_list`, `skill_search`, and `skill_load` tool definitions.
- [ ] Return metadata and availability in list/search without loading full content.
- [ ] Load main content or a contained resource only after enabled/healthy checks.
- [ ] Return structured dependency/sandbox remediation errors when a Skill is not ready.
- [ ] Ensure no memory Skill is exposed.
- [ ] Test list/search/load, path escape rejection, and not-ready responses.

### Task 4: Tauri Skill management commands

**Files:**
- Create: `src-tauri/src/commands/skills.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Create: `src/lib/api/skills.ts`

- [ ] Add commands for listing, registering, unregistering, enabling, refreshing, scanning, dependency planning, installing, health checks, and sandbox cleanup.
- [ ] Add command for granting per-Skill/per-workspace permission without accepting arbitrary paths.
- [ ] Return stable DTOs for Svelte and map errors to existing application error conventions.

### Task 5: Sandbox terminal

**Files:**
- Create: `src-tauri/src/skills/terminal.rs`
- Modify: `src-tauri/src/skills/mod.rs`
- Modify: `src-tauri/src/commands/skills.rs`
- Create: `src/lib/components/SkillTerminal.svelte`
- Modify: `src/lib/api/skills.ts`

- [ ] Implement a Skill-bound terminal session with fixed sandbox cwd, environment, PATH, timeout, and process tracking.
- [ ] Support command input/output, interruption, resize metadata, and session close.
- [ ] Keep terminal commands inside the Skill sandbox and redact sensitive input from audit logs.
- [ ] Allow interactive login commands only after explicit UI confirmation; do not expose arbitrary host cwd.
- [ ] Add terminal session tests for sandbox path enforcement and lifecycle.

### Task 6: Skill management UI

**Files:**
- Create: `src/routes/settings/skills/+page.svelte`
- Modify: `src/routes/+layout.svelte`
- Modify: `src/lib/api/skills.ts`

- [ ] Add navigation entry and page for registered Skills.
- [ ] Implement register directory, scan workspace, enable/disable, refresh, unregister, sandbox cleanup, and dependency plan/install flows.
- [ ] Show source, availability, sandbox, dependency, permissions, network, logs, and resources.
- [ ] Add terminal tab backed by SkillTerminal.
- [ ] Make memory Skills absent from scan and list UI.

### Task 7: Verification

**Files:**
- Modify/add tests under `src-tauri/tests/` and `tests/` as needed.

- [ ] Run Rust formatting and tests.
- [ ] Run `npm run check`.
- [ ] Run `npm run build`.
- [ ] Exercise MCP list/search/load contract and UI type checks.
- [ ] Review changed symbols and confirm no secrets or host paths are logged.
