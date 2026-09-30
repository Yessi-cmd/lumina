<script setup lang="ts">
import { computed, ref, shallowRef, watch } from "vue";
import { useRoute } from "vue-router";
import { api, PANEL_HISTORY_COUNT, type DataSource, type GameSummary, type Summoner } from "../api";
import CareerPanel from "../components/career/CareerPanel.vue";
import AppIcon from "../components/common/AppIcon.vue";
import CountUp from "../components/common/CountUp.vue";
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
  { id: 2400, label: "海克斯大乱斗" },
];

type Outcome = "all" | "win" | "loss" | "mvp" | "svp";
const OUTCOMES: { id: Outcome; label: string }[] = [
  { id: "all", label: "全部" },
  { id: "win", label: "胜利" },
  { id: "loss", label: "失败" },
  { id: "mvp", label: "MVP" },
  { id: "svp", label: "SVP" },
];
/** Champion and outcome filters run on loaded games, so a narrow filter keeps loading
 * older pages until it has this many games, up to AUTO_LOAD_LIMIT games in total. */
const AUTO_FILL = 10;
const AUTO_LOAD_LIMIT = 200;

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
/** A name lookup is in flight (the game list has its own `loading`). */
const searching = ref(false);
// Guards against a slow response for a previous player overwriting the current one.
let generation = 0;

/** Champions in the loaded games, most played first. */
const championOptions = computed(() => {
  const counts = new Map<number, number>();
  for (const g of games.value) counts.set(g.championId, (counts.get(g.championId) ?? 0) + 1);
  const options = [...counts].map(([id, n]) => ({ id, n, name: gd.championName(id) }));
  return options.sort((a, b) => b.n - a.n || a.name.localeCompare(b.name, "zh-CN"));
});

const outcomeFilter = ref<Outcome>("all");

function matchesOutcome(g: GameSummary, outcome: Outcome): boolean {
  switch (outcome) {
    case "all":
      return true;
    case "win":
    case "loss":
      return g.result === outcome;
    case "mvp":
    case "svp":
      return g.badge === outcome;
  }
}

const shown = computed(() => {
  const champion = championFilter.value;
  const outcome = outcomeFilter.value;
  return games.value.filter(
    (g) => (champion === null || g.championId === champion) && matchesOutcome(g, outcome),
  );
});

/** Filters applied to loaded games rather than by the server. */
const localFilter = computed(() => championFilter.value !== null || outcomeFilter.value !== "all");

