<script setup lang="ts">
import { computed, ref, shallowRef, watch } from "vue";
import { useRoute } from "vue-router";
import { api, PANEL_HISTORY_COUNT, type DataSource, type GameSummary, type Summoner } from "../api";
import CareerPanel from "../components/career/CareerPanel.vue";
import AppIcon from "../components/common/AppIcon.vue";
import MatchRow from "../components/match/MatchRow.vue";
import { profileIconUrl, useGameDataStore } from "../stores/gameData";
import { useLcuStore } from "../stores/lcu";
import { kdaRatio } from "../utils/format";

// Same size as the game panel, so opening a player from there hits the cache.
const PAGE_SIZE = PANEL_HISTORY_COUNT;
/** Queues offered in the mode filter; SGP filters them on the server. */
const QUEUES: { id: number; label: string }[] = [
  { id: 420, label: "单双排" },
  { id: 440, label: "灵活排位" },
  { id: 490, label: "快速匹配" },
  { id: 400, label: "匹配（征召）" },
  { id: 430, label: "匹配（自选）" },
  { id: 450, label: "极地大乱斗" },
  { id: 1700, label: "斗魂竞技场" },
];

const lcu = useLcuStore();
const gd = useGameDataStore();
const route = useRoute();

/** Game list or career analysis of the shown player. */
const view = ref<"games" | "career">("games");
const queueFilter = ref<number | null>(null);
const championFilter = ref<number | null>(null);

const query = ref("");
const summoner = shallowRef<Summoner | null>(null);
const games = shallowRef<GameSummary[]>([]);
const source = ref<DataSource | null>(null);
const sgpError = ref<string | null>(null);
const hasMore = ref(false);
const loading = ref(false);
const error = ref<string | null>(null);
// Guards against a slow response for a previous player overwriting the current one.
let generation = 0;

/** Champions in the loaded games, most played first. */
const championOptions = computed(() => {
  const counts = new Map<number, number>();
  for (const g of games.value) counts.set(g.championId, (counts.get(g.championId) ?? 0) + 1);
  const options = [...counts].map(([id, n]) => ({ id, n, name: gd.championName(id) }));
  return options.sort((a, b) => b.n - a.n || a.name.localeCompare(b.name, "zh-CN"));
});

const shown = computed(() => {
  const champion = championFilter.value;
  return champion === null ? games.value : games.value.filter((g) => g.championId === champion);
});

/** Win rate and KDA of the shown games; remakes do not count. */
const summary = computed(() => {
  const counted = shown.value.filter((g) => g.result === "win" || g.result === "loss");
  if (counted.length === 0) return null;
  const wins = counted.filter((g) => g.result === "win").length;
  const sum = (f: (g: GameSummary) => number) => counted.reduce((acc, g) => acc + f(g), 0);
  return {
    games: counted.length,
    wins,
    winRate: Math.round((wins / counted.length) * 100),
    kda: kdaRatio(sum((g) => g.kills), sum((g) => g.deaths), sum((g) => g.assists)),
  };
});

const riotId = computed(() => {
  const s = summoner.value;
  if (!s) return "";
  return s.gameName ? `${s.gameName}#${s.tagLine}` : s.displayName;
});

async function show(target: Summoner) {
  const gen = ++generation;
  summoner.value = target;
  games.value = [];
  source.value = null;
  sgpError.value = null;
  hasMore.value = false;
  await fetchPage(gen);
}

/** A new mode means a new list from the server; the champion filter stays. */
function onQueueChange() {
  const target = summoner.value;
  if (target) show(target);
}

function clearFilters() {
  championFilter.value = null;
  if (queueFilter.value === null) return;
  queueFilter.value = null;
  onQueueChange();
}

function loadMore() {
  if (!loading.value) fetchPage(generation);
}

async function fetchPage(gen: number) {
  const target = summoner.value;
  if (!target) return;
  loading.value = true;
  error.value = null;
  try {
    const page = await api.matchHistory(target.puuid, games.value.length, PAGE_SIZE, queueFilter.value);
    if (gen !== generation) return;
    games.value = [...games.value, ...page.games];
    source.value = page.source;
    sgpError.value = page.sgpError;
    hasMore.value = page.games.length > 0;
  } catch (err) {
    if (gen === generation) error.value = String(err);
  } finally {
    if (gen === generation) loading.value = false;
  }
}

async function search() {
  const text = query.value.trim();
  if (!text) return;
  error.value = null;
  try {
    await show(await api.lookupSummoner(text));
  } catch (err) {
    error.value = String(err);
  }
}

async function showPuuid(puuid: string) {
  try {
    await show(await api.summonerByPuuid(puuid));
  } catch (err) {
    error.value = String(err);
  }
}

function showSelf() {
  const self = lcu.snapshot.summoner;
  if (self) show(self);
}

// `/match-history?puuid=...` opens a specific player (used by the game panels later);
// otherwise default to the logged-in summoner once connected.
watch(
  () => [lcu.connected, route.query.puuid] as const,
  ([connected, puuid]) => {
    if (!connected) return;
    if (typeof puuid === "string" && puuid) showPuuid(puuid);
    else if (!summoner.value) showSelf();
  },
  { immediate: true },
);
</script>

