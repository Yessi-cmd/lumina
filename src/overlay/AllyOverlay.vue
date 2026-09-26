<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, shallowRef, watch } from "vue";
import { api, events, type RankedChampions, type Roster } from "../api";
import { useGameDataStore } from "../stores/gameData";
import { ROW_HEIGHT, rowTop } from "./layout";

interface Row {
  key: string;
  floor: number;
  puuid: string | null;
  state: "anonymous" | "loading" | "error" | "ready";
  summary: RankedChampions | null;
}

const gd = useGameDataStore();
const roster = shallowRef<Roster | null>(null);
const names = reactive(new Map<string, string>());
const ranked = reactive(new Map<string, RankedChampions | "loading" | "error">());
let unlisten: (() => void) | undefined;

onMounted(async () => {
  let gotEvent = false;
  unlisten = await events.onRoster((r) => {
    gotEvent = true;
    roster.value = r;
  });
  const current = await api.ongoingRoster();
  if (!gotEvent) roster.value = current;
});
onUnmounted(() => unlisten?.());

/** Teammates in pick order; anonymous ones the client did not reveal come last. */
const rows = computed<Row[]>(() => {
  const r = roster.value;
  if (!r || r.stage !== "champSelect") return [];
  const known = r.allies.map((p, i): Row => {
    const loaded = ranked.get(p.puuid) ?? "loading";
    const summary = typeof loaded === "object" ? loaded : null;
    const state = summary ? "ready" : (loaded as "loading" | "error");
    return { key: p.puuid, floor: i + 1, puuid: p.puuid, state, summary };
  });
  const anonymous = r.anonymousAllies.map(
    (_, i): Row => ({
      key: `anonymous-${i}`,
      floor: known.length + i + 1,
      puuid: null,
      state: "anonymous",
      summary: null,
    }),
  );
  return [...known, ...anonymous].slice(0, 5);
});

watch(
  () => rows.value.map((r) => r.puuid).join(","),
  () => {
    for (const row of rows.value) {
      const puuid = row.puuid;
      if (!puuid || ranked.has(puuid)) continue;
      ranked.set(puuid, "loading");
      api
        .rankedChampions(puuid)
        .then((summary) => ranked.set(puuid, summary))
        .catch(() => ranked.set(puuid, "error"));
      api
        .summonerByPuuid(puuid)
        .then((s) => names.set(puuid, s.gameName || s.displayName))
        .catch(() => {});
    }
  },
  { immediate: true },
);

function winRate(wins: number, games: number): number {
  return games > 0 ? Math.round((wins / games) * 100) : 0;
}

function kda(k: number, d: number, a: number): string {
  return `${k.toFixed(1)}/${d.toFixed(1)}/${a.toFixed(1)}`;
}
</script>

<template>
  <TransitionGroup name="overlay-row" tag="div" class="relative h-full w-[220px]" appear>
    <div
      v-for="(row, i) in rows"
      :key="row.key"
      class="overlay-card"
      :style="{ top: `${rowTop(i)}px`, height: `${ROW_HEIGHT}px`, transitionDelay: `${i * 40}ms` }"
    >
      <div class="flex items-center gap-1 truncate leading-3 font-semibold text-amber-300">
        <span>{{ row.floor }} 楼</span>
        <span class="text-zinc-500">·</span>
        <span class="truncate">{{ row.puuid ? (names.get(row.puuid) ?? "…") : "匿名队友" }}</span>
      </div>

      <p v-if="row.state === 'anonymous'" class="mt-1.5 text-zinc-500">身份未公开，暂时查不到战绩。</p>
      <template v-else-if="row.state === 'loading'">
        <div v-for="n in 3" :key="n" class="skeleton rounded mt-1 h-3" :style="{ width: `${90 - n * 12}%` }" />
      </template>
      <p v-else-if="row.state === 'error'" class="mt-1.5 text-orange-300">战绩加载失败</p>
      <template v-else-if="row.summary">
        <div class="mt-px mb-0.5 truncate text-[8px] leading-[9px] text-zinc-400">
          近期单双排 · {{ row.summary.games }} 场 · 胜率 {{ winRate(row.summary.wins, row.summary.games) }}% · 均
          {{ kda(row.summary.avgKills, row.summary.avgDeaths, row.summary.avgAssists) }}
        </div>
        <p v-if="row.summary.champions.length === 0" class="mt-1 text-zinc-500">近期没有单双排对局</p>
        <div
          v-for="c in row.summary.champions.slice(0, 3)"
          :key="c.championId"
          class="mt-px flex h-3 items-center gap-1.5 text-[9px] leading-none"
        >
          <img :src="gd.championIcon(c.championId)" class="size-[13px] shrink-0 rounded-[3px] bg-zinc-800" />
          <span class="min-w-0 flex-1 truncate text-zinc-200">{{ gd.championName(c.championId) }}</span>
          <span class="text-zinc-400 tabular-nums">{{ c.games }} 场</span>
          <span
            class="w-8 text-right tabular-nums"
            :class="winRate(c.wins, c.games) >= 50 ? 'text-emerald-300' : 'text-red-300'"
          >
            {{ winRate(c.wins, c.games) }}%
          </span>
          <span class="w-[62px] text-right text-[8px] text-zinc-400 tabular-nums">
            {{ kda(c.avgKills, c.avgDeaths, c.avgAssists) }}
          </span>
        </div>
      </template>
    </div>
  </TransitionGroup>
</template>
