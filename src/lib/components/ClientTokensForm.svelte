<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { confirm, message } from "@tauri-apps/plugin-dialog";
  import CopyButton from "$lib/components/CopyButton.svelte";

  type TokenInfo = {
    id: string;
    name: string;
    scope: "read_only" | "full";
    allowed_tools: string[];
    created_at: number;
    expires_at: number | null;
    revoked_at: number | null;
  };
  type NewToken = { info: TokenInfo; token: string };

  let { workspaceId, endpoint = "", enabled = false }: { workspaceId: string; endpoint?: string; enabled?: boolean } = $props();
  let items = $state<TokenInfo[]>([]);
  let name = $state("");
  let scope = $state<"read_only" | "full">("read_only");
  let expiresIn = $state("90");
  let allowedTools = $state("");
  let freshlyCreated = $state("");
  let busy = $state(false);
  let error = $state("");
  let loadSeq = 0;

  $effect(() => {
    const id = workspaceId;
    void load(id);
  });

  async function load(id: string) {
    const seq = ++loadSeq;
    try {
      const tokens = await invoke<TokenInfo[]>("list_client_tokens", { workspaceId: id });
      if (id !== workspaceId || seq !== loadSeq) return;
      items = tokens.filter((token) => token.revoked_at === null);
    } catch (e) {
      if (id === workspaceId && seq === loadSeq) error = String(e);
    }
  }

  async function create() {
    if (busy || !name.trim() || !enabled) return;
    busy = true;
    error = "";
    freshlyCreated = "";
    try {
      const created = await invoke<NewToken>("create_client_token", {
        workspaceId,
        name: name.trim(),
        scope,
        expiresInDays: expiresIn === "never" ? null : Number(expiresIn),
        allowedTools: allowedTools.split(",").map(s => s.trim()).filter(Boolean),
      });
      freshlyCreated = created.token;
      name = "";
      allowedTools = "";
      await load(workspaceId);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function revoke(info: TokenInfo) {
    if (busy) return;
    const yes = await confirm(`撤销并删除「${info.name}」？该 Token 会立即失效，记录将从列表中删除，不能恢复。`, {
      title: "撤销并删除客户端",
      kind: "warning",
    });
    if (!yes) return;
    busy = true;
    try {
      await invoke("revoke_client_token", { workspaceId, tokenId: info.id });
      items = items.filter((item) => item.id !== info.id);
      await load(workspaceId);
    } catch (e) {
      await message(String(e), { title: "撤销失败", kind: "error" });
    } finally {
      busy = false;
    }
  }

  const readableDate = (seconds: number | null) =>
    seconds === null ? "长期有效" : new Date(seconds * 1000).toLocaleDateString();
</script>

<div class="grid gap-3 rounded-lg border border-[var(--color-border)] p-3">
  <div>
    <p class="text-sm font-semibold">独立客户端 Token</p>
    <p class="text-xs text-[var(--color-text-muted)] mt-1">
      为 Claude Code、Cursor、Codex 等可自定义请求头的 MCP 客户端分别签发。撤销实时生效，保留现有 ChatGPT OAuth。
    </p>
  </div>
  {#if !enabled}
    <p class="text-xs text-amber-500">请先点击下方「保存配置」启用混合认证，然后再创建客户端 Token。</p>
  {/if}

  <div class="grid gap-2">
    <label class="grid gap-1">
      <span class="text-xs text-[var(--color-text-muted)]">客户端名称</span>
      <input class="rounded-md border border-[var(--color-border)] bg-[var(--color-bg)] p-2 text-sm"
        bind:value={name} maxlength="64" placeholder="例如：我的 Claude Code" required />
    </label>
    <div class="grid grid-cols-2 gap-2">
      <label class="grid gap-1">
        <span class="text-xs text-[var(--color-text-muted)]">工具权限</span>
        <select bind:value={scope} class="rounded-md border border-[var(--color-border)] bg-[var(--color-bg)] p-2 text-sm">
          <option value="read_only">只读（推荐）</option>
          <option value="full">全部已开放工具</option>
        </select>
      </label>
      <label class="grid gap-1">
        <span class="text-xs text-[var(--color-text-muted)]">有效期</span>
        <select bind:value={expiresIn} class="rounded-md border border-[var(--color-border)] bg-[var(--color-bg)] p-2 text-sm">
          <option value="30">30 天</option>
          <option value="90">90 天</option>
          <option value="365">365 天</option>
          <option value="never">不过期</option>
        </select>
      </label>
    </div>
    <label class="grid gap-1">
      <span class="text-xs text-[var(--color-text-muted)]">限定工具（可选，逗号分隔）</span>
      <input class="rounded-md border border-[var(--color-border)] bg-[var(--color-bg)] p-2 text-sm"
        bind:value={allowedTools} placeholder="留空表示允许该权限范围内的全部工具" />
    </label>
    <button type="button" onclick={() => void create()} disabled={busy || !name.trim() || !enabled}
      class="rounded-md bg-[var(--color-accent)] px-3 py-2 text-sm font-medium text-white disabled:opacity-50">
      {busy ? "处理中…" : "生成独立 Token"}
    </button>
  </div>
  {#if freshlyCreated}
    <div class="rounded-lg border border-amber-500/50 p-3 grid gap-2">
      <p class="text-xs font-medium">请现在复制保存，此 Token 只显示一次，关闭后无法重新查看。</p>
      <div class="flex items-center gap-2">
        <input class="min-w-0 flex-1 rounded-md bg-[var(--color-bg)] p-2 font-mono text-xs" type="password"
          readonly value={freshlyCreated} />
        <CopyButton value={freshlyCreated} label="复制 Token" />
      </div>
      <button type="button" class="text-xs underline text-left" onclick={() => (freshlyCreated = "")}>我已保存，隐藏 Token</button>
    </div>
  {/if}
  {#if error}<p class="text-xs text-red-500">{error}</p>{/if}
  <div class="grid gap-2">
    <p class="text-xs font-medium">已授权客户端</p>
    {#each items as item (item.id)}
      <div class="flex items-center gap-2 border-t border-[var(--color-border)] pt-2 text-xs">
        <div class="flex-1 min-w-0">
          <p class="font-medium truncate">{item.name}</p>
          <p class="text-[var(--color-text-muted)]">
            {item.scope === "read_only" ? "只读" : "全部工具"} · 到期：{readableDate(item.expires_at)}
            {item.allowed_tools.length ? ` · 限定 ${item.allowed_tools.length} 个工具` : ""}
          </p>
        </div>
        <button type="button" class="text-red-500 underline" disabled={busy} onclick={() => void revoke(item)}>撤销</button>
      </div>
    {/each}
    {#if !items.length}<p class="text-xs text-[var(--color-text-muted)]">暂无已创建的客户端 Token。</p>{/if}
  </div>
  <div class="text-xs text-[var(--color-text-muted)] grid gap-1">
    <div class="flex items-center gap-2">
      <p class="min-w-0 flex-1 break-all">远程 MCP 地址：{endpoint || "使用工作区提供的公网 HTTPS /mcp 地址"}</p>
      {#if endpoint}<CopyButton value={endpoint} label="复制地址" />{/if}
    </div>
    <p>连接方式：HTTP 请求头 Authorization: Bearer [生成的 Token]</p>
    <p>“全部工具”仍受工作区及宿主机策略约束；只读 Token 不允许命令执行、Skill 执行和外部 MCP 调用。</p>
  </div>
</div>
