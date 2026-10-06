<script setup lang="ts">
import { computed, shallowRef, watch } from "vue";
import { useRouter } from "vue-router";
import {
  api,
  type GameResult,
  type PlayerPower,
  type PlayerProfile,
  type PlayerTag,
  type RosterPlayer,
  type SampleScope,
  type Summoner,
} from "../../api";
import { useGameDataStore } from "../../stores/gameData";
import TagChip from "./TagChip.vue";

const props = defineProps<{
  player: RosterPlayer;
  /** 0 when unknown. */
  queueId: number;
  /** Roster-wide tags (premade, met, gank, lane, horses). */
  relationTags?: PlayerTag[];
  power?: PlayerPower;
}>();
const gd = useGameDataStore();
const router = useRouter();

const summoner = shallowRef<Summoner | null>(null);
const profile = shallowRef<PlayerProfile | null>(null);
const error = shallowRef<string | null>(null);

const MAX_TAGS = 5;
const POSITIONS: Record<string, string> = {
  TOP: "上单",
  JUNGLE: "打野",
  MIDDLE: "中单",
  BOTTOM: "下路",
  UTILITY: "辅助",
};
const SCOPES: Record<SampleScope, string> = {
  ranked: "单双排",
  sameQueue: "同模式",
  all: "全部",
};
const DOT: Record<GameResult, string> = {
  win: "bg-emerald-500",
  loss: "bg-red-500",
  remake: "bg-zinc-500",
  abort: "bg-zinc-600",
};

watch(
  () => props.player.puuid,
  (puuid, _previous, onCleanup) => {
    let active = true;
    onCleanup(() => { active = false; });
    summoner.value = null;
    profile.value = null;
    api
      .summonerByPuuid(puuid)
      .then((s) => {
        if (active) summoner.value = s;
      })
      .catch(() => {});
  },
  { immediate: true },
);

// Re-evaluated when the pick changes (practice / signature tags) or the queue becomes known.
watch(
  () => [props.player.puuid, props.player.championId, props.queueId, props.player.position] as const,
  async ([puuid, championId, queueId, position], _previous, onCleanup) => {
    let active = true;
    onCleanup(() => { active = false; });
    error.value = null;
    try {
      const p = await api.playerProfile(puuid, championId, queueId, position);
      if (active) profile.value = p;
    } catch (err) {
      if (active) error.value = String(err);
    }
  },
  { immediate: true },
);

const name = computed(() => {
  const s = summoner.value;
  if (!s) return "…";
  return s.gameName ? `${s.gameName}#${s.tagLine}` : s.displayName || "…";
});

const tags = computed(() => {
  const all = [...(props.relationTags ?? []), ...(profile.value?.tags ?? [])];
  return all.sort((a, b) => b.priority - a.priority);
});
const shownTags = computed(() => tags.value.slice(0, MAX_TAGS));
/** 小代 cards burn: gold for a teammate, red for an opponent. */
const carryCard = computed(() => {
  const carry = tags.value.find((t) => t.id === "carry");
  if (!carry) return "";
  return carry.tone === "positive"
    ? "border-orange-400/50 bg-linear-to-r from-orange-500/[0.14] via-red-500/[0.05] to-zinc-900/65 shadow-[0_0_22px_-8px_rgb(249_115_22/0.75)] hover:border-orange-300/70"
    : "border-red-500/50 bg-linear-to-r from-red-600/[0.16] via-fuchsia-600/[0.05] to-zinc-900/65 shadow-[0_0_22px_-8px_rgb(239_68_68/0.8)] hover:border-red-400/70";
});
const hiddenTags = computed(() => tags.value.slice(MAX_TAGS));

const powerClass = computed(() => {
  const p = props.power?.power ?? 50;
  if (p >= 60) return "bg-emerald-400/10 text-emerald-300 ring-emerald-400/30";
  if (p <= 40) return "bg-red-400/10 text-red-300 ring-red-400/30";
  return "bg-white/5 text-zinc-300 ring-white/10";
});

