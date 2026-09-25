<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import ActionsAuthForm from "$lib/components/ActionsAuthForm.svelte";
  import ActionsPolicyForm, {
    type ActionsPolicyDraft,
  } from "$lib/components/ActionsPolicyForm.svelte";
  import AuthConfigForm from "$lib/components/AuthConfigForm.svelte";
  import HealthPanel from "$lib/components/HealthPanel.svelte";
  import LogViewer from "$lib/components/LogViewer.svelte";
  import RuntimePolicyForm, {
    type RuntimePolicyDraft,
  } from "$lib/components/RuntimePolicyForm.svelte";
  import ServicePanel from "$lib/components/ServicePanel.svelte";
  import GptQuickCopy from "$lib/components/GptQuickCopy.svelte";
  import StatusOrb from "$lib/components/StatusOrb.svelte";
  import Tabs from "$lib/components/Tabs.svelte";
  import TunnelConfigForm, {
    type TunnelFormConfig,
    type SaveTunnelOptions,
  } from "$lib/components/TunnelConfigForm.svelte";
  import WorkspaceMetaForm from "$lib/components/WorkspaceMetaForm.svelte";
  import {
    deleteWorkspace,
    getActionsRuntimeStatus,
    getRuntimeStatus,
    listWorkspaces,
    startActionsRuntime,
    startRuntime,
    restartRuntime,
    restartActionsRuntime,
    stopActionsRuntime,
    stopRuntime,
    updateWorkspace,
  } from "$lib/api/workspaces";
  import { listFrpProfiles, setLastWorkspace, type FrpProfileDto } from "$lib/api/settings";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { restartTunnel, stopTunnel } from "$lib/api/tunnel";
  import { runServiceToggle, notifyStartFailure } from "$lib/runtime/service";
  import { showToast } from "$lib/stores/toast";
  import { promptServiceRestart } from "$lib/runtime/restart-hint";
  import { actionsRuntimeStates, mcpRuntimeStates, workspaces } from "$lib/stores/app";
  import {
    actionsConfig,
    actionsLocalEndpoint,
    actionsOAuthAuthorizeUrl,
    actionsOAuthTokenUrl,
    actionsOpenApiUrl,
    actionsPrivacyUrl,
    frpPublicUrl,
    mcpLocalEndpoint,
    type AuthConfig,
    type ActionsAuthDraft,
    type RuntimeState,
    type WorkspaceProfile,
  } from "$lib/types";

  type ServiceTab = "mcp" | "actions";
  type SubTab = "config" | "logs" | "health";

  let profile = $state<WorkspaceProfile | null>(null);
  let mcpStatus = $state<RuntimeState>("stopped");
  let actionsStatus = $state<RuntimeState>("stopped");
  let mcpStatusMessage = $state("");
  let actionsStatusMessage = $state("");
  let mcpBusy = $state(false);
  let actionsBusy = $state(false);
  let mcpLocal = $state("");
  let mcpPublic = $state("");
  let actionsLocal = $state("");
  let actionsPublic = $state("");
  let frpProfiles = $state<FrpProfileDto[]>([]);

  let activeService = $state<ServiceTab>("mcp");
  let mcpSubTab = $state<SubTab>("logs");
  let actionsSubTab = $state<SubTab>("logs");
  let loadGeneration = 0;

  let showGptModal = $state(false);
  let showSettingsModal = $state(false);
  let restartingTunnel = $state(false);

  async function handleQuickRestartTunnel() {
    if (!workspaceId || !profile || restartingTunnel) return;
    restartingTunnel = true;
    try {
      const status = await restartTunnel(workspaceId, activeService);
      if (status.publicUrl) {
        if (activeService === "mcp") {
          mcpPublic = `${status.publicUrl.replace(/\/$/, "")}/mcp`;
        } else {
          actionsPublic = `${status.publicUrl.replace(/\/$/, "")}/openapi.json`;
        }
      }
      showToast("已触发隧道重新连接", { kind: "success" });
    } catch (error) {
      showToast(String(error), { title: "隧道重启失败", kind: "error" });
    } finally {
      restartingTunnel = false;
    }
  }

  async function copyEndpoint(url: string) {
    if (!url) {
      showToast("当前暂无公网地址", { kind: "warning" });
      return;
    }
    try {
      await navigator.clipboard.writeText(url);
      showToast("已复制地址到剪贴板", { kind: "success" });
    } catch {
      showToast("复制失败", { kind: "error" });
    }
  }

  const subTabs = [
    { value: "logs", label: "实时日志" },
    { value: "config", label: "网络与隧道配置" },
    { value: "health", label: "健康检查" },
  ];

  const workspaceId = $derived($page.params.id);
  const actions = $derived(profile ? actionsConfig(profile) : null);

  const mcpTunnelForm = $derived<TunnelFormConfig>({
    type: profile?.tunnel.type ?? "none",
    public_url: profile?.tunnel.public_url ?? "",
    frp_server: profile?.tunnel.frp_server ?? "",
    frp_subdomain: profile?.tunnel.frp_subdomain ?? "",
    frp_profile_id: profile?.tunnel.frp_profile_id ?? "",
    frp_server_port: profile?.tunnel.frp_server_port ?? 7000,
    cloudflare_mode: profile?.tunnel.cloudflare_mode ?? "quick",
    cloudflare_http2: profile?.tunnel.cloudflare_http2 ?? true,
    use_proxy: profile?.tunnel.use_proxy ?? true,
  });

  const actionsTunnelForm = $derived<TunnelFormConfig>({
    type: actions?.tunnel_type ?? "none",
    public_url: actions?.public_url ?? "",
    frp_server: actions?.frp_server ?? "",
    frp_subdomain: actions?.frp_subdomain ?? "",
    frp_profile_id: actions?.frp_profile_id ?? "",
    frp_server_port: actions?.frp_server_port ?? 7000,
    cloudflare_mode: actions?.cloudflare_mode ?? "quick",
    cloudflare_http2: actions?.cloudflare_http2 ?? true,
    use_proxy: actions?.use_proxy ?? true,
  });

  function stateLabel(state: RuntimeState): string {
    switch (state) {
      case "running":
        return "运行中";
      case "starting":
        return "启动中";
      case "stopping":
        return "停止中";
      case "error":
        return "错误";
      default:
        return "已停止";
    }
  }

  function applyMcpRuntime(
    runtime: { state: RuntimeState; localEndpoint: string; publicEndpoint: string; localMessage?: string },
    id = workspaceId,
  ) {
    if (!id || id !== workspaceId) return;
    mcpStatus = runtime.state;
    mcpStatusMessage = runtime.localMessage ?? "";
    mcpLocal = runtime.localEndpoint;
    mcpPublic = runtime.publicEndpoint;
    mcpRuntimeStates.update((current) => ({ ...current, [id]: runtime.state }));
  }

  function applyActionsRuntime(runtime: {
    state: RuntimeState;
    localEndpoint: string;
    publicEndpoint: string;
    localMessage?: string;
  },
    id = workspaceId,
  ) {
    if (!id || id !== workspaceId) return;
    actionsStatus = runtime.state;
    actionsStatusMessage = runtime.localMessage ?? "";
    actionsLocal = runtime.localEndpoint;
    actionsPublic = runtime.publicEndpoint;
    actionsRuntimeStates.update((current) => ({ ...current, [id]: runtime.state }));
  }

  async function load(id = workspaceId) {
    if (!id) return;
    const generation = ++loadGeneration;
    const items = await listWorkspaces();
    if (generation !== loadGeneration || id !== workspaceId) return;
    workspaces.set(items);
    frpProfiles = await listFrpProfiles();
    if (generation !== loadGeneration || id !== workspaceId) return;
    const nextProfile = items.find((item) => item.id === id) ?? null;
    if (generation !== loadGeneration || id !== workspaceId) return;
    profile = nextProfile;
    if (nextProfile) {
      await setLastWorkspace(nextProfile.id);
    }
    if (generation !== loadGeneration || id !== workspaceId) return;
    if (!nextProfile) {
      await goto("/");
      return;
    }

    const [mcpRuntime, actionsRuntime] = await Promise.all([
      getRuntimeStatus(id),
      getActionsRuntimeStatus(id),
    ]);
    if (generation !== loadGeneration || id !== workspaceId) return;
    applyMcpRuntime(mcpRuntime, id);
    applyActionsRuntime(actionsRuntime, id);
  }

  async function refreshProfile(id = workspaceId): Promise<WorkspaceProfile | null> {
    if (!id) return null;
    const items = await listWorkspaces();
    if (id !== workspaceId) return null;
    workspaces.set(items);
    const nextProfile = items.find((item) => item.id === id) ?? null;
    profile = nextProfile;
    return nextProfile;
  }

  function tunnelConfigured(type: string | undefined): boolean {
    return type === "cloudflare" || type === "frp";
  }

  async function afterServiceStart(
    service: "mcp" | "actions",
    runtime: { state: RuntimeState; publicEndpoint: string },
    id: string,
  ) {
    const nextProfile = await refreshProfile(id);
    if (id !== workspaceId) return;
    const tunnelType =
      service === "mcp"
        ? nextProfile?.tunnel.type
        : nextProfile
          ? actionsConfig(nextProfile).tunnel_type
          : undefined;
    if (runtime.state === "running" && tunnelConfigured(tunnelType) && !runtime.publicEndpoint) {
      showToast(
        "本地服务已启动，但隧道未能自动连接。请检查代理设置与隧道配置，或查看日志。",
        { title: "隧道未连接", kind: "warning", duration: 8000 },
      );
    }
  }

  async function toggleMcp() {
    const id = workspaceId;
    if (!id || mcpBusy) return;
    const wasRunning = mcpStatus === "running";
    mcpBusy = true;
    try {
      const runtime = await runServiceToggle(
        wasRunning,
        () => startRuntime(id),
        () => stopRuntime(id),
        "MCP",
      );
      if (runtime && id === workspaceId) {
        applyMcpRuntime(runtime, id);
        if (!wasRunning) {
          if (runtime.state === "running") {
            await afterServiceStart("mcp", runtime, id);
          } else {
            notifyStartFailure("MCP", runtime);
          }
        }
      }
    } finally {
      mcpBusy = false;
    }
  }

  async function toggleActions() {
    const id = workspaceId;
    if (!id || actionsBusy) return;
    const wasRunning = actionsStatus === "running";
    actionsBusy = true;
    try {
      const runtime = await runServiceToggle(
        wasRunning,
        () => startActionsRuntime(id),
        () => stopActionsRuntime(id),
        "Actions",
      );
      if (runtime && id === workspaceId) {
        applyActionsRuntime(runtime, id);
        if (!wasRunning) {
          if (runtime.state === "running") {
            await afterServiceStart("actions", runtime, id);
          } else {
            notifyStartFailure("Actions", runtime);
          }
        }
      }
    } finally {
      actionsBusy = false;
    }
  }

  async function saveMcpPort(port: number) {
    if (!profile || profile.runtime.local_port === port) return;
    const next: WorkspaceProfile = {
      ...profile,
      runtime: { ...profile.runtime, local_port: port },
    };
    await updateWorkspace(next);
    profile = next;
    mcpLocal = mcpLocalEndpoint(port);
    await load();
  }

  async function saveActionsPort(port: number) {
    if (!profile) return;
    const current = actionsConfig(profile);
    if (current.local_port === port) return;
    const next: WorkspaceProfile = {
      ...profile,
      actions: { ...current, local_port: port },
    };
    await updateWorkspace(next);
    profile = next;
    actionsLocal = actionsLocalEndpoint(port);
    await load();
  }

  function publicEndpointFromTunnel(config: TunnelFormConfig, suffix: string): string {
    const base = frpPublicUrl(
      config.type,
      config.frp_subdomain,
      config.frp_server,
      config.frp_profile_id,
      frpProfiles,
      config.public_url,
    );
    if (base) {
      return `${base.replace(/\/$/, "")}${suffix}`;
    }
    return "";
  }

  async function restartTunnelIfConfigured(
    targetWorkspaceId: string,
    config: TunnelFormConfig,
    service: "mcp" | "actions",
  ) {
    if (config.type === "none") {
      await stopTunnel(targetWorkspaceId, service);
      return;
    }
    const status = await restartTunnel(targetWorkspaceId, service);
    if (workspaceId !== targetWorkspaceId) return;
    if (status.publicUrl) {
      if (service === "mcp") {
        mcpPublic = `${status.publicUrl.replace(/\/$/, "")}/mcp`;
      } else {
        actionsPublic = `${status.publicUrl.replace(/\/$/, "")}/openapi.json`;
      }
    }
  }

  async function saveMcpTunnel(config: TunnelFormConfig, options?: SaveTunnelOptions) {
    if (!profile) return;
    const targetWorkspaceId = workspaceId;
    if (!targetWorkspaceId) return;
    const next: WorkspaceProfile = {
      ...profile,
      tunnel: {
        ...profile.tunnel,
        type: config.type,
        public_url: config.public_url,
        frp_server: config.frp_server,
        frp_subdomain: config.frp_subdomain,
        frp_profile_id: config.frp_profile_id,
        frp_server_port: config.frp_server_port,
        cloudflare_mode: config.cloudflare_mode,
        cloudflare_http2: config.cloudflare_http2,
        use_proxy: config.use_proxy,
      },
    };
    await updateWorkspace(next);
    if (!options?.skipTunnelRestart) {
      await restartTunnelIfConfigured(targetWorkspaceId, config, "mcp");
    }
    if (workspaceId !== targetWorkspaceId) return;
    profile = next;
    mcpPublic = publicEndpointFromTunnel(config, "/mcp");
    if (!options?.skipTunnelRestart && !options?.skipServicePrompt) {
      await load();
      if (workspaceId !== targetWorkspaceId) return;
    }
    if (!options?.skipServicePrompt) {
      await promptServiceRestart(mcpStatus === "running", "MCP 服务");
    }
  }

  async function saveActionsTunnel(config: TunnelFormConfig, options?: SaveTunnelOptions) {
    if (!profile) return;
    const targetWorkspaceId = workspaceId;
    if (!targetWorkspaceId) return;
    const current = actionsConfig(profile);
    const next: WorkspaceProfile = {
      ...profile,
      actions: {
        ...current,
        tunnel_type: config.type,
        public_url: config.public_url,
        frp_server: config.frp_server,
        frp_subdomain: config.frp_subdomain,
        frp_profile_id: config.frp_profile_id,
        frp_server_port: config.frp_server_port,
        cloudflare_mode: config.cloudflare_mode,
        cloudflare_http2: config.cloudflare_http2,
        use_proxy: config.use_proxy,
      },
    };
    await updateWorkspace(next);
    if (!options?.skipTunnelRestart) {
      await restartTunnelIfConfigured(targetWorkspaceId, config, "actions");
    }
    if (workspaceId !== targetWorkspaceId) return;
    profile = next;
    actionsPublic = publicEndpointFromTunnel(config, "/openapi.json");
    if (!options?.skipTunnelRestart && !options?.skipServicePrompt) {
      await load();
      if (workspaceId !== targetWorkspaceId) return;
    }
    if (!options?.skipServicePrompt) {
      await promptServiceRestart(actionsStatus === "running", "Actions 服务");
    }
  }

  async function saveMcpPolicy(draft: RuntimePolicyDraft) {
    if (!profile) return;
    const next: WorkspaceProfile = {
      ...profile,
      runtime: {
        ...profile.runtime,
        tool_profile: draft.toolProfile,
        permission_mode: draft.permissionMode,
        allowed_commands: draft.allowedCommands,
        workspace_local_entries: draft.workspaceLocalEntries,
        workspace_script_extensions: draft.workspaceScriptExtensions,
      },
    };
    await updateWorkspace(next);
    profile = next;
    await load();
    await promptServiceRestart(mcpStatus === "running", "MCP 服务");
  }

  async function saveActionsPolicy(draft: ActionsPolicyDraft) {
    if (!profile) return;
    const current = actionsConfig(profile);
    const next: WorkspaceProfile = {
      ...profile,
      actions: {
        ...current,
        allowed_commands: draft.allowedCommands,
        max_patch_bytes: draft.maxPatchBytes,
        permission_mode: draft.permissionMode,
      },
    };
    await updateWorkspace(next);
    profile = next;
    await load();
    await promptServiceRestart(actionsStatus === "running", "Actions 服务");
  }

  async function saveMcpAuth(auth: AuthConfig, options?: { skipRuntimeRestart?: boolean }) {
    if (!profile || !workspaceId) return;
    const next: WorkspaceProfile = { ...profile, auth };
    await updateWorkspace(next);
    profile = next;
    if (!options?.skipRuntimeRestart && mcpStatus === "running") {
      try {
        await restartRuntime(workspaceId);
      } catch (error) {
        showToast(String(error), { title: "服务重启失败", kind: "error", duration: 8000 });
      }
    }
  }

  async function saveActionsAuth(draft: ActionsAuthDraft) {
    if (!profile || !workspaceId) return;
    const current = actionsConfig(profile);
    const next: WorkspaceProfile = {
      ...profile,
      actions: {
        ...current,
        auth_type: draft.authType,
        oauth_client_id: draft.oauthClientId || current.oauth_client_id,
        oauth_scopes: draft.oauthScopes,
        use_shared_secrets: draft.useSharedSecrets,
      },
    };
    await updateWorkspace(next);
    profile = next;
    if (actionsStatus === "running") {
      try {
        await restartActionsRuntime(workspaceId);
      } catch (error) {
        showToast(String(error), { title: "服务重启失败", kind: "error", duration: 8000 });
      }
    }
  }

  async function saveWorkspaceName(name: string) {
    if (!profile || profile.name === name) return;
    const next: WorkspaceProfile = { ...profile, name };
    await updateWorkspace(next);
    profile = next;
    workspaces.update((items) =>
      items.map((item) => (item.id === next.id ? { ...item, name: next.name } : item)),
    );
  }

  async function saveWorkspacePath(path: string) {
    if (!profile || profile.path === path) return;
    const next: WorkspaceProfile = { ...profile, path };
    await updateWorkspace(next);
    profile = next;
    showToast("工作区目录已更新", { kind: "success" });
    await promptServiceRestart(mcpStatus === "running", "MCP 服务");
    await promptServiceRestart(actionsStatus === "running", "Actions 服务");
  }

  async function removeWorkspace() {
    if (!profile || !workspaceId) return;
    const confirmed = await confirm(`确定删除工作区「${profile.name}」？此操作不可撤销。`, {
      title: "删除工作区",
      kind: "warning",
      okLabel: "删除",
      cancelLabel: "取消",
    });
    if (!confirmed) return;
    await deleteWorkspace(workspaceId);
    workspaces.update((items) => items.filter((item) => item.id !== workspaceId));
    mcpRuntimeStates.update((states) => {
      const next = { ...states };
      delete next[workspaceId];
      return next;
    });
    actionsRuntimeStates.update((states) => {
      const next = { ...states };
      delete next[workspaceId];
      return next;
    });
    goto("/");
  }

  $effect(() => {
    const id = workspaceId;
    if (!id) return;
    profile = null;
    void load(id);

    return () => {
      loadGeneration += 1;
    };
  });
