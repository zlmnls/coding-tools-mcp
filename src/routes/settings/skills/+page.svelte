<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { message } from "@tauri-apps/plugin-dialog";
  import { listSkills, registerSkill, unregisterSkill, skillInstallPlan, installSkillDependencies, setSkillExecution, discoverSkills, registerDiscoveredSkill, registerDiscoveredSkillsBatch, type SkillMetadata, type InstallPlan, type DiscoveredSkill, type SkillSourceScope } from "$lib/api/skills";
  import SkillTerminal from "$lib/components/SkillTerminal.svelte";
  import SkillInstallTerminal from "$lib/components/SkillInstallTerminal.svelte";

  let skills = $state<SkillMetadata[]>([]);
  let selected = $state<SkillMetadata | null>(null);
  let plan = $state<InstallPlan | null>(null);
  let loading = $state(false);
  let showInstall = $state(false);
  let showDiscovery = $state(false);
  let discovered = $state<DiscoveredSkill[]>([]);
  let sourceFilter = $state<SkillSourceScope | "all">("all");
  let discovering = $state(false);
  let selectedPaths = $state<string[]>([]);
  let batchOperating = $state(false);

  const visibleDiscovered = $derived(sourceFilter === "all" ? discovered : discovered.filter((item) => item.scope === sourceFilter));
  const visibleUnregistered = $derived(visibleDiscovered.filter((item) => !item.registered));
  const allSelected = $derived(visibleUnregistered.length > 0 && visibleUnregistered.every((item) => selectedPaths.includes(item.path)));
  const selectedCount = $derived(selectedPaths.filter((p) => visibleUnregistered.some((item) => item.path === p)).length);

  function toggleSelectAll() {
    const currentUnregPaths = visibleUnregistered.map((item) => item.path);
    if (allSelected) {
      selectedPaths = selectedPaths.filter((p) => !currentUnregPaths.includes(p));
    } else {
      const combined = new Set([...selectedPaths, ...currentUnregPaths]);
      selectedPaths = Array.from(combined);
    }
  }

  function toggleCandidate(path: string) {
    if (selectedPaths.includes(path)) {
      selectedPaths = selectedPaths.filter((p) => p !== path);
    } else {
      selectedPaths = [...selectedPaths, path];
    }
  }

  async function batchInherit(pathsToInherit?: string[]) {
    const targets = pathsToInherit ?? selectedPaths.filter((p) => visibleUnregistered.some((item) => item.path === p));
    if (targets.length === 0) return;
    batchOperating = true;
    try {
      await registerDiscoveredSkillsBatch(targets);
      selectedPaths = selectedPaths.filter((p) => !targets.includes(p));
      await scanSources();
      await refresh();
      await message(`已成功继承 ${targets.length} 个 Skill 到 MCP！`, { title: "批量导入成功", kind: "info" });
    } catch (error) {
      await message(String(error), { title: "批量继承失败", kind: "error" });
    } finally {
      batchOperating = false;
    }
  }

  async function scanSources() { discovering = true; try { discovered = await discoverSkills(); } catch (error) { await message(String(error), { title: "发现 Skill 失败", kind: "error" }); } finally { discovering = false; } }
  async function inheritSkill(candidate: DiscoveredSkill) { try { await registerDiscoveredSkill(candidate.path); await scanSources(); await refresh(); } catch (error) { await message(String(error), { title: "继承 Skill 失败", kind: "error" }); } }

  async function refresh() { skills = await listSkills(); if (selected) selected = skills.find((item) => item.id === selected?.id) ?? null; }
  async function addSkill() {
    const path = await open({ directory: true, multiple: false });
    if (!path || Array.isArray(path)) return;
    loading = true;
    try { selected = await registerSkill(path); await refresh(); } catch (error) { await message(String(error), { title: "注册 Skill 失败", kind: "error" }); } finally { loading = false; }
  }
  async function removeSkill(id: string) { await unregisterSkill(id); selected = null; plan = null; await refresh(); }
  async function showPlan(skill: SkillMetadata) { selected = skill; plan = await skillInstallPlan(skill.id); }
  async function install() { if (!selected) return; loading = true; try { await installSkillDependencies(selected.id); await refresh(); selected = skills.find((item) => item.id === selected?.id) ?? selected; plan = await skillInstallPlan(selected.id); } catch (error) { await message(String(error), { title: "依赖安装失败", kind: "error" }); } finally { loading = false; } }
  async function updateExecution(mode: "Local" | "ReadOnly" | "Sandbox", allowExecution: boolean) { if (!selected) return; loading = true; try { selected = await setSkillExecution(selected.id, mode, allowExecution); await refresh(); } catch (error) { await message(String(error), { title: "更新执行模式失败", kind: "error" }); } finally { loading = false; } }
  onMount(() => { void refresh(); });
