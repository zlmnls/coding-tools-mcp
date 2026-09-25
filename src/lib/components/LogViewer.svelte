<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { readWorkspaceLogs, type LogChunk, type LogService } from "$lib/api/logs";
  import { showToast } from "$lib/stores/toast";

  interface Props {
    workspaceId: string;
    service: LogService;
    autoRefresh?: boolean;
    title?: string;
  }

  let { workspaceId, service, autoRefresh = true, title }: Props = $props();

  let chunks = $state<LogChunk[]>([]);
  let busy = $state(false);
  let error = $state("");
  let filter = $state<"all" | "tunnel" | "auth" | "requests">("all");
  let autoPolling = $state(true);
  let pollTimer: ReturnType<typeof setInterval> | null = null;

  const heading = $derived(title ?? (service === "mcp" ? "网关实时监控终端" : "Actions 监控终端"));

  const filteredChunks = $derived(
    chunks.filter((c) => {
      if (filter === "all") return true;
      if (filter === "tunnel") return c.name.includes("cloudflared") || c.name.includes("frpc");
      if (filter === "auth") return c.name === "mcp-auth.log";
      if (filter === "requests") return c.name === "mcp-requests.log";
      return true;
    })
  );

  function logMeta(name: string): { title: string; badge: string; badgeColor: string; desc: string } {
    if (name.includes("cloudflared")) {
      return {
        title: "Cloudflare 隧道边缘网络",
        badge: "TUNNEL",
        badgeColor: "bg-amber-500/15 text-amber-600 dark:text-amber-400 border-amber-500/20",
        desc: "排查 Cloudflare 边缘节点连通性、i/o timeout、隧道断开与重新注册",
      };
    }
    if (name.includes("frpc")) {
      return {
        title: "FRP 客户端网络日志",
        badge: "FRP",
        badgeColor: "bg-blue-500/15 text-blue-600 dark:text-blue-400 border-blue-500/20",
        desc: "排查 FRP 客户端穿透隧道运行状况",
      };
    }
    if (name === "mcp-auth.log") {
      return {
        title: "OAuth 鉴权与 401 诊断日志",
        badge: "AUTH",
        badgeColor: "bg-purple-500/15 text-purple-600 dark:text-purple-400 border-purple-500/20",
        desc: "记录每次 tools/call 的 token 指纹、鉴权状态、Host 与 Canonical 对齐检查",
      };
    }
    if (name === "mcp-requests.log") {
      return {
        title: "JSON-RPC 工具调用与执行流水",
        badge: "EXEC",
        badgeColor: "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 border-emerald-500/20",
        desc: "记录收到的 tools/call 请求、参数结构与子进程退出状态码",
      };
    }
    return { title: name, badge: "LOG", badgeColor: "bg-gray-500/15 text-gray-500 border-gray-500/20", desc: "" };
  }

  async function refresh(silent = false) {
    if ((busy && !silent) || !workspaceId) return;
    if (!silent) busy = true;
    error = "";
    try {
      chunks = await readWorkspaceLogs(workspaceId, service);
    } catch (err) {
      error = String(err);
      chunks = [];
    } finally {
      if (!silent) busy = false;
    }
  }

  async function copyContent(content: string, name: string) {
    try {
      await navigator.clipboard.writeText(content);
      showToast(`已复制 ${name} 日志内容`, { kind: "success" });
    } catch {
      showToast("复制失败", { kind: "error" });
    }
  }

  function togglePolling() {
    autoPolling = !autoPolling;
    if (autoPolling) {
      startPolling();
      showToast("已开启日志自动轮询 (每 3 秒)", { kind: "info" });
    } else {
      stopPolling();
      showToast("已暂停日志自动轮询", { kind: "info" });
    }
  }

  function startPolling() {
    stopPolling();
    pollTimer = setInterval(() => {
      void refresh(true);
    }, 3000);
  }

  function stopPolling() {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
  }

  onMount(() => {
    void refresh();
    if (autoPolling) {
      startPolling();
    }
  });

  onDestroy(() => {
    stopPolling();
  });
</script>

