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
  ranked: "排位",
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
  (puuid) => {
    summoner.value = null;
    profile.value = null;
    api
      .summonerByPuuid(puuid)
      .then((s) => {
        if (puuid === props.player.puuid) summoner.value = s;
      })
      .catch(() => {});
  },
  { immediate: true },
);

// Re-evaluated when the pick changes (practice / signature tags) or the queue becomes known.
watch(
  () => [props.player.puuid, props.player.championId, props.queueId, props.player.position] as const,
  async ([puuid, championId, queueId, position]) => {
    error.value = null;
    try {
      const p = await api.playerProfile(puuid, championId, queueId, position);
      if (puuid === props.player.puuid) profile.value = p;
    } catch (err) {
      if (puuid === props.player.puuid) error.value = String(err);
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
const hiddenTags = computed(() => tags.value.slice(MAX_TAGS));

const powerClass = computed(() => {
  const p = props.power?.power ?? 50;
  if (p >= 60) return "bg-emerald-400/10 text-emerald-300 ring-emerald-400/30";
  if (p <= 40) return "bg-red-400/10 text-red-300 ring-red-400/30";
  return "bg-white/5 text-zinc-300 ring-white/10";
});

const winRateClass = computed(() => {
  const rate = profile.value?.winRate ?? 0;
  if (rate >= 0.6) return "text-emerald-400";
  if (rate < 0.45) return "text-red-400";
  return "text-zinc-200";
});

function percent(value: number): string {
  return `${Math.round(value * 100)}%`;
}

function openHistory() {
  router.push({ path: "/match-history", query: { puuid: props.player.puuid } });
}
</script>

<template>
  <div
    class="group flex w-full cursor-pointer items-start gap-3 rounded-xl border bg-zinc-900/75 px-3 py-2.5 transition-[background-color,border-color,transform,box-shadow] duration-300 ease-out-expo hover:-translate-y-px hover:bg-zinc-800/60 active:translate-y-0"
    :class="
      player.isSelf
        ? 'border-amber-400/40 shadow-[0_0_0_1px_rgb(245_158_11/0.1),0_8px_24px_-14px_rgb(245_158_11/0.5)]'
        : 'border-white/[0.06] hover:border-white/10'
    "
    @click="openHistory"
  >
    <div class="relative shrink-0">
      <img
        v-if="player.championId > 0"
        :src="gd.championIcon(player.championId)"
        :title="gd.championName(player.championId)"
        class="size-11 rounded-lg bg-zinc-800 ring-1 ring-white/10"
      />
      <div v-else class="size-11 rounded-lg bg-zinc-800 ring-1 ring-white/10" />
      <span
        v-if="POSITIONS[player.position]"
        class="absolute -bottom-1.5 left-1/2 -translate-x-1/2 rounded-md border border-white/10 bg-zinc-950 px-1 text-[10px] whitespace-nowrap text-zinc-300"
      >
        {{ POSITIONS[player.position] }}
      </span>
    </div>

    <div class="min-w-0 flex-1">
      <div class="flex items-baseline gap-2">
        <span class="truncate font-medium text-zinc-100 select-text">{{ name }}</span>
        <span v-if="summoner" class="shrink-0 text-xs text-zinc-500">
          Lv.{{ summoner.summonerLevel }}
        </span>
        <span
          v-if="power"
          class="ml-auto shrink-0 rounded-full px-2 py-0.5 text-xs font-semibold tabular-nums ring-1 ring-inset"
          :class="powerClass"
          :title="power.breakdown"
        >
          战力 {{ Math.round(power.power) }}
        </span>
      </div>

      <div v-if="tags.length" class="mt-1 flex flex-wrap gap-1" @click.stop>
        <TagChip v-for="tag in shownTags" :key="tag.id + tag.label" :tag="tag" />
        <span
          v-if="hiddenTags.length"
          class="rounded-md bg-white/5 px-1.5 py-[3px] text-[11px] leading-none text-zinc-400 ring-1 ring-white/10 ring-inset"
          :title="hiddenTags.map((t) => `${t.label}：${t.detail}`).join('\n')"
        >
          +{{ hiddenTags.length }}
        </span>
      </div>

      <p v-if="error" class="mt-1 truncate text-xs text-red-400" :title="error">{{ error }}</p>
      <p v-else-if="!profile" class="mt-1 text-xs text-zinc-500">加载战绩…</p>
      <p v-else-if="profile.sampleGames === 0" class="mt-1 text-xs text-zinc-500">近期没有对局</p>
      <template v-else>
        <div class="mt-1 flex flex-wrap items-center gap-x-3 gap-y-0.5 text-xs">
          <span :class="winRateClass" :title="`样本：近期${SCOPES[profile.scope]}对局`">
            胜率 {{ percent(profile.winRate) }}
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
            :title="`Akari Score，基于 ${profile.akariScore.games} 场完整数据`"
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
        <div class="mt-1 flex items-center gap-3">
          <div class="flex gap-0.5">
            <span
              v-for="(r, i) in profile.recent"
              :key="i"
              class="size-2 rounded-full"
              :class="DOT[r]"
            />
          </div>
          <div class="flex gap-1">
            <img
              v-for="c in profile.topChampions"
              :key="c.championId"
              :src="gd.championIcon(c.championId)"
              :title="`${gd.championName(c.championId)} ${c.wins}胜${c.games - c.wins}负`"
              class="size-5 rounded-md bg-zinc-800 ring-1 ring-white/10"
            />
          </div>
        </div>
      </template>
    </div>
  </div>
</template>
