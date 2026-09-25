<script setup lang="ts">
import { computed, ref } from "vue";
import { api, type ConnectionStatus } from "../api";
import { phaseLabel, useLcuStore } from "../stores/lcu";

const lcu = useLcuStore();
const s = computed(() => lcu.snapshot);

const STATUS: Record<ConnectionStatus, { text: string; dot: string }> = {
  connected: { text: "已连接", dot: "bg-emerald-400" },
  connecting: { text: "连接中", dot: "animate-pulse bg-amber-400" },
  disconnected: { text: "未检测到客户端", dot: "bg-zinc-500" },
};
const status = computed(() => STATUS[s.value.status]);

const riotId = computed(() => {
  const summoner = s.value.summoner;
  if (!summoner) return "-";
  if (summoner.gameName) return `${summoner.gameName}#${summoner.tagLine}`;
  return summoner.displayName || "-";
});

const relaunchError = ref<string | null>(null);
async function relaunchAsAdmin() {
  relaunchError.value = null;
  try {
    await api.relaunchAsAdmin();
  } catch (err) {
    relaunchError.value = String(err);
  }
}

const sourceLabel = computed(() =>
  s.value.client?.source === "lockfile" ? "lockfile" : "进程命令行",
);
</script>

<template>
  <section class="flex max-w-3xl flex-col gap-4">
    <h1 class="text-xl font-semibold">概览</h1>

    <div class="rounded-lg border border-zinc-800 bg-zinc-900 p-4">
      <div class="flex items-center gap-2">
        <span class="size-2.5 rounded-full" :class="status.dot" />
        <span class="font-medium">{{ status.text }}</span>
      </div>
      <dl v-if="s.client" class="mt-3 grid grid-cols-[6rem_1fr] gap-y-1 text-sm">
        <dt class="text-zinc-500">服务器</dt>
        <dd>{{ s.client.platformId ?? "-" }}</dd>
        <dt class="text-zinc-500">进程</dt>
        <dd>PID {{ s.client.pid }} · 端口 {{ s.client.port }}</dd>
        <dt class="text-zinc-500">凭据来源</dt>
        <dd>{{ sourceLabel }}</dd>
      </dl>
      <p v-else-if="s.status === 'disconnected'" class="mt-2 text-sm text-zinc-400">
        启动英雄联盟客户端后会自动连接。
      </p>
      <p v-if="s.lastError" class="mt-3 text-sm break-all text-red-400">{{ s.lastError }}</p>
      <button
        v-if="s.needsAdmin"
        class="mt-3 rounded-md bg-amber-500 px-3 py-1.5 text-sm font-medium text-zinc-950 hover:bg-amber-400"
        @click="relaunchAsAdmin"
      >
        以管理员身份重启
      </button>
      <p v-if="relaunchError" class="mt-2 text-sm text-red-400">{{ relaunchError }}</p>
    </div>

    <div class="grid grid-cols-2 gap-4">
      <div class="rounded-lg border border-zinc-800 bg-zinc-900 p-4">
        <div class="text-xs text-zinc-500">当前召唤师</div>
        <div class="mt-1 truncate text-lg font-medium">{{ riotId }}</div>
        <div v-if="s.summoner" class="text-sm text-zinc-400">等级 {{ s.summoner.summonerLevel }}</div>
      </div>
      <div class="rounded-lg border border-zinc-800 bg-zinc-900 p-4">
        <div class="text-xs text-zinc-500">游戏阶段</div>
        <div class="mt-1 text-lg font-medium">
          {{ lcu.connected ? phaseLabel(s.gameflowPhase) : "-" }}
        </div>
        <div v-if="lcu.connected" class="text-sm text-zinc-400">{{ s.gameflowPhase }}</div>
      </div>
    </div>
  </section>
</template>
