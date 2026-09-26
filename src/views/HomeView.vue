<script setup lang="ts">
import { computed, ref } from "vue";
import { api, type ConnectionStatus } from "../api";
import AppIcon, { type IconName } from "../components/common/AppIcon.vue";
import { profileIconUrl } from "../stores/gameData";
import { phaseLabel, useLcuStore } from "../stores/lcu";

const lcu = useLcuStore();
const s = computed(() => lcu.snapshot);

const STATUS: Record<ConnectionStatus, { text: string; dot: string; pill: string }> = {
  connected: {
    text: "已连接",
    dot: "bg-emerald-400",
    pill: "border-emerald-400/25 bg-emerald-400/10 text-emerald-300",
  },
  connecting: {
    text: "连接中",
    dot: "animate-pulse bg-amber-400",
    pill: "border-amber-400/25 bg-amber-400/10 text-amber-300",
  },
  disconnected: {
    text: "未检测到客户端",
    dot: "bg-zinc-500",
    pill: "border-white/10 bg-white/5 text-zinc-400",
  },
};
const status = computed(() => STATUS[s.value.status]);

const riotId = computed(() => {
  const summoner = s.value.summoner;
  if (!summoner) return "未登录";
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

const tiles = computed(() => {
  const client = s.value.client;
  const list: { label: string; icon: IconName; value: string; hint?: string }[] = [
    { label: "服务器", icon: "server", value: client?.platformId ?? "-" },
    {
      label: "战绩数据",
      icon: "database",
      value: client ? (client.sgpServer ? "SGP" : "仅 LCU") : "-",
      hint: client?.sgpServer ?? undefined,
    },
    {
      label: "客户端进程",
      icon: "activity",
      value: client ? `PID ${client.pid}` : "-",
      hint: client ? `端口 ${client.port}` : undefined,
    },
    {
      label: "凭据来源",
      icon: "key",
      value: client ? (client.source === "lockfile" ? "lockfile" : "进程命令行") : "-",
    },
  ];
  return list;
});
</script>

<template>
  <section class="flex max-w-4xl flex-col gap-5">
    <header>
      <div class="eyebrow">Overview</div>
      <h1 class="page-title mt-1">概览</h1>
    </header>

    <div class="card relative overflow-hidden p-5">
      <div
        class="pointer-events-none absolute -top-24 -right-16 size-64 rounded-full bg-amber-500/10 blur-3xl"
      />
      <div class="relative flex items-center gap-4">
        <div class="relative shrink-0">
          <img
            v-if="s.summoner"
            :src="profileIconUrl(s.summoner.profileIconId)"
            class="size-16 rounded-2xl bg-zinc-800 ring-2 ring-amber-400/40"
          />
          <div
            v-else
            class="flex size-16 items-center justify-center rounded-2xl bg-zinc-800 text-zinc-500 ring-1 ring-white/10"
          >
            <AppIcon name="user" :size="26" />
          </div>
          <span
            v-if="s.summoner"
            class="absolute -bottom-2 left-1/2 -translate-x-1/2 rounded-full border border-white/10 bg-zinc-900 px-1.5 text-[11px] text-zinc-300 tabular-nums"
          >
            {{ s.summoner.summonerLevel }}
          </span>
        </div>
        <div class="min-w-0 flex-1">
          <div class="truncate text-xl font-semibold tracking-tight text-zinc-50">{{ riotId }}</div>
          <div class="mt-1.5 flex items-center gap-2">
            <span
              class="inline-flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-xs"
              :class="status.pill"
            >
              <span class="size-1.5 rounded-full" :class="status.dot" />
              {{ status.text }}
            </span>
            <span v-if="lcu.connected" class="text-xs text-zinc-500">{{ phaseLabel(s.gameflowPhase) }}</span>
          </div>
        </div>
        <button v-if="s.needsAdmin" class="btn btn-primary" @click="relaunchAsAdmin">
          <AppIcon name="shield" :size="15" />
          以管理员身份重启
        </button>
      </div>

      <p v-if="s.status === 'disconnected' && !s.client" class="relative mt-4 text-sm text-zinc-400">
        启动英雄联盟客户端后会自动连接。
      </p>
      <p v-if="s.lastError" class="relative mt-4 text-sm break-all text-red-400">{{ s.lastError }}</p>
      <p v-if="relaunchError" class="relative mt-2 text-sm text-red-400">{{ relaunchError }}</p>
    </div>

    <div class="grid grid-cols-2 gap-3 lg:grid-cols-4">
      <div v-for="tile in tiles" :key="tile.label" class="card p-4">
        <div class="flex items-center gap-2 text-zinc-500">
          <AppIcon :name="tile.icon" :size="15" />
          <span class="text-xs">{{ tile.label }}</span>
        </div>
        <div class="mt-2 truncate text-[15px] font-medium text-zinc-100" :title="tile.value">
          {{ tile.value }}
        </div>
        <div v-if="tile.hint" class="mt-0.5 truncate text-xs text-zinc-500">{{ tile.hint }}</div>
      </div>
    </div>
  </section>
</template>
