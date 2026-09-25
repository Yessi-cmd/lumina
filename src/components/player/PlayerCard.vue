<script setup lang="ts">
import { computed, shallowRef, watch } from "vue";
import { useRouter } from "vue-router";
import {
  api,
  PANEL_HISTORY_COUNT,
  type GameResult,
  type GameSummary,
  type RosterPlayer,
  type Summoner,
} from "../../api";
import { useGameDataStore } from "../../stores/gameData";
import { summarize } from "../../utils/stats";

const props = defineProps<{ player: RosterPlayer }>();
const gd = useGameDataStore();
const router = useRouter();

const summoner = shallowRef<Summoner | null>(null);
const games = shallowRef<GameSummary[] | null>(null);
const error = shallowRef<string | null>(null);

const POSITIONS: Record<string, string> = {
  TOP: "上单",
  JUNGLE: "打野",
  MIDDLE: "中单",
  BOTTOM: "下路",
  UTILITY: "辅助",
};
const DOT: Record<GameResult, string> = {
  win: "bg-emerald-500",
  loss: "bg-red-500",
  remake: "bg-zinc-500",
  abort: "bg-zinc-600",
};

async function load(puuid: string) {
  summoner.value = null;
  games.value = null;
  error.value = null;
  api
    .summonerByPuuid(puuid)
    .then((s) => {
      if (puuid === props.player.puuid) summoner.value = s;
    })
    .catch(() => {});
  try {
    // The backend prefetched this exact page when the player appeared, so it is usually cached.
    const page = await api.matchHistory(puuid, 0, PANEL_HISTORY_COUNT);
    if (puuid === props.player.puuid) games.value = page.games;
  } catch (err) {
    if (puuid === props.player.puuid) error.value = String(err);
  }
}

watch(() => props.player.puuid, load, { immediate: true });

const stats = computed(() => (games.value ? summarize(games.value) : null));
const name = computed(() => {
  const s = summoner.value;
  if (!s) return "…";
  return s.gameName ? `${s.gameName}#${s.tagLine}` : s.displayName || "…";
});
const winRateClass = computed(() => {
  const rate = stats.value?.winRate ?? 0;
  if (rate >= 0.6) return "text-emerald-400";
  if (rate < 0.45) return "text-red-400";
  return "text-zinc-200";
});

function openHistory() {
  router.push({ path: "/match-history", query: { puuid: props.player.puuid } });
}
</script>

<template>
  <button
    class="flex w-full items-center gap-3 rounded-lg border bg-zinc-900 p-3 text-left hover:bg-zinc-800"
    :class="player.isSelf ? 'border-amber-500/60' : 'border-zinc-800'"
    @click="openHistory"
  >
    <div class="relative shrink-0">
      <img
        v-if="player.championId > 0"
        :src="gd.championIcon(player.championId)"
        :title="gd.championName(player.championId)"
        class="size-12 rounded-md bg-zinc-800"
      />
      <div v-else class="size-12 rounded-md bg-zinc-800" />
      <span
        v-if="POSITIONS[player.position]"
        class="absolute -bottom-1 left-1/2 -translate-x-1/2 rounded bg-zinc-950 px-1 text-[10px] text-zinc-300"
      >
        {{ POSITIONS[player.position] }}
      </span>
    </div>

    <div class="min-w-0 flex-1">
      <div class="flex items-baseline gap-2">
        <span class="truncate font-medium">{{ name }}</span>
        <span v-if="summoner" class="shrink-0 text-xs text-zinc-500">
          Lv.{{ summoner.summonerLevel }}
        </span>
      </div>

      <p v-if="error" class="truncate text-xs text-red-400" :title="error">{{ error }}</p>
      <p v-else-if="!stats" class="text-xs text-zinc-500">加载战绩…</p>
      <p v-else-if="stats.counted === 0" class="text-xs text-zinc-500">近期没有对局</p>
      <template v-else>
        <div class="mt-0.5 flex items-center gap-3 text-xs">
          <span :class="winRateClass">
            胜率 {{ Math.round(stats.winRate * 100) }}%
            <span class="text-zinc-500">({{ stats.wins }}/{{ stats.counted }})</span>
          </span>
          <span class="text-zinc-400">
            {{ stats.avgKills.toFixed(1) }} / {{ stats.avgDeaths.toFixed(1) }} /
            {{ stats.avgAssists.toFixed(1) }}
          </span>
        </div>
        <div class="mt-1.5 flex items-center gap-3">
          <div class="flex gap-0.5">
            <span
              v-for="(r, i) in stats.recent"
              :key="i"
              class="size-2 rounded-full"
              :class="DOT[r]"
            />
          </div>
          <div class="flex gap-1">
            <img
              v-for="c in stats.topChampions"
              :key="c.championId"
              :src="gd.championIcon(c.championId)"
              :title="`${gd.championName(c.championId)} ${c.wins}胜${c.games - c.wins}负`"
              class="size-5 rounded bg-zinc-800"
            />
          </div>
        </div>
      </template>
    </div>
  </button>
</template>
