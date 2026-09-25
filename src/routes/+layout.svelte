<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import { open } from "@tauri-apps/plugin-dialog";
  import AppShell from "$lib/components/AppShell.svelte";
  import ToastHost from "$lib/components/ToastHost.svelte";
  import WorkspaceNavItem from "$lib/components/WorkspaceNavItem.svelte";
  import {
    createWorkspace,
    getActionsRuntimeStatus,
    getRuntimeStatus,
    listWorkspaces,
  } from "$lib/api/workspaces";
  import { getLastWorkspaceId } from "$lib/api/settings";
  import { actionsRuntimeStates, mcpRuntimeStates, workspaces } from "$lib/stores/app";
  import { showToast } from "$lib/stores/toast";
  import { startUiMemoryGuard } from "$lib/ui-memory-guard";
  import { startCloseGuard } from "$lib/close-guard";
  import CloseConfirmDialog from "$lib/components/CloseConfirmDialog.svelte";
  import type { RuntimeState } from "$lib/types";

  let { children } = $props();
  let closeConfirmOpen = $state(false);

  async function refreshWorkspaces() {
    const items = await listWorkspaces();
    workspaces.set(items);

    const mcpStates: Record<string, RuntimeState> = {};
    const actionsStates: Record<string, RuntimeState> = {};
    await Promise.all(
      items.map(async (item) => {
        try {
          const [mcp, actions] = await Promise.all([
            getRuntimeStatus(item.id),
            getActionsRuntimeStatus(item.id),
          ]);
          mcpStates[item.id] = mcp.state;
          actionsStates[item.id] = actions.state;
        } catch {
          mcpStates[item.id] = "stopped";
          actionsStates[item.id] = "stopped";
        }
      }),
    );
    mcpRuntimeStates.set(mcpStates);
    actionsRuntimeStates.set(actionsStates);
  }

  async function addWorkspace() {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (!selected || Array.isArray(selected)) return;
      const profile = await createWorkspace(selected);
      await refreshWorkspaces();
      goto(`/workspace/${profile.id}`);
    } catch (error) {
      showToast(String(error), {
        title: "添加工作区失败",
        kind: "error",
        duration: 8000,
      });
    }
  }

  function openWorkspace(id: string) {
    goto(`/workspace/${id}`);
  }

  function openFrpSettings() {
    goto("/settings/frp");
  }

  function openSoftwareSettings() {
    goto("/settings/software");
  }

  function openSkillsSettings() {
    goto("/settings/skills");
  }

  function openMcpsSettings() {
    goto("/settings/mcps");
  }

  function openGeneralSettings() {
    goto("/settings/general");
  }

  function openKeysSettings() {
    goto("/settings/keys");
  }

  let activeWorkspaceId = $state<string>("");

  onMount(() => {
    const stopGuard = startUiMemoryGuard();
    const stopClose = startCloseGuard(() => {
      closeConfirmOpen = true;
    });
    void (async () => {
      await refreshWorkspaces();
      const lastId = await getLastWorkspaceId();
      if (lastId && $workspaces.some((item) => item.id === lastId)) {
        activeWorkspaceId = lastId;
      } else if ($workspaces.length > 0) {
        activeWorkspaceId = $workspaces[0].id;
      }
      const path = $page.url.pathname;
      if (path === "/") {
        if (activeWorkspaceId) {
          goto(`/workspace/${activeWorkspaceId}`);
        } else {
          goto("/settings/mcps");
        }
      }
    })();
    return () => {
      stopGuard();
      stopClose();
    };
  });
</script>

<AppShell>
  {#snippet sidebar()}
    <div class="space-y-1">
      <div class="flex items-center justify-between px-2 pt-1 pb-0.5">
        <span class="tx-sidebar-section-label !m-0 !p-0">工作区管理</span>
        <button
          type="button"
          class="rounded px-1.5 py-0.5 text-[11px] font-medium text-indigo-400 hover:bg-white/5 hover:text-indigo-300 transition-colors"
          onclick={addWorkspace}
          title="选取本地文件夹添加新工作区"
        >
          + 添加
        </button>
      </div>

      <div class="space-y-0.5 max-h-40 overflow-y-auto pr-0.5">
        {#each $workspaces as ws (ws.id)}
          <button
            type="button"
            class="tx-settings-link flex items-center justify-between text-left {$page.url.pathname === `/workspace/${ws.id}` ? 'active' : ''}"
            onclick={() => {
              activeWorkspaceId = ws.id;
              openWorkspace(ws.id);
            }}
          >
            <div class="flex items-center gap-2 min-w-0">
              <span class="h-1.5 w-1.5 rounded-full shrink-0 {$mcpRuntimeStates[ws.id] === 'running' ? 'bg-emerald-400 shadow-[0_0_6px_rgba(52,211,153,0.8)]' : 'bg-gray-500'}"></span>
              <span class="truncate font-medium">{ws.name}</span>
            </div>
            <span class="text-[10px] opacity-60 font-mono shrink-0 ml-1.5">
              {$mcpRuntimeStates[ws.id] === 'running' ? '运行中' : '停止'}
            </span>
          </button>
        {/each}
      </div>

      <div class="pt-2">
        <p class="tx-sidebar-section-label">能力与扩展</p>
        <button
          type="button"
          class="tx-settings-link flex items-center gap-2.5 {$page.url.pathname === '/settings/mcps' ? 'active' : ''}"
          onclick={openMcpsSettings}
        >
          <span class="rounded bg-indigo-500/20 px-1.5 py-0.5 text-[10px] font-semibold text-indigo-300">MCP</span>
          <span>外部 MCP 注册</span>
        </button>
        <button
          type="button"
          class="tx-settings-link flex items-center gap-2.5 {$page.url.pathname === '/settings/skills' ? 'active' : ''}"
          onclick={openSkillsSettings}
        >
          <span class="rounded bg-sky-500/20 px-1.5 py-0.5 text-[10px] font-semibold text-sky-300">SKILL</span>
          <span>Skill 管理</span>
        </button>
      </div>

      <div class="pt-2">
        <p class="tx-sidebar-section-label">网络与密钥</p>
        <button
          type="button"
          class="tx-settings-link {$page.url.pathname === '/settings/keys' ? 'active' : ''}"
          onclick={openKeysSettings}
        >
          共享密钥
        </button>
        <button
          type="button"
          class="tx-settings-link {$page.url.pathname === '/settings/frp' ? 'active' : ''}"
          onclick={openFrpSettings}
        >
          FRP 隧道配置
        </button>
        <button
          type="button"
          class="tx-settings-link {$page.url.pathname === '/settings/software' ? 'active' : ''}"
          onclick={openSoftwareSettings}
        >
          隧道软件管理
        </button>
        <button
          type="button"
          class="tx-settings-link {$page.url.pathname === '/settings/general' ? 'active' : ''}"
          onclick={openGeneralSettings}
        >
          通用设置
        </button>
      </div>
    </div>
  {/snippet}

  {#snippet children()}
    {@render children()}
  {/snippet}
</AppShell>

<ToastHost />
<CloseConfirmDialog
  open={closeConfirmOpen}
  onCancel={() => {
    closeConfirmOpen = false;
  }}
/>
