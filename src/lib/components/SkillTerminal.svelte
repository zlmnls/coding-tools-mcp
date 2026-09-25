<script lang="ts">
  import { runSkillTerminal, type TerminalResult } from "$lib/api/skills";
  let { skillId } = $props<{ skillId: string }>();
  let command = $state("");
  let output = $state<TerminalResult | null>(null);
  let running = $state(false);
  async function execute() { if (!command.trim()) return; running = true; try { output = await runSkillTerminal(skillId, command); } finally { running = false; } }
</script>
<div class="grid gap-3">
  <div class="rounded-md border border-[var(--color-border)] bg-[var(--color-bg)] p-3 font-mono text-xs whitespace-pre-wrap min-h-48">{#if output}{output.stdout}{output.stderr}{#if output.exit_code !== 0}\n[exit {output.exit_code}]{/if}{:else}终端绑定到 Skill 独立沙箱工作目录。{/if}</div>
  <form class="flex gap-2" onsubmit={(event) => { event.preventDefault(); void execute(); }}>
    <input class="tx-input tx-mono flex-1" bind:value={command} placeholder="输入沙箱内命令，例如 npm --version" />
    <button class="rounded-md bg-[var(--color-accent)] px-3 py-1.5 text-sm text-white" disabled={running}>执行</button>
  </form>
</div>
