<script setup lang="ts">
import { computed, ref, shallowRef, watch } from "vue";
import { useRoute } from "vue-router";
import { api, PANEL_HISTORY_COUNT, type DataSource, type GameSummary, type Summoner } from "../api";
import MatchRow from "../components/match/MatchRow.vue";
import { profileIconUrl } from "../stores/gameData";
import { useLcuStore } from "../stores/lcu";

// Same size as the game panel, so opening a player from there hits the cache.
const PAGE_SIZE = PANEL_HISTORY_COUNT;

const lcu = useLcuStore();
const route = useRoute();

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

function loadMore() {
  if (!loading.value) fetchPage(generation);
}

async function fetchPage(gen: number) {
  const target = summoner.value;
  if (!target) return;
  loading.value = true;
  error.value = null;
  try {
    const page = await api.matchHistory(target.puuid, games.value.length, PAGE_SIZE);
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
  <section class="flex max-w-5xl flex-col gap-4">
    <div class="flex items-center gap-3">
      <h1 class="text-xl font-semibold">战绩</h1>
      <form class="ml-auto flex gap-2" @submit.prevent="search">
        <input
          v-model="query"
          placeholder="名字#标签"
          class="w-64 rounded-md border border-zinc-700 bg-zinc-900 px-3 py-1.5 text-sm outline-none focus:border-amber-400"
        />
        <button
          type="submit"
          :disabled="!lcu.connected"
          class="rounded-md bg-amber-500 px-3 py-1.5 text-sm font-medium text-zinc-950 hover:bg-amber-400 disabled:opacity-40"
        >
          查询
        </button>
        <button
          type="button"
          :disabled="!lcu.snapshot.summoner"
          class="rounded-md border border-zinc-700 px-3 py-1.5 text-sm hover:bg-zinc-800 disabled:opacity-40"
          @click="showSelf"
        >
          我自己
        </button>
      </form>
    </div>

    <p v-if="!lcu.connected" class="text-sm text-zinc-400">连接英雄联盟客户端后才能查询战绩。</p>

    <div v-if="summoner" class="flex items-center gap-3 rounded-lg border border-zinc-800 bg-zinc-900 p-3">
      <img :src="profileIconUrl(summoner.profileIconId)" class="size-12 rounded-full bg-zinc-800" />
      <div class="min-w-0">
        <div class="truncate text-lg font-medium">{{ riotId }}</div>
        <div class="text-sm text-zinc-400">等级 {{ summoner.summonerLevel }}</div>
      </div>
      <div v-if="source" class="ml-auto text-right text-xs">
        <span
          class="rounded px-1.5 py-0.5"
          :class="source === 'sgp' ? 'bg-emerald-900 text-emerald-300' : 'bg-zinc-800 text-zinc-300'"
        >
          数据来源 {{ source.toUpperCase() }}
        </span>
        <div v-if="sgpError" class="mt-1 max-w-xs truncate text-zinc-500" :title="sgpError">
          SGP 不可用：{{ sgpError }}
        </div>
      </div>
    </div>

    <p v-if="error" class="text-sm break-all text-red-400">{{ error }}</p>

    <div class="flex flex-col gap-1.5">
      <MatchRow v-for="game in games" :key="game.gameId" :game="game" />
    </div>

    <p v-if="summoner && !loading && games.length === 0 && !error" class="text-sm text-zinc-400">
      没有找到对局记录。
    </p>

    <button
      v-if="hasMore || loading"
      :disabled="loading"
      class="self-center rounded-md border border-zinc-700 px-4 py-1.5 text-sm hover:bg-zinc-800 disabled:opacity-50"
      @click="loadMore()"
    >
      {{ loading ? "加载中…" : "加载更多" }}
    </button>
  </section>
</template>
