# Skill Source Discovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (recommended) to implement this plan task-by-task.

**Goal:** Automatically discover Trae, Codex, project, system, and custom local Skills and let users register them into MCP without manually locating each directory.

**Architecture:** Add a Rust discovery service that enumerates configured source roots, validates contained `SKILL.md` files, labels source scope, ranks conflicts, and returns candidates without copying source files. Registration reuses the existing local execution model and stores the original path. The Svelte Skill page adds source filters, scan/refresh, candidate cards, conflict metadata, and inherit actions.

**Tech Stack:** Rust, serde, Tauri commands, Svelte 5, TypeScript, existing Skill parser/storage.

---

### Task 1: Discovery domain model and source enumeration

**Files:**
- Create: `src-tauri/src/skills/discovery.rs`
- Modify: `src-tauri/src/skills/model.rs`
- Modify: `src-tauri/src/skills/mod.rs`

- [ ] Define source scope (`project`, `user`, `system`, `custom`) and serializable discovery candidate DTOs.
- [ ] Enumerate macOS and cross-platform Trae/Codex/Agents/Claude/Cursor roots plus workspace roots.
- [ ] Include configured custom roots without accepting unsafe relative paths.
- [ ] Recursively discover valid `SKILL.md` directories with depth/count limits, skipping dependency/cache directories.
- [ ] Filter memory Skills and deduplicate exact paths.
- [ ] Rank conflicts as project > user > system > custom and mark registered candidates.
- [ ] Add Rust unit tests for source labeling, filtering, deduplication, and conflict ranking.

### Task 2: Discovery and inheritance commands

**Files:**
- Modify: `src-tauri/src/commands/skills.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/skills/mod.rs`

- [ ] Add `discover_skills(workspace_path, custom_roots)` returning candidates.
- [ ] Add `register_discovered_skill(path)` using existing local registration and preserving the original source path.
- [ ] Add `list_skill_source_roots` for UI diagnostics.
- [ ] Ensure system sources are read-only and registration never mutates source directories.
- [ ] Preserve local execution defaults and existing memory Skill exclusion.

### Task 3: Frontend API and discovery UI

**Files:**
- Modify: `src/lib/api/skills.ts`
- Modify: `src/routes/settings/skills/+page.svelte`

- [ ] Add typed discovery candidate/source-root interfaces and invoke wrappers.
- [ ] Add “发现本地 Skill” panel with scan/refresh controls.
- [ ] Display source scope, path, registration state, conflict rank, description, and execution mode.
- [ ] Add scope filters for all/project/user/system/custom.
- [ ] Add inherit/register action per candidate and refresh registered list after success.
- [ ] Show scan errors and inaccessible system roots without failing the whole scan.
- [ ] Keep existing manual directory registration and terminal installation flows.

### Task 4: Verification

**Files:**
- Add tests under `src-tauri/src/skills/discovery.rs` or existing Rust test modules.

- [ ] Run `cargo fmt --check` and `cargo test` or targeted Rust tests.
- [ ] Run `npm run check`.
- [ ] Run `npm run desktop:build` for Apple Silicon.
- [ ] Confirm no source directory is modified by discovery or inheritance.
- [ ] Confirm MCP `skill_list`, `skill_search`, `skill_load`, and `skill_exec` see inherited Skills.