</script>

<section class="page-scroll">
  <header class="page-header">
    <p class="page-kicker">能力扩展</p>
    <h2 class="page-title">Skill 管理</h2>
    <p class="mt-2 max-w-2xl text-sm text-[var(--color-text-muted)]">注册 Skill 到独立沙箱，按需加载内容，并在依赖页确认安装计划。</p>
  </header>
  <div class="page-body grid gap-6 xl:grid-cols-[minmax(0,1fr)_minmax(320px,0.8fr)]">
    <div class="grid gap-4">
      <div class="tx-card p-4">
        <div class="flex items-center justify-between gap-3">
          <h3 class="text-sm font-semibold">已注册 Skill</h3>
          <div class="flex gap-2">
            <button class="rounded-md border border-[var(--color-border)] px-3 py-1.5 text-sm font-medium" disabled={loading} onclick={() => void addSkill()}>注册目录</button>
            <button class="rounded-md border border-[var(--color-border)] px-3 py-1.5 text-sm font-medium" disabled={discovering} onclick={() => { showDiscovery = !showDiscovery; if (showDiscovery && discovered.length === 0) void scanSources(); }}>{showDiscovery ? "收起发现" : "发现本地 Skill"}</button>
            <button class="rounded-md bg-[var(--color-accent)] px-3 py-1.5 text-sm font-medium text-white" disabled={loading} onclick={() => showInstall = !showInstall}>{showInstall ? "收起安装" : "终端安装"}</button>
          </div>
        </div>
        <div class="mt-4 grid gap-2">
          {#if skills.length === 0}<p class="text-sm text-[var(--color-text-muted)]">暂未注册 Skill。</p>{/if}
          {#each skills as skill (skill.id)}
            <button class="grid gap-1 rounded-md border border-[var(--color-border)] p-3 text-left hover:border-[var(--color-accent)]" onclick={() => void showPlan(skill)}>
              <span class="flex items-center justify-between text-sm font-medium"><span>{skill.name}</span><span class="text-xs text-[var(--color-text-muted)]">{skill.availability}</span></span>
              <span class="text-xs text-[var(--color-text-muted)]">{skill.description}</span>
              <span class="text-xs text-[var(--color-text-muted)]">沙箱：独立 · 依赖：{skill.dependencies.length} · 资源：{skill.resources.length}</span>
            </button>
          {/each}
        </div>
      </div>
      {#if showDiscovery}
        <div class="tx-card p-4">
          <div class="flex items-center justify-between gap-3"><h3 class="text-sm font-semibold">发现本地 Skill</h3><button class="text-xs text-[var(--color-accent)]" disabled={discovering} onclick={() => void scanSources()}>重新扫描</button></div>
          <p class="mt-2 text-xs text-[var(--color-text-muted)]">扫描 Trae、Codex、Agents、Claude、Cursor 的用户级、系统级和当前项目目录。发现后只登记原始路径，不修改来源文件。</p>
          <div class="mt-3 flex flex-wrap gap-1">
            {#each [["all", "全部"], ["project", "项目级"], ["user", "用户级"], ["system", "系统级"], ["custom", "自定义"]] as filter}
              <button class="rounded-full border border-[var(--color-border)] px-2.5 py-1 text-xs" class:bg-[var(--color-accent)]={sourceFilter === filter[0]} class:text-white={sourceFilter === filter[0]} onclick={() => sourceFilter = filter[0] as SkillSourceScope | "all"}>{filter[1]}</button>
            {/each}
          </div>
          {#if visibleUnregistered.length > 0}
            <div class="mt-3 flex flex-wrap items-center justify-between gap-2 rounded-md border border-[var(--color-border)] bg-[var(--color-bg)]/60 px-3 py-2">
              <div class="flex items-center gap-2">
                <button
                  class="rounded-md border border-[var(--color-border)] px-2.5 py-1 text-xs font-medium"
                  disabled={batchOperating}
                  onclick={toggleSelectAll}
                >
                  {allSelected ? "取消全选" : `一键全选（${visibleUnregistered.length}）`}
                </button>
                <span class="text-xs text-[var(--color-text-muted)]">已选 {selectedCount} 个</span>
              </div>
              <button
                class="rounded-md bg-[var(--color-accent)] px-3 py-1.5 text-xs font-medium text-white disabled:opacity-50"
                disabled={selectedCount === 0 || batchOperating}
                onclick={() => void batchInherit()}
              >
                {batchOperating ? "批量继承中…" : `批量继承（${selectedCount}）`}
              </button>
            </div>
          {/if}
          <div class="mt-3 grid gap-2">
            {#if discovering}<p class="text-sm text-[var(--color-text-muted)]">正在扫描本地 Skill 来源…</p>{:else if visibleDiscovered.length === 0}<p class="text-sm text-[var(--color-text-muted)]">未发现可继承的 Skill。</p>{/if}
            {#each visibleDiscovered as candidate (candidate.path)}
              <div class="rounded-md border border-[var(--color-border)] p-3">
                <div class="flex items-start justify-between gap-3">
                  <div class="flex min-w-0 gap-2">
                    {#if !candidate.registered}
                      <input
                        class="mt-1 h-4 w-4 shrink-0 accent-[var(--color-accent)]"
                        type="checkbox"
                        aria-label={`选择 ${candidate.name}`}
                        checked={selectedPaths.includes(candidate.path)}
                        disabled={batchOperating}
                        onchange={() => toggleCandidate(candidate.path)}
                      />
                    {/if}
                    <div class="grid min-w-0 gap-1"><span class="text-sm font-medium">{candidate.name}</span><span class="text-xs text-[var(--color-text-muted)]">{candidate.source_label} · {candidate.scope}</span><span class="text-xs text-[var(--color-text-muted)] break-all">{candidate.path}</span><span class="text-xs text-[var(--color-text-muted)]">{candidate.description}</span>{#if candidate.conflict}<span class="text-xs text-amber-600">存在同名 Skill，优先级：{candidate.conflict_rank}</span>{/if}</div>
                  </div>
                  <button class="shrink-0 rounded-md px-2.5 py-1 text-xs" class:bg-[var(--color-accent)]={!candidate.registered} class:text-white={!candidate.registered} class:border={!candidate.registered} disabled={candidate.registered || batchOperating} onclick={() => void inheritSkill(candidate)}>{candidate.registered ? "已注册" : "继承到 MCP"}</button>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}
      {#if showInstall}
        <div class="tx-card p-4">
          <h3 class="text-sm font-semibold">通过终端安装 Skill</h3>
          <div class="mt-3"><SkillInstallTerminal onRegistered={() => void refresh()} /></div>
        </div>
      {/if}
    </div>
    <div class="tx-card p-4">
      {#if selected}
        <div class="flex items-center justify-between"><h3 class="text-sm font-semibold">{selected.name}</h3><button class="text-xs text-red-500" onclick={() => void removeSkill(selected!.id)}>解除注册</button></div>
        <p class="mt-2 text-xs text-[var(--color-text-muted)]">{selected.source_path}</p>
        <div class="mt-4 grid gap-2 text-xs"><div>状态：{selected.availability}</div><div>来源目录：{selected.source_path}</div><div>执行模式：{selected.execution_mode === "Local" ? "本机环境" : selected.execution_mode === "ReadOnly" ? "只读" : "独立沙箱"}</div><div>自动执行：{selected.allow_execution ? "已开启" : "已关闭"}</div><div>网络：{selected.sandbox.network ? "已授权" : "关闭"}</div></div>
        <div class="mt-4 flex flex-wrap gap-2">
          <button class="rounded-md border border-[var(--color-border)] px-3 py-1.5 text-xs" disabled={loading} onclick={() => void updateExecution("Local", true)}>本机自动执行</button>
          <button class="rounded-md border border-[var(--color-border)] px-3 py-1.5 text-xs" disabled={loading} onclick={() => void updateExecution("ReadOnly", false)}>只读识别</button>
          <button class="rounded-md border border-[var(--color-border)] px-3 py-1.5 text-xs" disabled={loading} onclick={() => void updateExecution("Sandbox", true)}>沙箱执行</button>
        </div>
        <h4 class="mt-5 text-sm font-semibold">依赖安装计划</h4>
        {#if plan}<div class="mt-2 grid gap-2">{#each plan.commands as command}<code class="rounded bg-[var(--color-bg)] p-2 text-xs">{command}</code>{/each}{#if plan.commands.length === 0}<p class="text-xs text-[var(--color-text-muted)]">没有可自动安装的依赖。系统依赖或交互式登录请使用沙箱终端。</p>{:else}<button class="rounded-md bg-[var(--color-accent)] px-3 py-1.5 text-sm text-white" disabled={loading} onclick={() => void install()}>安装依赖</button>{/if}</div>{/if}
        <h4 class="mt-5 text-sm font-semibold">沙箱终端</h4>
        <div class="mt-2"><SkillTerminal skillId={selected.id} /></div>
      {:else}<p class="text-sm text-[var(--color-text-muted)]">选择一个 Skill 查看沙箱和依赖。</p>{/if}
    </div>
  </div>
</section>
