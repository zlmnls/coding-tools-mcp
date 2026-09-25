# Skill Match Convergence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the ambiguous `skill_search` MCP entry with a single task-oriented `skill_match` tool and make progressive Skill use understandable to context-free Agents.

**Architecture:** Registered Skills expose lightweight trigger/capability metadata. `skill_match` accepts the complete user task, scores registered Skills using names, descriptions, tags, triggers, and capabilities, and returns ranked candidates with reasons and readiness. `skill_load` can read Skill instructions even when dependencies are pending, while `skill_exec` remains responsible for execution readiness. MCP initialization instructions document the required match → load → resource → exec workflow.

**Tech Stack:** Rust, serde, serde_json, Tauri MCP registry/dispatcher, Svelte/TypeScript API types if needed.

---

### Task 1: Skill metadata for matching

**Files:**
- Modify: `src-tauri/src/skills/model.rs`
- Modify: `src-tauri/src/skills/parser.rs`
- Modify: `src-tauri/src/skills/mod.rs`

- [ ] Add `triggers`, `capabilities`, and `requires_execution` fields to Skill metadata and registration DTOs with backward-compatible defaults.
- [ ] Parse optional frontmatter-style `triggers`, `capabilities`, and `requires_execution` fields.
- [ ] Fall back to name, description, and tags when fields are absent.
- [ ] Include matching metadata in `skill_list` output.
- [ ] Add parser tests for declared and fallback metadata.

### Task 2: Replace MCP search with task matching

**Files:**
- Modify: `src-tauri/src/tools/registry.rs`
- Modify: `src-tauri/src/tools/dispatch.rs`
- Add/modify: `src-tauri/src/tools/skill_tests.rs`

- [ ] Remove `skill_search` from all exposed/core/read-only tool lists and schema.
- [ ] Add `skill_match` with required `task` and optional `limit` (1-10, default 3).
- [ ] Score candidates by token matches across name, description, tags, triggers, and capabilities.
- [ ] Return `skill_id`, metadata, score, reason, availability, and next action.
- [ ] Return an empty matches list rather than an error when nothing is relevant.
- [ ] Test ranking, readiness metadata, and that `skill_search` is no longer exposed.

### Task 3: Progressive loading and MCP instructions

**Files:**
- Modify: `src-tauri/src/skills/mod.rs`
- Modify: `src-tauri/src/mcp/server.rs`

- [ ] Allow `skill_load` to return content when dependencies are pending, including `availability`, `can_execute`, and dependency details.
- [ ] Keep `skill_exec` blocked unless the Skill is enabled, execution is allowed, and dependencies are ready.
- [ ] Add explicit initialize instructions: use `skill_match` first, then `skill_load`, load resources on demand, and only then use `skill_exec`.
- [ ] Explain local versus sandbox execution and the no-match fallback.

### Task 4: Verification

**Files:**
- Modify tests as required by compiler diagnostics.

- [ ] Run `cargo fmt --check` and `cargo test`.
- [ ] Run `npm run check`.
- [ ] Run `npm run desktop:build` for Apple Silicon.
- [ ] Verify `tools/list` includes `skill_list`, `skill_match`, `skill_load`, `skill_exec`, but not `skill_search`.