// Keep paging back while a local filter has too few games to show.
watch(
  () => [localFilter.value, shown.value.length, loading.value, hasMore.value] as const,
  ([filtered, count, busy, more]) => {
    const room = games.value.length < AUTO_LOAD_LIMIT && !error.value;
    if (filtered && !busy && more && room && count < AUTO_FILL) {
      loadMore();
    }
  },
);

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
  outcomeFilter.value = "all";
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
  searching.value = true;
  try {
    await show(await api.lookupSummoner(text));
  } catch (err) {
    error.value = String(err);
  } finally {
    searching.value = false;
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
  <section class="stagger flex max-w-6xl flex-col gap-4">
    <header class="page-header flex items-end gap-4">
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
        <button type="submit" :disabled="!lcu.connected || searching || !query.trim()" class="btn btn-primary">
          {{ searching ? "查询中…" : "查询" }}
        </button>
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
    <p v-if="error" class="text-sm break-all text-red-400">{{ error }}</p>

    <div v-if="summoner" class="card overflow-hidden p-4">
      <div
        class="pointer-events-none absolute -top-24 -left-12 size-64 rounded-full bg-[radial-gradient(closest-side,rgb(245_158_11/0.18),transparent)]"
      />
      <div
        class="pointer-events-none absolute -right-10 -bottom-24 size-64 rounded-full transition-colors duration-700"
        :class="
          summary && summary.winRate >= 50
            ? 'bg-[radial-gradient(closest-side,rgb(16_185_129/0.14),transparent)]'
            : 'bg-[radial-gradient(closest-side,rgb(239_68_68/0.12),transparent)]'
        "
      />
      <div class="relative flex items-center gap-4">
        <div class="relative shrink-0">
          <img
            :src="profileIconUrl(summoner.profileIconId)"
            class="size-14 rounded-2xl bg-zinc-800 ring-2 ring-amber-400/40 shadow-[0_8px_24px_-8px_rgb(245_158_11/0.5)]"
          />
          <span
            class="absolute -bottom-2 left-1/2 -translate-x-1/2 rounded-full border border-amber-300/40 bg-linear-to-b from-zinc-800 to-zinc-950 px-1.5 text-[11px] font-semibold text-amber-200 tabular-nums"
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

        <div v-if="summary" class="ml-auto flex items-center gap-6 pr-2">
          <div class="text-right">
            <div class="eyebrow">场次</div>
            <div class="mt-0.5 text-lg font-semibold text-zinc-100">
              <CountUp :value="summary.games" />
              <span class="ml-1 text-xs font-normal text-zinc-500"><CountUp :value="summary.wins" /> 胜</span>
            </div>
          </div>
          <div class="text-right">
            <div class="eyebrow">KDA</div>
            <div class="mt-0.5 text-lg font-semibold text-zinc-100">
              <CountUp v-if="summary.kda !== 'Perfect'" :value="Number(summary.kda)" :decimals="2" />
              <span v-else class="text-amber-300">完美</span>
            </div>
          </div>
          <!-- Win rate ring -->
          <div class="relative size-16 shrink-0">
            <svg viewBox="0 0 36 36" class="size-full -rotate-90">
              <circle cx="18" cy="18" r="15.5" fill="none" stroke="rgb(255 255 255 / 0.07)" stroke-width="3" />
              <circle
                cx="18"
                cy="18"
                r="15.5"
                fill="none"
                stroke-width="3"
                stroke-linecap="round"
                pathLength="100"
                stroke-dasharray="100"
                :stroke-dashoffset="100 - summary.winRate"
                class="transition-[stroke-dashoffset,stroke] duration-1000 ease-out-expo"
                :class="summary.winRate >= 50 ? 'stroke-emerald-400' : 'stroke-red-400'"
                :style="{ filter: `drop-shadow(0 0 4px ${summary.winRate >= 50 ? 'rgb(52 211 153 / 0.6)' : 'rgb(248 113 113 / 0.6)'})` }"
              />
            </svg>
            <div class="absolute inset-0 flex flex-col items-center justify-center leading-none">
              <span
                class="text-sm font-semibold"
                :class="summary.winRate >= 50 ? 'text-emerald-300' : 'text-red-300'"
              >
                <CountUp :value="summary.winRate" />%
              </span>
              <span class="mt-0.5 text-[9px] text-zinc-500">胜率</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Tabs with a pill that slides between them. -->
    <div v-if="summoner" class="segmented relative self-start">
      <span
        class="pointer-events-none absolute inset-y-0.5 left-0.5 w-[calc(50%-3px)] rounded-md bg-linear-to-b from-amber-300/25 to-amber-500/10 shadow-[inset_0_1px_0_0_rgb(255_255_255/0.1),0_4px_14px_-6px_rgb(245_158_11/0.6)] ring-1 ring-amber-400/25 transition-transform duration-500 ease-spring ring-inset"
        :style="{ transform: view === 'career' ? 'translateX(calc(100% + 2px))' : 'none' }"
      />
      <button
        class="segment relative w-24 py-1.5 text-sm"
        :class="view === 'games' && 'text-amber-100'"
        @click="view = 'games'"
      >
        对局记录
      </button>
      <button
        class="segment relative w-24 py-1.5 text-sm"
        :class="view === 'career' && 'text-amber-100'"
        @click="view = 'career'"
      >
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
      <div class="segmented">
        <button
          v-for="o in OUTCOMES"
          :key="o.id"
          class="segment px-3"
          :class="outcomeFilter === o.id && 'segment-active'"
          @click="outcomeFilter = o.id"
        >
          {{ o.label }}
        </button>
      </div>
      <button
        v-if="queueFilter !== null || localFilter"
        class="btn btn-ghost py-1"
        @click="clearFilters"
      >
        <AppIcon name="close" :size="14" />
        清除筛选
      </button>
      <span v-if="localFilter" class="ml-auto text-xs text-zinc-500">
        <template v-if="source === 'lcu' && (outcomeFilter === 'mvp' || outcomeFilter === 'svp')">
          MVP/SVP 需要 SGP 数据，当前来源没有。
        </template>
        <template v-else>
          在已加载的 {{ games.length }} 场中找到 {{ shown.length }} 场{{
            hasMore ? (loading ? "，继续往前翻…" : "，点底部“加载更多”继续往前翻") : ""
          }}
        </template>
      </span>
    </div>

    <TransitionGroup name="list" tag="div" class="flex flex-col gap-1.5">
      <MatchRow
        v-for="(game, i) in shown"
        :key="game.gameId"
        :game="game"
        :puuid="summoner?.puuid ?? ''"
        :style="{ transitionDelay: `${Math.min(i % PAGE_SIZE, 12) * 35}ms` }"
      />
    </TransitionGroup>
    <div v-if="loading && games.length === 0" class="flex flex-col gap-1.5">
      <div v-for="i in 6" :key="i" class="skeleton h-14 rounded-lg" />
    </div>

    <p v-if="summoner && !loading && shown.length === 0 && !error" class="empty-state">
      {{ games.length === 0 ? "没有找到对局记录。" : "已加载的对局里没有符合筛选的记录。" }}
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