<template>
  <section class="flex max-w-6xl flex-col gap-4">
    <header class="flex items-end gap-4">
      <div>
        <div class="eyebrow">Match history</div>
        <h1 class="page-title mt-1">战绩</h1>
      </div>
      <form class="ml-auto flex gap-2" @submit.prevent="search">
        <div class="relative">
          <AppIcon
            name="search"
            :size="15"
            class="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-zinc-500"
          />
          <input v-model="query" placeholder="名字#标签" class="field w-64 pl-9" />
        </div>
        <button type="submit" :disabled="!lcu.connected" class="btn btn-primary">查询</button>
        <button
          type="button"
          :disabled="!lcu.snapshot.summoner"
          class="btn btn-secondary"
          @click="showSelf"
        >
          <AppIcon name="user" :size="15" />
          我自己
        </button>
      </form>
    </header>

    <p v-if="!lcu.connected" class="empty-state">连接英雄联盟客户端后才能查询战绩。</p>

    <div v-if="summoner" class="card relative overflow-hidden p-4">
      <div class="pointer-events-none absolute -top-20 -left-10 size-56 rounded-full bg-amber-500/10 blur-3xl" />
      <div class="relative flex items-center gap-4">
        <div class="relative shrink-0">
          <img
            :src="profileIconUrl(summoner.profileIconId)"
            class="size-14 rounded-2xl bg-zinc-800 ring-2 ring-amber-400/30"
          />
          <span
            class="absolute -bottom-2 left-1/2 -translate-x-1/2 rounded-full border border-white/10 bg-zinc-900 px-1.5 text-[11px] text-zinc-300 tabular-nums"
          >
            {{ summoner.summonerLevel }}
          </span>
        </div>
        <div class="min-w-0">
          <div class="truncate text-lg font-semibold tracking-tight text-zinc-50 select-text">{{ riotId }}</div>
          <div class="mt-1 flex items-center gap-2 text-xs">
            <span
              v-if="source"
              class="rounded-full border px-2 py-0.5"
              :class="
                source === 'sgp'
                  ? 'border-emerald-400/25 bg-emerald-400/10 text-emerald-300'
                  : 'border-white/10 bg-white/5 text-zinc-400'
              "
            >
              数据来源 {{ source.toUpperCase() }}
            </span>
            <span v-if="sgpError" class="max-w-xs truncate text-zinc-500" :title="sgpError">
              SGP 不可用：{{ sgpError }}
            </span>
          </div>
        </div>

        <div v-if="summary" class="ml-auto flex gap-6 pr-2">
          <div class="text-right">
            <div class="eyebrow">场次</div>
            <div class="mt-0.5 text-lg font-semibold text-zinc-100 tabular-nums">
              {{ summary.games }}
              <span class="text-xs font-normal text-zinc-500">{{ summary.wins }} 胜</span>
            </div>
          </div>
          <div class="text-right">
            <div class="eyebrow">胜率</div>
            <div
              class="mt-0.5 text-lg font-semibold tabular-nums"
              :class="summary.winRate >= 50 ? 'text-emerald-400' : 'text-red-400'"
            >
              {{ summary.winRate }}%
            </div>
          </div>
          <div class="text-right">
            <div class="eyebrow">KDA</div>
            <div class="mt-0.5 text-lg font-semibold text-zinc-100 tabular-nums">{{ summary.kda }}</div>
          </div>
        </div>
      </div>
    </div>

    <div v-if="summoner" class="segmented self-start">
      <button class="segment px-4 py-1.5 text-sm" :class="view === 'games' && 'segment-active'" @click="view = 'games'">
        对局记录
      </button>
      <button class="segment px-4 py-1.5 text-sm" :class="view === 'career' && 'segment-active'" @click="view = 'career'">
        生涯分析
      </button>
    </div>

    <Transition name="fade" mode="out-in">
    <CareerPanel v-if="summoner && view === 'career'" :key="summoner.puuid" :puuid="summoner.puuid" />
    <div v-else class="flex flex-col gap-4">
    <div v-if="summoner" class="flex flex-wrap items-center gap-2">
      <AppIcon name="filter" :size="15" class="text-zinc-500" />
      <select v-model="queueFilter" class="field py-1" @change="onQueueChange">
        <option :value="null">全部模式</option>
        <option v-for="q in QUEUES" :key="q.id" :value="q.id">{{ q.label }}</option>
      </select>
      <select v-model="championFilter" class="field py-1">
        <option :value="null">全部英雄</option>
        <option v-for="c in championOptions" :key="c.id" :value="c.id">{{ c.name }}（{{ c.n }}）</option>
      </select>
      <button
        v-if="queueFilter !== null || championFilter !== null"
        class="btn btn-ghost py-1"
        @click="clearFilters"
      >
        <AppIcon name="close" :size="14" />
        清除筛选
      </button>
      <span v-if="championFilter !== null && hasMore" class="ml-auto text-xs text-zinc-500">
        英雄筛选只在已加载的 {{ games.length }} 场里找，点底部“加载更多”可以往前翻。
      </span>
    </div>

    <p v-if="error" class="text-sm break-all text-red-400">{{ error }}</p>

    <TransitionGroup name="list" tag="div" class="flex flex-col gap-1.5">
      <MatchRow
        v-for="(game, i) in shown"
        :key="game.gameId"
        :game="game"
        :puuid="summoner?.puuid ?? ''"
        :style="{ transitionDelay: `${Math.min(i % PAGE_SIZE, 12) * 25}ms` }"
      />
    </TransitionGroup>
    <div v-if="loading && games.length === 0" class="flex flex-col gap-1.5">
      <div v-for="i in 6" :key="i" class="skeleton h-14 rounded-lg" />
    </div>

    <p v-if="summoner && !loading && shown.length === 0 && !error" class="empty-state">
      {{ games.length === 0 ? "没有找到对局记录。" : "已加载的对局里没有这个英雄。" }}
    </p>

    <button
      v-if="hasMore || (loading && games.length > 0)"
      :disabled="loading"
      class="btn btn-secondary self-center px-6"
      @click="loadMore()"
    >
      {{ loading ? "加载中…" : "加载更多" }}
    </button>
    </div>
    </Transition>
  </section>
</template>
