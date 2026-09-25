import { invoke } from "@tauri-apps/api/core";

export interface McpServerConfig {
  id: string;
  name: string;
  description: string;
  instructions?: string;
  usage_mode?: "always_on" | "on_demand";
  transport: string;
  command: string;
  args: string[];
  env: Record<string, string>;
  enabled: boolean;
  created_at?: string;
  updated_at?: string;
}

export interface McpToolSummary {
  name: string;
  exposed_name: string;
  description: string;
  input_schema: unknown;
}

export interface DiscoveredMcpServer {
  source: string;
  source_file: string;
  config: McpServerConfig;
  already_registered: boolean;
}

export const listExternalMcps = () => invoke<McpServerConfig[]>("list_external_mcps");
export const saveExternalMcp = (config: McpServerConfig) => invoke<void>("save_external_mcp", { config });
export const deleteExternalMcp = (id: string) => invoke<void>("delete_external_mcp", { id });
export const setExternalMcpEnabled = (id: string, enabled: boolean) =>
  invoke<void>("set_external_mcp_enabled", { id, enabled });
export const testExternalMcp = (config: McpServerConfig) =>
  invoke<McpToolSummary[]>("test_external_mcp", { config });
export const discoverExternalMcps = () => invoke<DiscoveredMcpServer[]>("discover_external_mcps");
