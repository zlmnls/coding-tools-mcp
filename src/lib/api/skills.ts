import { invoke } from "@tauri-apps/api/core";

export type SkillAvailability = "Registered" | "DependenciesPending" | "Ready" | "Broken";
export interface SkillDependency { name: string; requirement?: string; kind: string; installed: boolean; detail?: string; }
export interface SkillMetadata { id: string; name: string; description: string; tags: string[]; triggers: string[]; capabilities: string[]; requires_execution: boolean; source_path: string; enabled: boolean; availability: SkillAvailability; resources: string[]; dependencies: SkillDependency[]; sandbox: { root: string; runtime: string; workspace: string; permission: string; network: boolean }; execution_mode: "Local" | "ReadOnly" | "Sandbox"; allow_execution: boolean; }
export interface InstallPlan { skill_id: string; commands: string[]; network: boolean; target: string; missing_system_dependencies: string[]; }
export interface TerminalResult { command: string; cwd: string; stdout: string; stderr: string; exit_code: number | null; }
export interface InstallSession { session_id: string; sandbox_root: string; workspace: string; network: boolean; created_at: string; }
export interface SkillCandidate { path: string; name: string; description: string; }
export type SkillSourceScope = "project" | "user" | "system" | "custom";
export interface SkillSourceRoot { scope: SkillSourceScope; label: string; path: string; exists: boolean; readable: boolean; }
export interface DiscoveredSkill { id: string; name: string; description: string; path: string; scope: SkillSourceScope; source_label: string; registered: boolean; conflict_rank: number; conflict: boolean; }

export const listSkills = () => invoke<SkillMetadata[]>("list_skills");
export const registerSkill = (path: string) => invoke<SkillMetadata>("register_skill", { path });
export const unregisterSkill = (id: string) => invoke<void>("unregister_skill", { id });
export const loadSkill = (id: string, resource?: string) => invoke<{ content: string }>("load_skill", { id, resource });
export const skillInstallPlan = (id: string) => invoke<InstallPlan>("skill_install_plan", { id });
export const installSkillDependencies = (id: string) => invoke<void>("install_skill_dependencies", { id });
export const runSkillTerminal = (id: string, command: string) => invoke<TerminalResult>("run_skill_terminal", { id, command });
export const setSkillExecution = (id: string, mode: "Local" | "ReadOnly" | "Sandbox", allowExecution: boolean) => invoke<SkillMetadata>("set_skill_execution", { id, mode, allowExecution });

export const createSkillInstallSession = () => invoke<InstallSession>("create_skill_install_session");
export const runSkillInstallTerminal = (sessionId: string, command: string) => invoke<TerminalResult>("run_skill_install_terminal", { sessionId, command });
export const detectInstalledSkills = (sessionId: string) => invoke<SkillCandidate[]>("detect_installed_skills", { sessionId });
export const registerDetectedSkill = (sessionId: string, path: string) => invoke<SkillMetadata>("register_detected_skill", { sessionId, path });
export const cancelSkillInstallSession = (sessionId: string) => invoke<void>("cancel_skill_install_session", { sessionId });
export const discoverSkills = (workspacePath?: string, customRoots: string[] = []) => invoke<DiscoveredSkill[]>("discover_skills", { workspacePath, customRoots });
export const listSkillSourceRoots = (workspacePath?: string, customRoots: string[] = []) => invoke<SkillSourceRoot[]>("list_skill_source_roots", { workspacePath, customRoots });
export const registerDiscoveredSkill = (path: string) => invoke<SkillMetadata>("register_discovered_skill", { path });
export const registerDiscoveredSkillsBatch = (paths: string[]) => invoke<SkillMetadata[]>("register_discovered_skills_batch", { paths });
