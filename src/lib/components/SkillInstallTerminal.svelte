<script lang="ts">
  import { createSkillInstallSession, runSkillInstallTerminal, detectInstalledSkills, registerDetectedSkill, cancelSkillInstallSession, type InstallSession, type TerminalResult, type SkillCandidate } from "$lib/api/skills";

  let { onRegistered } = $props<{ onRegistered?: () => void }>();
  let session = $state<InstallSession | null>(null);
  let command = $state("");
  let output = $state<TerminalResult | null>(null);
  let running = $state(false);
  let candidates = $state<SkillCandidate[]>([]);
  let selectedCandidate = $state<string | null>(null);
  let detecting = $state(false);
  let log = $state<string[]>([]);

  async function startSession() {
    session = await createSkillInstallSession();
    output = null;
    candidates = [];
    selectedCandidate = null;
    log = [];
  }

  async function execute() {
    if (!session || !command.trim()) return;
    running = true;
    try {
      output = await runSkillInstallTerminal(session.session_id, command);
      log.push(`$ ${command}`);
      if (output.stdout) log.push(output.stdout);
      if (output.stderr) log.push(output.stderr);
      if (output.exit_code !== 0 && output.exit_code !== null) log.push(`[exit ${output.exit_code}]`);
      command = "";
    } finally { running = false; }
  }

  async function detect() {
    if (!session) return;
    detecting = true;
    try {
      candidates = await detectInstalledSkills(session.session_id);
      if (candidates.length === 1) selectedCandidate = candidates[0].path;
    } finally { detecting = false; }
  }

  async function register() {
    if (!session || !selectedCandidate) return;
    try {
      await registerDetectedSkill(session.session_id, selectedCandidate);
      session = null;
      candidates = [];
      selectedCandidate = null;
      output = null;
      log = [];
      onRegistered?.();
    } catch (error) { log.push(`注册失败: ${error}`); }
  }

  async function cancel() {
    if (!session) return;
    await cancelSkillInstallSession(session.session_id);
    session = null;
    candidates = [];
    selectedCandidate = null;
    output = null;
    log = [];
  }

  async function onKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" && !event.shiftKey) { event.preventDefault(); void execute(); }
  }
</script>

<div class="grid gap-3">
  {#if !session}
    <div class="grid gap-3">
      <p class="text-sm text-[var(--color-text-muted)]">创建一个临时安装沙箱，通过终端命令下载或安装 Skill，然后自动探测并注册。</p>
      <button class="rounded-md bg-[var(--color-accent)] px-3 py-1.5 text-sm font-medium text-white" onclick={() => void startSession()}>开始终端安装</button>
    </div>
  {:else}
    <div class="grid gap-3">
      <div class="flex items-center justify-between">
        <span class="text-xs text-[var(--color-text-muted)]">沙箱：{session.workspace}</span>
        <button class="text-xs text-red-500" onclick={() => void cancel()}>取消安装</button>
      </div>

      <div class="rounded-md border border-[var(--color-border)] bg-[var(--color-bg)] p-3 font-mono text-xs whitespace-pre-wrap min-h-48 max-h-64 overflow-y-auto">
        {#if log.length > 0}{log.join("\n")}{:else}在下方输入安装命令，例如 git clone 或 npx{/if}
      </div>

      <form class="flex gap-2" onsubmit={(event) => { event.preventDefault(); void execute(); }}>
        <input class="tx-input tx-mono flex-1" bind:value={command} onkeydown={onKeydown} placeholder="输入安装命令，例如 git clone https://..." />
        <button class="rounded-md bg-[var(--color-accent)] px-3 py-1.5 text-sm text-white" disabled={running}>执行</button>
      </form>

      <p class="text-xs text-amber-600">安装终端支持 curl、wget、管道脚本、私有命令和交互式登录。命令会在 Skill 临时沙箱中执行，但仍可能访问网络；执行前请确认命令来源。</p>
      <div class="flex gap-2">
        <button class="rounded-md border border-[var(--color-border)] px-3 py-1.5 text-sm" disabled={detecting} onclick={() => void detect()}>探测 Skill</button>
      </div>

      {#if candidates.length > 0}
        <div class="grid gap-2">
          <h4 class="text-sm font-semibold">检测到 {candidates.length} 个 Skill</h4>
          {#if candidates.length === 1}
            <div class="rounded-md border border-[var(--color-accent)] p-3 text-sm">
              <div class="font-medium">{candidates[0].name}</div>
              <div class="text-xs text-[var(--color-text-muted)]">{candidates[0].path}</div>
              <div class="text-xs text-[var(--color-text-muted)] mt-1">{candidates[0].description}</div>
            </div>
            <button class="rounded-md bg-[var(--color-accent)] px-3 py-1.5 text-sm text-white" onclick={() => void register()}>注册此 Skill</button>
          {:else}
            <div class="grid gap-1">
              {#each candidates as candidate (candidate.path)}
                <label class="flex items-start gap-2 rounded-md border border-[var(--color-border)] p-3 cursor-pointer hover:border-[var(--color-accent)]" class:border-accent={selectedCandidate === candidate.path}>
                  <input type="radio" name="candidate" value={candidate.path} bind:group={selectedCandidate} class="mt-1" />
                  <div class="grid gap-0.5">
                    <span class="text-sm font-medium">{candidate.name}</span>
                    <span class="text-xs text-[var(--color-text-muted)]">{candidate.path}</span>
                    <span class="text-xs text-[var(--color-text-muted)]">{candidate.description}</span>
                  </div>
                </label>
              {/each}
            </div>
            <button class="rounded-md bg-[var(--color-accent)] px-3 py-1.5 text-sm text-white" disabled={!selectedCandidate} onclick={() => void register()}>注册选中 Skill</button>
          {/if}
        </div>
      {:else if candidates.length === 0 && log.length > 0}
        <p class="text-xs text-[var(--color-text-muted)]">未发现 SKILL.md。可继续执行安装命令或手动选择目录。</p>
      {/if}
    </div>
  {/if}
</div>
