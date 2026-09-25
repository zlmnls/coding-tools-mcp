<script lang="ts">
  import { onMount } from "svelte";
  import { message } from "@tauri-apps/plugin-dialog";
  import {
    listExternalMcps,
    saveExternalMcp,
    deleteExternalMcp,
    setExternalMcpEnabled,
    testExternalMcp,
    discoverExternalMcps,
    type McpServerConfig,
    type McpToolSummary,
    type DiscoveredMcpServer,
  } from "$lib/api/mcps";

  let servers = $state<McpServerConfig[]>([]);
  let discovered = $state<DiscoveredMcpServer[]>([]);
  let loading = $state(false);
  let discovering = $state(false);
  let showForm = $state(false);
  let isEditing = $state(false);

  // 表单状态
  let formId = $state("");
  let formName = $state("");
  let formDescription = $state("");
  let formInstructions = $state("");
  let formUsageMode = $state<"always_on" | "on_demand">("on_demand");
  let formTransport = $state("stdio");
  let formCommand = $state("");
  let formArgsText = $state("");
  let formEnvText = $state("");
  let formEnabled = $state(true);

  // 测试与工具展示状态
  let testingId = $state<string | null>(null);
  let activeTools = $state<Record<string, McpToolSummary[]>>({});
  let testError = $state<Record<string, string>>({});

  async function refresh() {
    loading = true;
    try {
      servers = await listExternalMcps();
    } catch (error) {
      await message(String(error), { title: "加载外部 MCP 失败", kind: "error" });
    } finally {
      loading = false;
    }
  }

  async function scanDiscovered() {
    discovering = true;
    try {
      discovered = await discoverExternalMcps();
    } catch (error) {
      await message(String(error), { title: "发现本地 MCP 失败", kind: "error" });
    } finally {
      discovering = false;
    }
  }

  function openCreateForm() {
    isEditing = false;
    formId = "";
    formName = "";
    formDescription = "";
    formInstructions = "";
    formUsageMode = "on_demand";
    formTransport = "stdio";
    formCommand = "";
    formArgsText = "";
    formEnvText = "";
    formEnabled = true;
    showForm = true;
  }

  function openEditForm(server: McpServerConfig) {
    isEditing = true;
    formId = server.id;
    formName = server.name;
    formDescription = server.description;
    formInstructions = server.instructions || "";
    formUsageMode = server.usage_mode ?? "on_demand";
    formTransport = server.transport || "stdio";
    formCommand = server.command;
    formArgsText = (server.args || []).join("\n");
    formEnvText = Object.entries(server.env || {})
      .map(([k, v]) => `${k}=${v}`)
      .join("\n");
    formEnabled = server.enabled;
    showForm = true;
  }

  async function handleSave() {
    const id = formId.trim();
    const command = formCommand.trim();
    if (!id) {
      await message("请填写 MCP 标识 (ID)", { title: "提示", kind: "warning" });
      return;
    }
    if (!command) {
      await message("请填写执行命令 (Command)", { title: "提示", kind: "warning" });
      return;
    }

    const args = formArgsText
      .split("\n")
      .map((s) => s.trim())
      .filter((s) => s.length > 0);

    const env: Record<string, string> = {};
    for (const line of formEnvText.split("\n")) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith("#")) continue;
      const idx = trimmed.indexOf("=");
      if (idx > 0) {
        const k = trimmed.slice(0, idx).trim();
        const v = trimmed.slice(idx + 1).trim();
        if (k) env[k] = v;
      }
    }

    const config: McpServerConfig = {
      id,
      name: formName.trim() || id,
      description: formDescription.trim(),
      instructions: formInstructions.trim(),
      usage_mode: formUsageMode,
      transport: formTransport,
      command,
      args,
      env,
      enabled: formEnabled,
    };

    loading = true;
    try {
      await saveExternalMcp(config);
      showForm = false;
      await refresh();
      await scanDiscovered();
    } catch (error) {
      await message(String(error), { title: "保存失败", kind: "error" });
    } finally {
      loading = false;
    }
  }

  async function handleDelete(id: string) {
    if (!confirm(`确定要解除注册 MCP 服务 "${id}" 吗？`)) return;
    loading = true;
    try {
      await deleteExternalMcp(id);
      delete activeTools[id];
      delete testError[id];
      await refresh();
      await scanDiscovered();
    } catch (error) {
      await message(String(error), { title: "删除失败", kind: "error" });
    } finally {
      loading = false;
    }
  }

  async function handleToggleEnabled(server: McpServerConfig) {
    const next = !server.enabled;
    loading = true;
    try {
      await setExternalMcpEnabled(server.id, next);
      server.enabled = next;
      await refresh();
    } catch (error) {
      await message(String(error), { title: "更新状态失败", kind: "error" });
    } finally {
      loading = false;
    }
  }

  async function handleTest(server: McpServerConfig) {
    testingId = server.id;
    testError[server.id] = "";
    try {
      const tools = await testExternalMcp(server);
      activeTools[server.id] = tools;
      await message(`测试连接成功！已获取到 ${tools.length} 个工具。`, {
        title: "连接成功",
        kind: "info",
      });
    } catch (error) {
      testError[server.id] = String(error);
      await message(`测试连接失败:\n${error}`, { title: "连接失败", kind: "error" });
    } finally {
      testingId = null;
    }
  }

  async function handleImportDiscovered(item: DiscoveredMcpServer) {
    loading = true;
    try {
      await saveExternalMcp(item.config);
      await refresh();
      await scanDiscovered();
      await message(`已成功导入 "${item.config.name}"！`, {
        title: "导入成功",
        kind: "info",
      });
    } catch (error) {
      await message(String(error), { title: "导入失败", kind: "error" });
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void (async () => {
      await refresh();
      await scanDiscovered();
    })();
  });
