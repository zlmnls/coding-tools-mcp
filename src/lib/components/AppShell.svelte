<script lang="ts">
  import ThemeToggle from "$lib/components/ThemeToggle.svelte";
  import { APP_VERSION } from "$lib/app-version";
  import type { Snippet } from "svelte";

  interface Props {
    children: Snippet;
    sidebar: Snippet;
    onAddWorkspace?: () => void | Promise<void>;
    settingsNav?: Snippet;
  }

  let { children, sidebar, onAddWorkspace, settingsNav }: Props = $props();
</script>

<div class="app-layout">
  <aside class="tx-sidebar">
    <div class="tx-sidebar-header">
      <div class="flex items-start justify-between gap-2">
        <div class="flex items-center gap-2.5">
          <img src="/favicon.png" alt="Logo" class="h-8 w-8 rounded-lg shadow-sm" />
          <div>
            <p class="tx-brand-kicker">Gateway</p>
            <h1 class="tx-brand-title">Mcp Tools</h1>
          </div>
        </div>
        <ThemeToggle />
      </div>
    </div>

    <div class="tx-sidebar-body">
      {@render sidebar()}
    </div>

    <div class="tx-sidebar-footer">
      {#if settingsNav}
        {@render settingsNav()}
      {/if}
      <div class="tx-app-meta">
        <p class="tx-app-version">v{APP_VERSION}</p>
        <span class="text-[10px] text-emerald-400/80 font-mono tracking-wide">● Ready</span>
      </div>
    </div>
  </aside>

  <main class="tx-main">
    {@render children()}
  </main>
</div>

<svelte:head>
  <title>MCP-Gateway</title>
</svelte:head>