/** Fewer ranked games this week than this and the rate is marked 样本少. */
const WEEK_SAMPLE_MIN = 5;
const week = computed(() => profile.value?.week ?? null);
const weekClass = computed(() => {
  const w = week.value;
  if (!w || w.games === 0) return "text-zinc-500";
  const rate = w.wins / w.games;
  if (rate >= 0.6) return "text-emerald-400";
  if (rate < 0.45) return "text-red-400";
  return "text-zinc-200";
});

const winRateClass = computed(() => {
  const rate = profile.value?.winRate ?? 0;
  if (rate >= 0.6) return "text-emerald-400";
  if (rate < 0.45) return "text-red-400";
  return "text-zinc-200";
});

/** Recent results across all modes, and ranked on its own: an ARAM losing run should not
 * read as a bad ranked player. */
const resultRows = computed(() => {
  const p = profile.value;
  if (!p) return [];
  const rows = [{ label: "近期", results: p.recent, tip: "最近对局（含娱乐模式），左边最新" }];
  if (p.recentRanked.length) {
    rows.push({ label: "单双", results: p.recentRanked, tip: "最近单双排（不含灵活组排），左边最新" });
  }
  return rows;
});

function percent(value: number): string {
  return `${Math.round(value * 100)}%`;
}

function openHistory() {
  router.push({ path: "/match-history", query: { puuid: props.player.puuid, from: "ongoing" } });
}
</script>