</script>

{#if profile && actions}
  {@const isRunning = activeService === "mcp" ? mcpStatus === "running" : actionsStatus === "running"}
  {@const currentPublic = activeService === "mcp" ? mcpPublic : actionsPublic}
  {@const currentPort = activeService === "mcp" ? profile.runtime.local_port : actions.local_port}
  {@const currentTunnelType = activeService === "mcp" ? profile.tunnel.type : actions.tunnel_type}

  <section class="page-scroll flex flex-col h-full">
    <!-- 顶部状态与快速自救栏 -->
    <header class="page-header border-b border-[var(--color-border)] bg-[var(--color-bg)]/80 backdrop-blur px-6 py-4">
      <div class="flex flex-wrap items-center justify-between gap-4">
        <!-- 左侧：工作区与运行指示 -->
        <div class="flex items-center gap-3 min-w-0">
          <span class="h-3 w-3 rounded-full shrink-0 {isRunning ? 'bg-emerald-400 shadow-[0_0_10px_rgba(52,211,153,0.9)]' : 'bg-gray-400'}"></span>
          <div class="min-w-0">
            <div class="flex items-center gap-2">
              <h2 class="text-lg font-bold text-[var(--color-text)] truncate">{profile.name}</h2>
              <span class="rounded px-2 py-0.5 text-[11px] font-mono font-medium {isRunning ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400' : 'bg-gray-500/10 text-gray-500'}">
                {activeService.toUpperCase()} {stateLabel(activeService === "mcp" ? mcpStatus : actionsStatus)}
              </span>
            </div>
            <p class="text-xs text-[var(--color-text-muted)] font-mono truncate max-w-md mt-0.5" title={profile.path}>
              📁 {profile.path}
            </p>
          </div>
        </div>

        <!-- 右侧：高频自救与连接操作组 -->
        <div class="flex flex-wrap items-center gap-2">
          <!-- 服务模式切换 -->
          <div class="flex items-center rounded-lg border border-[var(--color-border)] bg-black/5 dark:bg-white/5 p-0.5 text-xs mr-1">
            <button
              type="button"
              class="rounded px-2.5 py-1 font-medium transition-colors {activeService === 'mcp' ? 'bg-white dark:bg-gray-800 shadow-sm text-black dark:text-white' : 'text-[var(--color-text-muted)] hover:text-black dark:hover:text-white'}"
              onclick={() => (activeService = "mcp")}
            >
              MCP 服务
            </button>
            <button
              type="button"
              class="rounded px-2.5 py-1 font-medium transition-colors {activeService === 'actions' ? 'bg-white dark:bg-gray-800 shadow-sm text-black dark:text-white' : 'text-[var(--color-text-muted)] hover:text-black dark:hover:text-white'}"
              onclick={() => (activeService = "actions")}
            >
              Actions 服务
            </button>
          </div>

          <!-- 高频操作 1：一键重启隧道 -->
          <button
            type="button"
            class="flex items-center gap-1.5 rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-1.5 text-xs font-semibold text-amber-700 dark:text-amber-400 hover:bg-amber-500/20 disabled:opacity-50 transition-colors shadow-sm"
            disabled={restartingTunnel}
            onclick={handleQuickRestartTunnel}
            title="隧道异常、连接超时或掉线时快速重新打通公网隧道"
          >
            <span>{restartingTunnel ? "重启中…" : "⚡ 重启隧道"}</span>
          </button>

          <!-- 启停服务 -->
          {#if activeService === "mcp"}
            <button
              type="button"
              class="rounded-lg px-3.5 py-1.5 text-xs font-medium text-white shadow-sm transition-all {mcpStatus === 'running' ? 'bg-red-600 hover:bg-red-700' : 'bg-emerald-600 hover:bg-emerald-700'}"
              disabled={mcpBusy}
              onclick={toggleMcp}
            >
              {mcpBusy ? "处理中…" : mcpStatus === "running" ? "停止服务" : "启动服务"}
            </button>
          {:else}
            <button
              type="button"
              class="rounded-lg px-3.5 py-1.5 text-xs font-medium text-white shadow-sm transition-all {actionsStatus === 'running' ? 'bg-red-600 hover:bg-red-700' : 'bg-emerald-600 hover:bg-emerald-700'}"
              disabled={actionsBusy}
              onclick={toggleActions}
            >
              {actionsBusy ? "处理中…" : actionsStatus === "running" ? "停止服务" : "启动服务"}
            </button>
          {/if}

          <!-- 低频操作：连接指南弹窗 -->
          <button
            type="button"
            class="rounded-lg border border-[var(--color-border)] bg-[var(--color-surface)] px-3 py-1.5 text-xs font-medium hover:border-[var(--color-accent)] transition-colors shadow-sm"
            onclick={() => (showGptModal = true)}
          >
            📋 连接到 GPT
          </button>

          <!-- 高级配置抽屉 -->
          <button
            type="button"
            class="rounded-lg border border-[var(--color-border)] px-3 py-1.5 text-xs font-medium hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
            onclick={() => (showSettingsModal = true)}
            title="隧道参数、认证模式与目录设置"
          >
            ⚙ 设置
          </button>
        </div>
      </div>

      <!-- 紧凑公网状态条 -->
      <div class="mt-3.5 flex flex-wrap items-center justify-between gap-3 rounded-lg border border-[var(--color-border)] bg-black/[0.02] dark:bg-white/[0.02] px-3.5 py-2 text-xs">
        <div class="flex items-center gap-2 min-w-0">
          <span class="text-[var(--color-text-muted)] font-medium shrink-0">公网端点:</span>
          {#if currentPublic}
            <span class="font-mono text-[var(--color-accent)] font-semibold truncate select-all">{currentPublic}</span>
            <button
              type="button"
              class="rounded border border-[var(--color-border)] px-2 py-0.5 text-[11px] hover:bg-black/5 dark:hover:bg-white/10 shrink-0"
              onclick={() => void copyEndpoint(currentPublic)}
            >
              复制
            </button>
          {:else}
            <span class="text-[var(--color-text-muted)] italic">未分配公网端点（启动服务或检查隧道设置）</span>
          {/if}
        </div>

        <div class="flex items-center gap-3 text-[var(--color-text-muted)] shrink-0 font-mono text-[11px]">
          <span>本地端口: {currentPort}</span>
          <span>隧道类型: {currentTunnelType.toUpperCase()}</span>
          <span>认证: {profile.auth.type.toUpperCase()}</span>
        </div>
      </div>
    </header>

    <!-- 核心主体监控视窗（页面主角） -->
    <div class="page-body flex-1 p-6">
      {#if activeService === "mcp"}
        <LogViewer workspaceId={workspaceId!} service="mcp" />
      {:else}
        <LogViewer workspaceId={workspaceId!} service="actions" />
      {/if}
    </div>

    <!-- 弹窗 1：GPT 连接向导（初次配完后不再占屏幕） -->
    {#if showGptModal}
      <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4 backdrop-blur-sm">
        <div class="w-full max-w-2xl rounded-2xl border border-[var(--color-border)] bg-[var(--color-surface)] p-6 shadow-2xl max-h-[90vh] overflow-y-auto">
          <div class="flex items-center justify-between border-b border-[var(--color-border)] pb-3">
            <div>
              <h3 class="text-base font-bold">连接到 ChatGPT / Agent 向导</h3>
              <p class="text-xs text-[var(--color-text-muted)] mt-0.5">复制公网地址与认证凭据，在 GPT 平台或 Custom GPT 中填入。</p>
            </div>
            <button
              type="button"
              class="rounded-lg p-1 text-gray-400 hover:text-black dark:hover:text-white"
              onclick={() => (showGptModal = false)}
            >
              ✕
            </button>
          </div>

          <div class="mt-4">
            <GptQuickCopy
              workspaceId={workspaceId!}
              service={activeService}
              {profile}
              publicMcpEndpoint={mcpPublic}
              {frpProfiles}
            />
          </div>

          <div class="mt-6 flex justify-end border-t border-[var(--color-border)] pt-3">
            <button
              type="button"
              class="rounded-lg bg-[var(--color-accent)] px-4 py-2 text-xs font-semibold text-white shadow-sm"
              onclick={() => (showGptModal = false)}
            >
              完成并关闭
            </button>
          </div>
        </div>
      </div>
    {/if}

    <!-- 弹窗 2：高级设置（隧道、认证、本地目录、策略） -->
    {#if showSettingsModal}
      <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4 backdrop-blur-sm">
        <div class="w-full max-w-3xl rounded-2xl border border-[var(--color-border)] bg-[var(--color-surface)] p-6 shadow-2xl max-h-[90vh] overflow-y-auto">
          <div class="flex items-center justify-between border-b border-[var(--color-border)] pb-3">
            <div>
              <h3 class="text-base font-bold">工作区与隧道高级配置</h3>
              <p class="text-xs text-[var(--color-text-muted)] mt-0.5">修改本地端口、FRP/Cloudflare 隧道参数、认证模式及代码执行策略。</p>
            </div>
            <button
              type="button"
              class="rounded-lg p-1 text-gray-400 hover:text-black dark:hover:text-white"
              onclick={() => (showSettingsModal = false)}
            >
              ✕
            </button>
          </div>

          <div class="mt-4 grid gap-6">
            <!-- 绑定目录 -->
            <div class="rounded-xl border border-[var(--color-border)] p-4 bg-black/[0.02] dark:bg-white/[0.02]">
              <p class="text-xs font-semibold text-[var(--color-text-muted)] mb-2">本地工作目录绑定</p>
              <WorkspaceMetaForm
                name={profile.name}
                path={profile.path}
                onSave={saveWorkspaceName}
                onUpdatePath={saveWorkspacePath}
              />
            </div>

            {#if activeService === "mcp"}
              <div class="rounded-xl border border-[var(--color-border)] p-4">
                <p class="text-xs font-semibold text-[var(--color-text-muted)] mb-3">MCP 隧道配置</p>
                <TunnelConfigForm
                  workspaceId={workspaceId!}
                  service="mcp"
                  config={mcpTunnelForm}
                  onSave={saveMcpTunnel}
                />
              </div>

              <div class="rounded-xl border border-[var(--color-border)] p-4">
                <p class="text-xs font-semibold text-[var(--color-text-muted)] mb-3">认证方式</p>
                <AuthConfigForm
                  workspaceId={workspaceId!}
                  auth={profile.auth}
                  onSaveProfile={saveMcpAuth}
                />
              </div>

              <div class="rounded-xl border border-[var(--color-border)] p-4">
                <p class="text-xs font-semibold text-[var(--color-text-muted)] mb-3">策略与权限</p>
                <RuntimePolicyForm
                  toolProfile={profile.runtime.tool_profile}
                  permissionMode={profile.runtime.permission_mode}
                  allowedCommands={profile.runtime.allowed_commands ?? ""}
                  workspaceLocalEntries={profile.runtime.workspace_local_entries ?? true}
                  workspaceScriptExtensions={profile.runtime.workspace_script_extensions ?? ".exe,.bat,.cmd,.ps1"}
                  onSave={saveMcpPolicy}
                />
              </div>
            {:else}
              <div class="rounded-xl border border-[var(--color-border)] p-4">
                <p class="text-xs font-semibold text-[var(--color-text-muted)] mb-3">Actions 隧道配置</p>
                <TunnelConfigForm
                  workspaceId={workspaceId!}
                  service="actions"
                  config={actionsTunnelForm}
                  onSave={saveActionsTunnel}
                />
              </div>

              <div class="rounded-xl border border-[var(--color-border)] p-4">
                <p class="text-xs font-semibold text-[var(--color-text-muted)] mb-3">Actions 认证</p>
                <ActionsAuthForm
                  workspaceId={workspaceId!}
                  authType={actions.auth_type}
                  oauthClientId={actions.oauth_client_id ?? ""}
                  oauthScopes={actions.oauth_scopes ?? ""}
                  openapiUrl={actionsOpenApiUrl(profile, frpProfiles)}
                  privacyUrl={actionsPrivacyUrl(profile, frpProfiles)}
                  oauthAuthorizeUrl={actionsOAuthAuthorizeUrl(profile, frpProfiles)}
                  oauthTokenUrl={actionsOAuthTokenUrl(profile, frpProfiles)}
                  useSharedSecrets={actions.use_shared_secrets ?? false}
                  onSave={saveActionsAuth}
                />
              </div>

              <div class="rounded-xl border border-[var(--color-border)] p-4">
                <p class="text-xs font-semibold text-[var(--color-text-muted)] mb-3">Actions 策略</p>
                <ActionsPolicyForm
                  allowedCommands={actions.allowed_commands ?? ""}
                  maxPatchBytes={actions.max_patch_bytes ?? 200_000}
                  permissionMode={actions.permission_mode}
                  onSave={saveActionsPolicy}
                />
              </div>
            {/if}

            <!-- 危险区：删除工作区 -->
            <div class="flex items-center justify-between border-t border-red-500/20 pt-4 mt-2">
              <div>
                <p class="text-xs font-semibold text-red-500">删除此工作区</p>
                <p class="text-[11px] text-[var(--color-text-muted)]">移除该工作区的端口、隧道配置与运行状态（不影响本地源码文件）。</p>
              </div>
              <button
                type="button"
                class="rounded-lg border border-red-500/30 px-3 py-1.5 text-xs font-medium text-red-500 hover:bg-red-500/10 transition-colors"
                onclick={() => void removeWorkspace()}
              >
                删除工作区
              </button>
            </div>
          </div>

          <div class="mt-6 flex justify-end border-t border-[var(--color-border)] pt-3">
            <button
              type="button"
              class="rounded-lg bg-[var(--color-accent)] px-4 py-2 text-xs font-semibold text-white shadow-sm"
              onclick={() => (showSettingsModal = false)}
            >
              关闭
            </button>
          </div>
        </div>
      </div>
    {/if}
  </section>
{/if}