</script>

<section class="page-scroll">
  <header class="page-header">
    <p class="page-kicker">能力扩展与聚合</p>
    <h2 class="page-title">外部 MCP 注册</h2>
    <p class="mt-2 max-w-2xl text-sm text-[var(--color-text-muted)]">
      将本地其他 MCP 服务（如 Mem0 长期记忆等）注册为扩展能力。GPT 可直接调用已暴露工具，也可通过 <code>mcp_match</code>、<code>mcp_list</code> 与 <code>mcp_call</code> 按需发现并执行。
    </p>
  </header>

  <div class="page-body grid gap-6">
    <!-- 顶部操作栏 -->
    <div class="flex items-center justify-between gap-4">
      <div class="flex items-center gap-2">
        <span class="text-sm font-medium">已注册服务：{servers.length} 个</span>
      </div>
      <div class="flex gap-2">
        <button
          class="rounded-md border border-[var(--color-border)] px-3 py-1.5 text-sm font-medium hover:border-[var(--color-accent)]"
          disabled={discovering}
          onclick={() => void scanDiscovered()}
        >
          {discovering ? "正在扫描…" : "扫描本地 MCP"}
        </button>
        <button
          class="rounded-md bg-[var(--color-accent)] px-3 py-1.5 text-sm font-medium text-white"
          onclick={openCreateForm}
        >
          + 手动注册 MCP
        </button>
      </div>
    </div>

    <!-- 自动发现面板 -->
    {#if discovered.length > 0}
      <div class="tx-card p-4">
        <div class="flex items-center justify-between">
          <h3 class="text-sm font-semibold">从本地客户端自动发现的 MCP 配置</h3>
          <span class="text-xs text-[var(--color-text-muted)]">支持一键导入</span>
        </div>
        <p class="mt-1 text-xs text-[var(--color-text-muted)]">
          检测到 Trae SOLO CN、Trae CN、Claude Desktop 中配置的外部 MCP。点击一键即可注册到本工具中。
        </p>
        <div class="mt-3 grid gap-2.5">
          {#each discovered as item (item.config.id + item.source)}
            <div
              class="flex items-start justify-between gap-3 rounded-md border border-[var(--color-border)] p-3"
            >
              <div class="grid gap-1">
                <div class="flex items-center gap-2">
                  <span class="text-sm font-semibold">{item.config.name}</span>
                  <span class="rounded bg-black/5 px-1.5 py-0.5 text-xs text-[var(--color-text-muted)]">
                    {item.source}
                  </span>
                  <span class="text-xs font-mono text-[var(--color-text-muted)]">
                    ID: {item.config.id}
                  </span>
                </div>
                <div class="text-xs font-mono text-[var(--color-text-muted)] break-all">
                  {item.config.command} {item.config.args.join(" ")}
                </div>
                <div class="text-xs text-[var(--color-text-muted)]">
                  来源：{item.source_file}
                </div>
              </div>
              <button
                class="shrink-0 rounded-md px-3 py-1 text-xs font-medium {!item.already_registered ? 'bg-[var(--color-accent)] text-white' : 'border border-[var(--color-border)] text-[var(--color-text-muted)]'}"
                disabled={item.already_registered || loading}
                onclick={() => void handleImportDiscovered(item)}
              >
                {item.already_registered ? "已注册" : "一键导入"}
              </button>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- 注册列表 -->
    <div class="grid gap-4">
      {#if servers.length === 0}
        <div class="tx-card p-6 text-center text-sm text-[var(--color-text-muted)]">
          暂未注册外部 MCP 服务。可以点击上方“扫描本地 MCP”一键导入 Mem0，或点击“+ 手动注册 MCP”配置。
        </div>
      {/if}

      {#each servers as server (server.id)}
        <div class="tx-card p-4">
          <div class="flex flex-wrap items-start justify-between gap-3">
            <div class="grid gap-1">
              <div class="flex items-center gap-2">
                <span class="text-base font-semibold">{server.name}</span>
                <span class="rounded bg-[var(--color-accent)]/10 px-2 py-0.5 text-xs font-mono text-[var(--color-accent)]">
                  ID: {server.id}
                </span>
                <span
                  class="rounded px-2 py-0.5 text-xs font-medium {server.enabled ? 'bg-emerald-50 text-emerald-600' : 'bg-gray-100 text-gray-500'}"
                >
                  {server.enabled ? "已启用 (GPT 可调用)" : "已停用"}
                </span>
                <span class="rounded bg-slate-100 px-2 py-0.5 text-xs font-medium text-slate-600">
                  {(server.usage_mode ?? "on_demand") === "always_on" ? "常驻能力" : "按需能力"}
                </span>
              </div>
              {#if server.description}
                <p class="text-xs text-[var(--color-text-muted)]">{server.description}</p>
              {/if}
              {#if server.instructions}
                <div class="mt-1 rounded bg-amber-500/5 p-2 text-xs border border-amber-500/20 text-amber-800">
                  <span class="font-semibold">已配置规范/Prompt:</span> 已配置 {server.instructions.length} 字符的专属使用规范，{(server.usage_mode ?? "on_demand") === "always_on" ? "完整规则会注入 GPT 握手上下文。" : "按需模式下不会在握手时注入完整规则。"}
                </div>
              {/if}
              <div class="mt-1 text-xs font-mono text-[var(--color-text-muted)] break-all">
                {server.command} {server.args.join(" ")}
              </div>
            </div>

            <!-- 操作按钮组 -->
            <div class="flex items-center gap-2">
              <button
                class="rounded-md border border-[var(--color-border)] px-2.5 py-1 text-xs hover:border-[var(--color-accent)]"
                disabled={testingId === server.id}
                onclick={() => void handleTest(server)}
              >
                {testingId === server.id ? "正在探测…" : "测试连接 / 拉取工具"}
              </button>
              <button
                class="rounded-md border border-[var(--color-border)] px-2.5 py-1 text-xs hover:border-[var(--color-accent)]"
                onclick={() => handleToggleEnabled(server)}
              >
                {server.enabled ? "停用" : "启用"}
              </button>
              <button
                class="rounded-md border border-[var(--color-border)] px-2.5 py-1 text-xs hover:border-[var(--color-accent)]"
                onclick={() => openEditForm(server)}
              >
                编辑
              </button>
              <button
                class="rounded-md border border-red-200 px-2.5 py-1 text-xs text-red-500 hover:bg-red-50"
                onclick={() => void handleDelete(server.id)}
              >
                解除注册
              </button>
            </div>
          </div>

          <!-- 工具展示列表 -->
          {#if activeTools[server.id] && activeTools[server.id].length > 0}
            <div class="mt-4 border-t border-[var(--color-border)] pt-3">
              <div class="text-xs font-semibold text-[var(--color-text-muted)]">
                暴露的工具列表 ({activeTools[server.id].length} 个):
              </div>
              <div class="mt-2 grid gap-2 sm:grid-cols-2">
                {#each activeTools[server.id] as tool}
                  <div class="rounded border border-[var(--color-border)] p-2 text-xs">
                    <div class="font-mono font-medium text-[var(--color-accent)]">
                      {tool.exposed_name}
                    </div>
                    {#if tool.description}
                      <div class="mt-0.5 text-[var(--color-text-muted)] line-clamp-2">
                        {tool.description}
                      </div>
                    {/if}
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          {#if testError[server.id]}
            <div class="mt-3 rounded bg-red-50 p-2 text-xs text-red-600 font-mono">
              错误: {testError[server.id]}
            </div>
          {/if}
        </div>
      {/each}
    </div>

    <!-- 弹窗/编辑抽屉 -->
    {#if showForm}
      <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
        <div class="w-full max-w-lg rounded-xl bg-white p-6 shadow-2xl">
          <div class="flex items-center justify-between border-b border-[var(--color-border)] pb-3">
            <h3 class="text-base font-semibold">
              {isEditing ? "编辑外部 MCP 服务" : "注册外部 MCP 服务"}
            </h3>
            <button class="text-gray-400 hover:text-black" onclick={() => (showForm = false)}>✕</button>
          </div>

          <div class="mt-4 grid gap-3 text-sm">
            <div>
              <label class="block font-medium text-xs text-[var(--color-text-muted)]" for="mcp-id">
                服务唯一标识 (ID)
              </label>
              <input
                id="mcp-id"
                class="mt-1 w-full rounded-md border border-[var(--color-border)] p-2 font-mono text-sm"
                placeholder="例如: mem0"
                disabled={isEditing}
                bind:value={formId}
              />
            </div>

            <div>
              <label class="block font-medium text-xs text-[var(--color-text-muted)]" for="mcp-name">
                显示名称
              </label>
              <input
                id="mcp-name"
                class="mt-1 w-full rounded-md border border-[var(--color-border)] p-2 text-sm"
                placeholder="例如: Mem0 长期记忆"
                bind:value={formName}
              />
            </div>

            <div>
              <label class="block font-medium text-xs text-[var(--color-text-muted)]" for="mcp-desc">
                描述信息
              </label>
              <input
                id="mcp-desc"
                class="mt-1 w-full rounded-md border border-[var(--color-border)] p-2 text-sm"
                placeholder="提供长期记忆存储与语义搜索能力"
                bind:value={formDescription}
              />
            </div>

            <div>
              <label class="block font-medium text-xs text-[var(--color-text-muted)]" for="mcp-usage-mode">
                使用模式
              </label>
              <select
                id="mcp-usage-mode"
                class="mt-1 w-full rounded-md border border-[var(--color-border)] p-2 text-sm"
                bind:value={formUsageMode}
              >
                <option value="on_demand">按需能力（推荐）</option>
                <option value="always_on">常驻能力</option>
              </select>
              <p class="mt-0.5 text-[11px] text-[var(--color-text-muted)]">
                常驻能力会把完整 Instructions 注入 GPT 握手上下文，适合 Mem0 等几乎每轮都参与的基础能力；按需能力只暴露简短描述，需要时再由 GPT 匹配和调用。
              </p>
            </div>

            <div>
              <label class="block font-medium text-xs text-[var(--color-text-muted)]" for="mcp-instructions">
                使用规范与 Prompt 规则 (Instructions)
              </label>
              <p class="mt-0.5 text-[11px] text-[var(--color-text-muted)]">
                常驻模式会在 GPT 握手时完整注入；按需模式会保存规则但不在初始化阶段展开，避免普通 MCP 大量占用上下文。
              </p>
              <textarea
                id="mcp-instructions"
                rows="5"
                class="mt-1 w-full rounded-md border border-[var(--color-border)] p-2 font-mono text-xs"
                placeholder="例如：你已接入 Mem0 MCP 工具，必须按三层记忆模型主动调用..."
                bind:value={formInstructions}
              ></textarea>
            </div>

            <div>
              <label class="block font-medium text-xs text-[var(--color-text-muted)]" for="mcp-cmd">
                可执行命令 (Command)
              </label>
              <input
                id="mcp-cmd"
                class="mt-1 w-full rounded-md border border-[var(--color-border)] p-2 font-mono text-sm"
                placeholder="例如: docker 或 npx 或 /path/to/binary"
                bind:value={formCommand}
              />
            </div>

            <div>
              <label class="block font-medium text-xs text-[var(--color-text-muted)]" for="mcp-args">
                参数列表 (每行一个参数)
              </label>
              <textarea
                id="mcp-args"
                rows="4"
                class="mt-1 w-full rounded-md border border-[var(--color-border)] p-2 font-mono text-xs"
                placeholder="run&#10;--rm&#10;-i&#10;-v&#10;mem0-mcp_mem0_data:/app/data&#10;--env-file&#10;/path/to/.env&#10;mem0-mcp"
                bind:value={formArgsText}
              ></textarea>
            </div>

            <div>
              <label class="block font-medium text-xs text-[var(--color-text-muted)]" for="mcp-env">
                环境变量 (KEY=VALUE，每行一个)
              </label>
              <textarea
                id="mcp-env"
                rows="2"
                class="mt-1 w-full rounded-md border border-[var(--color-border)] p-2 font-mono text-xs"
                placeholder="API_KEY=xyz"
                bind:value={formEnvText}
              ></textarea>
            </div>

            <div class="flex items-center gap-2 pt-1">
              <input type="checkbox" id="mcp-enabled" bind:checked={formEnabled} />
              <label for="mcp-enabled" class="text-xs">注册后立即启用并暴露工具</label>
            </div>
          </div>

          <div class="mt-6 flex justify-end gap-2 border-t border-[var(--color-border)] pt-3">
            <button
              class="rounded-md border border-[var(--color-border)] px-4 py-1.5 text-sm"
              onclick={() => (showForm = false)}
            >
              取消
            </button>
            <button
              class="rounded-md bg-[var(--color-accent)] px-4 py-1.5 text-sm font-medium text-white"
              disabled={loading}
              onclick={handleSave}
            >
              保存
            </button>
          </div>
        </div>
      </div>
    {/if}
  </div>
</section>