<template>
  <div
    class="group flex w-full cursor-pointer items-start gap-3 rounded-xl border bg-zinc-900/65 px-3 py-2 backdrop-blur-md transition-[background-color,border-color,transform,box-shadow] duration-300 ease-out-expo hover:-translate-y-0.5 hover:bg-zinc-800/60 hover:shadow-[0_16px_36px_-20px_rgb(0_0_0/0.9)] active:translate-y-0 active:scale-[0.995]"
    :class="
      player.isSelf
        ? 'animate-glow border-amber-400/40 bg-linear-to-r from-amber-500/[0.08] to-zinc-900/65 shadow-[0_0_0_1px_rgb(245_158_11/0.1),0_8px_24px_-14px_rgb(245_158_11/0.5)]'
        : carryCard || 'border-white/[0.06] hover:border-white/[0.12]'
    "
    role="link"
    tabindex="0"
    @click="openHistory"
    @keydown.enter.self="openHistory"
  >
    <div class="relative shrink-0">
      <img
        v-if="player.championId > 0"
        :src="gd.championIcon(player.championId)"
        v-tip="gd.championName(player.championId)"
        class="size-11 rounded-lg bg-zinc-800 ring-1 ring-white/10 transition-transform duration-500 ease-spring group-hover:scale-110 group-hover:-rotate-3"
      />
      <div v-else class="skeleton size-11 rounded-lg ring-1 ring-white/10" />
      <span
        v-if="POSITIONS[player.position]"
        class="absolute -bottom-1.5 left-1/2 -translate-x-1/2 rounded-md border border-white/10 bg-zinc-950 px-1 text-[10px] whitespace-nowrap text-zinc-300"
      >
        {{ POSITIONS[player.position] }}
      </span>
    </div>

    <div class="min-w-0 flex-1">
      <div class="flex items-baseline gap-2">
        <span v-if="summoner" class="truncate font-medium text-zinc-100 select-text">{{ name }}</span>
        <span v-else class="skeleton h-4 w-28 rounded" />
        <span v-if="summoner" class="shrink-0 text-xs text-zinc-500">
          Lv.{{ summoner.summonerLevel }}
        </span>
        <span
          v-if="power"
          class="ml-auto shrink-0 rounded-full px-2 py-0.5 text-xs font-semibold tabular-nums ring-1 ring-inset"
          :class="powerClass"
          v-tip="{ title: `战力 ${Math.round(power.power)}`, body: power.breakdown }"
        >
          战力 {{ Math.round(power.power) }}
        </span>
      </div>

      <div v-if="tags.length" class="mt-1 flex flex-wrap gap-1" @click.stop>
        <TagChip v-for="tag in shownTags" :key="tag.id + tag.label" :tag="tag" />
        <span
          v-if="hiddenTags.length"
          class="rounded-md bg-white/5 px-1.5 py-[3px] text-[11px] leading-none text-zinc-400 ring-1 ring-white/10 ring-inset"
          v-tip="{
            title: `另外 ${hiddenTags.length} 个标签`,
            body: hiddenTags.map((t) => `${t.label}：${t.detail}`).join('\n\n'),
          }"
        >
          +{{ hiddenTags.length }}
        </span>
      </div>

      <p v-if="error" class="mt-1 truncate text-xs text-red-400" v-tip="error">{{ error }}</p>
      <p v-else-if="!profile" class="mt-1 text-xs text-zinc-500">加载战绩…</p>
      <p v-else-if="profile.sampleGames === 0" class="mt-1 text-xs text-zinc-500">近期没有对局</p>
      <template v-else>
        <div class="mt-1 flex flex-wrap items-center gap-x-3 gap-y-0.5 text-xs">
          <span
            v-if="week"
            :class="weekClass"
            v-tip="`最近 7 天单双排（不含灵活组排）${week.games < WEEK_SAMPLE_MIN ? '，场次较少，仅供参考' : ''}`"
          >
            <template v-if="week.games > 0">
              本周单双排 {{ percent(week.wins / week.games) }}
              <span class="text-zinc-500">({{ week.wins }}/{{ week.games }})</span>
            </template>
            <template v-else>本周无单双排</template>
            <span
              v-if="week.games < WEEK_SAMPLE_MIN"
              class="ml-0.5 rounded bg-amber-400/10 px-1 py-px text-[10px] text-amber-300 ring-1 ring-amber-400/25 ring-inset"
            >
              样本少
            </span>
          </span>
          <span
            :class="week ? 'text-zinc-500' : winRateClass"
            v-tip="`样本：近期${SCOPES[profile.scope]}对局，KDA 和评分也按这些对局算`"
          >
            {{ week ? "近期" : "胜率" }} {{ percent(profile.winRate) }}
            <span class="text-zinc-500">
              ({{ profile.wins }}/{{ profile.sampleGames }} {{ SCOPES[profile.scope] }})
            </span>
          </span>
          <span class="text-zinc-400">
            {{ profile.avgKills.toFixed(1) }} / {{ profile.avgDeaths.toFixed(1) }} /
            {{ profile.avgAssists.toFixed(1) }}
          </span>
          <span
            v-if="profile.akariScore"
            class="text-zinc-400"
            v-tip="`Akari Score，基于 ${profile.akariScore.games} 场完整数据`"
          >
            评分 {{ profile.akariScore.total.toFixed(1) }}
          </span>
        </div>
        <div v-if="profile.team" class="mt-0.5 flex flex-wrap gap-x-3 text-xs text-zinc-500">
          <span>伤害 {{ percent(profile.team.damageShare) }}</span>
          <span>承伤 {{ percent(profile.team.damageTakenShare) }}</span>
          <span>经济 {{ percent(profile.team.goldShare) }}</span>
          <span>参团 {{ percent(profile.team.killParticipation) }}</span>
        </div>
        <div class="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1">
          <div
            v-for="row in resultRows"
            :key="row.label"
            class="flex items-center gap-1"
            v-tip="row.tip"
          >
            <span class="text-[10px] leading-none text-zinc-500">{{ row.label }}</span>
            <div class="flex gap-0.5">
              <span
                v-for="(r, i) in row.results"
                :key="i"
                class="size-2 rounded-full transition-transform duration-300 ease-spring group-hover:scale-125"
                :class="DOT[r]"
                :style="{ transitionDelay: `${i * 20}ms` }"
              />
            </div>
          </div>
          <div class="flex gap-1">
            <img
              v-for="c in profile.topChampions"
              :key="c.championId"
              :src="gd.championIcon(c.championId)"
              v-tip="`${gd.championName(c.championId)} ${c.wins}胜${c.games - c.wins}负`"
              class="size-5 rounded-md bg-zinc-800 ring-1 ring-white/10"
            />
          </div>
        </div>
      </template>
    </div>
  </div>
</template>
