# OAuth 401 Stability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Reduce intermittent MCP 401 responses by using one canonical OAuth issuer/audience, standard bearer challenges, and safe authentication diagnostics.

**Architecture:** Resolve the OAuth server identity once when the MCP listener starts from the configured public URL, and use that value for token issuance and validation instead of recalculating it from each request's proxy headers. Return `WWW-Authenticate` on all OAuth failures and record only non-secret request/token metadata for diagnosis.

**Tech Stack:** Rust, Axum, jsonwebtoken, existing OAuthRuntime and MCP listener tests.

---

### Task 1: Canonical OAuth server URL

**Files:**
- Modify: `src-tauri/src/mcp/listener.rs`
- Modify: `src-tauri/src/auth/oauth_flow.rs`
- Modify: `src-tauri/src/auth/oauth.rs`

- [ ] Add a canonical server URL field to `OAuthRuntime`.
- [ ] Initialize it from the configured public URL when present, otherwise from the listener's startup external URL.
- [ ] Use the canonical URL for access-token issuer and audience validation on every MCP request.
- [ ] Use the same canonical URL when issuing authorization-code access tokens.
- [ ] Keep request headers for proxy metadata and logging only, not token identity validation.

### Task 2: Standard 401 and diagnostics

**Files:**
- Modify: `src-tauri/src/auth/oauth_flow.rs`
- Modify: `src-tauri/src/mcp/listener.rs`

- [ ] Return `WWW-Authenticate: Bearer` for missing, malformed, and invalid bearer tokens.
- [ ] Include protected-resource metadata URL when a canonical public URL is available.
- [ ] Add safe auth diagnostics to existing MCP request logs: request ID, method, tool, authorization presence, host, forwarded host, canonical URL, and failure category.
- [ ] Never log complete access tokens, secrets, authorization codes, or client credentials.
- [ ] Keep existing successful request logging behavior.

### Task 3: Regression tests

**Files:**
- Modify: `src-tauri/src/auth/oauth_flow.rs`
- Modify: `src-tauri/src/auth/oauth.rs`
- Modify: `src-tauri/src/mcp/listener.rs`

- [ ] Test that changing request Host headers does not change validation against a configured canonical URL.
- [ ] Test that missing and invalid authorization responses include `WWW-Authenticate`.
- [ ] Test that a token issued with the canonical URL validates through a request with different proxy headers.
- [ ] Test that diagnostics contain no bearer token value.

### Task 4: Verification

**Files:**
- No additional files expected.

- [ ] Run targeted Rust tests.
- [ ] Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [ ] Run `npm run check`.
- [ ] Run `npm run desktop:build` for Apple Silicon.
