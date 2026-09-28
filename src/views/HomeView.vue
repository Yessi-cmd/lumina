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

/** Icon badge colours of the info tiles, in order. */
const TILE_TONES = [
  "bg-amber-400/10 text-amber-300 ring-amber-400/20",
  "bg-sky-400/10 text-sky-300 ring-sky-400/20",
  "bg-emerald-400/10 text-emerald-300 ring-emerald-400/20",
  "bg-violet-400/10 text-violet-300 ring-violet-400/20",
];

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
  <section class="stagger flex max-w-4xl flex-col gap-5">
    <header class="page-header">
      <div class="eyebrow">Overview</div>
      <h1 class="page-title mt-1">概览</h1>
    </header>

    <div class="card overflow-hidden p-6">
      <!-- Hero glow and a faint diagonal sheen. -->
      <div
        class="pointer-events-none absolute -top-28 -right-20 size-72 rounded-full bg-[radial-gradient(closest-side,rgb(245_158_11/0.22),transparent)]"
      />
      <div
        class="pointer-events-none absolute -bottom-32 left-1/4 size-72 rounded-full bg-[radial-gradient(closest-side,rgb(56_189_248/0.1),transparent)]"
      />
      <div
        class="pointer-events-none absolute inset-0 bg-[linear-gradient(115deg,transparent_40%,rgb(255_255_255/0.03)_50%,transparent_60%)]"
      />
      <div class="relative flex items-center gap-5">
        <div class="relative shrink-0">
          <!-- Turning ring behind the avatar. -->
          <div
            v-if="s.summoner"
            class="animate-spin-slow absolute -inset-1 rounded-[20px] bg-[conic-gradient(from_0deg,rgb(252_211_77),rgb(249_115_22),rgb(56_189_248),rgb(252_211_77))] opacity-80 blur-[1px]"
          />
          <img
            v-if="s.summoner"
            :src="profileIconUrl(s.summoner.profileIconId)"
            class="relative size-[72px] rounded-2xl bg-zinc-800 ring-2 ring-zinc-950"
          />
          <div
            v-else
            class="flex size-[72px] items-center justify-center rounded-2xl bg-zinc-800 text-zinc-500 ring-1 ring-white/10"
          >
            <AppIcon name="user" :size="28" />
          </div>
          <span
            v-if="s.summoner"
            class="absolute -bottom-2.5 left-1/2 -translate-x-1/2 rounded-full border border-amber-300/40 bg-linear-to-b from-zinc-800 to-zinc-950 px-2 text-[11px] font-semibold text-amber-200 tabular-nums shadow-[0_4px_12px_-4px_rgb(245_158_11/0.6)]"
          >
            {{ s.summoner.summonerLevel }}
          </span>
        </div>
        <div class="min-w-0 flex-1">
          <div class="text-xs text-zinc-500">欢迎回来</div>
          <div
            class="mt-0.5 truncate bg-linear-to-r from-white to-zinc-300 bg-clip-text text-2xl font-semibold tracking-tight text-transparent"
          >
            {{ riotId }}
          </div>
          <div class="mt-2 flex items-center gap-2">
            <span
              class="inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs backdrop-blur"
              :class="status.pill"
            >
              <span class="relative flex size-1.5">
                <span
                  v-if="s.status === 'connected'"
                  class="absolute inline-flex size-full animate-ping rounded-full bg-emerald-400 opacity-70"
                />
                <span class="relative size-1.5 rounded-full" :class="status.dot" />
              </span>
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

    <div class="stagger grid grid-cols-2 gap-3 lg:grid-cols-4">
      <div
        v-for="(tile, i) in tiles"
        :key="tile.label"
        class="group card p-4 transition-[transform,border-color,box-shadow] duration-300 ease-out-expo hover:-translate-y-1 hover:shadow-[0_18px_40px_-20px_rgb(0_0_0/0.9),0_0_0_1px_rgb(255_255_255/0.04)]"
      >
        <div class="flex items-center gap-2.5 text-zinc-400">
          <span
            class="flex size-7 items-center justify-center rounded-lg ring-1 ring-inset transition-transform duration-500 ease-spring group-hover:scale-110 group-hover:-rotate-6"
            :class="TILE_TONES[i % TILE_TONES.length]"
          >
            <AppIcon :name="tile.icon" :size="15" />
          </span>
          <span class="text-xs">{{ tile.label }}</span>
        </div>
        <div class="mt-3 truncate text-base font-semibold text-zinc-50" :title="tile.value">
          {{ tile.value }}
        </div>
        <div v-if="tile.hint" class="mt-0.5 truncate text-xs text-zinc-500">{{ tile.hint }}</div>
      </div>
    </div>
  </section>
</template>