<div class="flex flex-col gap-4">
  <!-- 终端顶部控制栏 -->
  <div class="flex flex-wrap items-center justify-between gap-3 border-b border-[var(--color-border)] pb-3">
    <div class="flex items-center gap-2">
      <span class="text-sm font-semibold">{heading}</span>
      <span class="flex items-center gap-1.5 text-xs text-[var(--color-text-muted)] font-mono">
        <span class="inline-block h-2 w-2 rounded-full {autoPolling ? 'bg-emerald-400 animate-pulse' : 'bg-gray-400'}"></span>
        {autoPolling ? "自动跟踪" : "已暂停"}
      </span>
    </div>

    <!-- 过滤器与快捷操作 -->
    <div class="flex flex-wrap items-center gap-2">
      <div class="flex items-center rounded-lg border border-[var(--color-border)] bg-black/5 dark:bg-white/5 p-0.5 text-xs">
        <button
          type="button"
          class="rounded px-2.5 py-1 font-medium transition-colors {filter === 'all' ? 'bg-white dark:bg-gray-800 shadow-sm text-black dark:text-white' : 'text-[var(--color-text-muted)] hover:text-black dark:hover:text-white'}"
          onclick={() => (filter = "all")}
        >
          全部日志
        </button>
        <button
          type="button"
          class="rounded px-2.5 py-1 font-medium transition-colors {filter === 'tunnel' ? 'bg-white dark:bg-gray-800 shadow-sm text-black dark:text-white' : 'text-[var(--color-text-muted)] hover:text-black dark:hover:text-white'}"
          onclick={() => (filter = "tunnel")}
        >
          隧道网络 (CF)
        </button>
        <button
          type="button"
          class="rounded px-2.5 py-1 font-medium transition-colors {filter === 'auth' ? 'bg-white dark:bg-gray-800 shadow-sm text-black dark:text-white' : 'text-[var(--color-text-muted)] hover:text-black dark:hover:text-white'}"
          onclick={() => (filter = "auth")}
        >
          鉴权诊断 (401)
        </button>
        <button
          type="button"
          class="rounded px-2.5 py-1 font-medium transition-colors {filter === 'requests' ? 'bg-white dark:bg-gray-800 shadow-sm text-black dark:text-white' : 'text-[var(--color-text-muted)] hover:text-black dark:hover:text-white'}"
          onclick={() => (filter = "requests")}
        >
          请求调用
        </button>
      </div>

      <button
        type="button"
        class="rounded-lg border border-[var(--color-border)] px-2.5 py-1 text-xs text-[var(--color-text-muted)] hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
        onclick={togglePolling}
        title="开启或暂停自动轮询"
      >
        {autoPolling ? "暂停跟踪" : "自动跟踪"}
      </button>

      <button
        type="button"
        class="rounded-lg bg-[var(--color-accent)] px-3 py-1 text-xs font-medium text-white shadow-sm hover:opacity-90 disabled:opacity-50 transition-opacity"
        disabled={busy}
        onclick={() => void refresh(false)}
      >
        {busy ? "刷新中…" : "立即刷新"}
      </button>
    </div>
  </div>

  {#if error}
    <div class="rounded-lg border border-[var(--color-error)]/30 bg-[var(--color-error)]/10 px-3 py-2 text-xs text-[var(--color-error)]">
      日志加载失败: {error}
    </div>
  {/if}

  <!-- 日志卡片流 -->
  {#if filteredChunks.length > 0}
    <div class="grid gap-4">
      {#each filteredChunks as chunk (chunk.name)}
        {@const meta = logMeta(chunk.name)}
        <div class="overflow-hidden rounded-xl border border-[var(--color-border)] bg-[#0d1117] text-[#c9d1d9] shadow-md">
          <!-- 卡片头部 -->
          <div class="flex items-center justify-between border-b border-[#30363d] bg-[#161b22] px-4 py-2.5">
            <div class="flex items-center gap-2.5 min-w-0">
              <span class="rounded border px-1.5 py-0.5 text-[10px] font-mono font-bold tracking-wider {meta.badgeColor}">
                {meta.badge}
              </span>
              <span class="text-xs font-semibold text-white truncate">{meta.title}</span>
              <span class="hidden sm:inline-block text-[11px] text-[#8b949e] truncate font-mono">
                {chunk.name}
              </span>
            </div>
            <div class="flex items-center gap-2 shrink-0">
              <span class="text-[10px] text-[#8b949e]">最新 8KB</span>
              <button
                type="button"
                class="rounded border border-[#30363d] px-2 py-0.5 text-[11px] text-[#8b949e] hover:bg-white/10 hover:text-white transition-colors"
                onclick={() => void copyContent(chunk.content, chunk.name)}
              >
                复制
              </button>
            </div>
          </div>

          {#if meta.desc}
            <div class="border-b border-[#21262d] bg-[#0d1117]/80 px-4 py-1.5 text-[11px] text-[#8b949e]">
              {meta.desc}
            </div>
          {/if}

          <!-- 终端日志内容 -->
          <pre
            class="max-h-[380px] overflow-auto whitespace-pre-wrap break-words p-4 font-mono text-xs leading-relaxed text-[#58a6ff] selection:bg-[#1f6feb] selection:text-white"
          >{chunk.content || "（暂无日志输出，等待请求触发或连接器访问...）"}</pre>
        </div>
      {/each}
    </div>
  {:else if !busy && !error}
    <div class="rounded-xl border border-dashed border-[var(--color-border)] p-12 text-center text-sm text-[var(--color-text-muted)]">
      当前分类下暂无日志输出。可在上方切换至“全部日志”查看。
    </div>
  {/if}
</div>
