<script setup lang="ts">
import { computed, ref } from "vue";
import type { GameResult, GameSummary } from "../../api";
import { useGameDataStore } from "../../stores/gameData";
import { formatDuration, kdaRatio, timeAgo } from "../../utils/format";
import GameDetailPanel from "./GameDetailPanel.vue";

const props = defineProps<{
  game: GameSummary;
  /** Whose history this row belongs to. */
  puuid: string;
}>();
const gd = useGameDataStore();
const expanded = ref(false);

const RESULT: Record<GameResult, { label: string; bar: string; text: string; bg: string }> = {
  win: { label: "胜利", bar: "bg-emerald-500", text: "text-emerald-400", bg: "bg-emerald-950/20" },
  loss: { label: "失败", bar: "bg-red-500", text: "text-red-400", bg: "bg-red-950/20" },
  remake: { label: "重开", bar: "bg-zinc-500", text: "text-zinc-400", bg: "" },
  abort: { label: "中止", bar: "bg-zinc-600", text: "text-zinc-500", bg: "" },
};
const MULTI_KILL: Record<number, string> = { 2: "双杀", 3: "三杀", 4: "四杀", 5: "五杀" };
const POSITIONS: Record<string, string> = {
  TOP: "上单",
  JUNGLE: "打野",
  MIDDLE: "中单",
  BOTTOM: "下路",
  UTILITY: "辅助",
};

const result = computed(() => RESULT[props.game.result]);
const kda = computed(() => kdaRatio(props.game.kills, props.game.deaths, props.game.assists));
const minutes = computed(() => Math.max(1, props.game.duration / 60));
const csPerMin = computed(() => (props.game.cs / minutes.value).toFixed(1));
const killParticipation = computed(() => {
  const kp = props.game.metrics?.killParticipation;
  return kp === undefined ? null : `${Math.round(kp * 100)}%`;
});
const multiKill = computed(() => MULTI_KILL[props.game.largestMultiKill] ?? null);

/** Both teams' champions, the player's own team first. */
const lineup = computed(() => {
  const mine = props.game.participants.filter((p) => p.teamId === props.game.teamId);
  const theirs = props.game.participants.filter((p) => p.teamId !== props.game.teamId);
  return [mine, theirs].filter((team) => team.length > 0);
});
</script>

<template>
  <div class="overflow-hidden rounded-md bg-zinc-900" :class="result.bg">
    <div
      class="grid cursor-pointer grid-cols-[4px_6.5rem_2.75rem_1.25rem_1.25rem_7rem_7.5rem_minmax(0,1fr)_auto_7.5rem] items-center gap-x-2.5 py-1.5 pr-3 hover:bg-zinc-800/60"
      :title="expanded ? '收起对局详情' : '展开对局详情'"
      @click="expanded = !expanded"
    >
      <div class="h-11 self-stretch" :class="result.bar" />

      <div class="min-w-0 text-xs leading-5">
        <div class="font-medium" :class="result.text">
          {{ result.label }}
          <span v-if="POSITIONS[game.position]" class="ml-1 text-zinc-500">
            {{ POSITIONS[game.position] }}
          </span>
        </div>
        <div class="truncate text-zinc-400" :title="gd.queueName(game.queueId, game.gameMode)">
          {{ gd.queueName(game.queueId, game.gameMode) }}
        </div>
        <div class="text-zinc-500">{{ timeAgo(game.createdAt) }} · {{ formatDuration(game.duration) }}</div>
      </div>

      <div class="relative">
        <img
          :src="gd.championIcon(game.championId)"
          :title="gd.championName(game.championId)"
          class="size-11 rounded-md bg-zinc-800"
        />
        <span class="absolute -right-1 -bottom-1 rounded bg-zinc-950 px-1 text-[10px] text-zinc-300">
          {{ game.champLevel }}
        </span>
      </div>

      <div class="flex flex-col gap-0.5">
        <template v-for="(spell, i) in game.spells" :key="i">
          <img v-if="gd.spellIcon(spell)" :src="gd.spellIcon(spell)" class="size-5 rounded bg-zinc-800" />
          <div v-else class="size-5 rounded bg-zinc-800" />
        </template>
      </div>

      <div class="flex flex-col items-center gap-0.5">
        <img
          v-if="gd.perkIcon(game.keystone)"
          :src="gd.perkIcon(game.keystone)"
          :title="gd.perkName(game.keystone)"
          class="size-5 rounded-full bg-zinc-800"
        />
        <div v-else class="size-5 rounded-full bg-zinc-800" />
        <img
          v-if="gd.perkIcon(game.subStyle)"
          :src="gd.perkIcon(game.subStyle)"
          :title="gd.perkName(game.subStyle)"
          class="size-4"
        />
        <div v-else class="size-4" />
      </div>

      <div class="text-center leading-5">
        <div class="text-sm font-medium tabular-nums">
          {{ game.kills }} / <span class="text-red-400">{{ game.deaths }}</span> / {{ game.assists }}
        </div>
        <div class="text-xs text-zinc-400">
          KDA {{ kda }}
          <span
            v-if="multiKill"
            class="ml-1 rounded bg-rose-700/80 px-1 text-[10px] text-white"
          >
            {{ multiKill }}
          </span>
        </div>
      </div>

      <div class="text-xs leading-5 text-zinc-400 tabular-nums">
        <div>补刀 {{ game.cs }} <span class="text-zinc-500">({{ csPerMin }})</span></div>
        <div>伤害 {{ (game.damageToChampions / 1000).toFixed(1) }}k</div>
        <div v-if="killParticipation">参团 {{ killParticipation }}</div>
      </div>

      <div />

      <div class="grid grid-cols-4 gap-0.5">
        <template v-for="(item, i) in game.items" :key="i">
          <img
            v-if="gd.itemIcon(item)"
            :src="gd.itemIcon(item)"
            class="size-6 rounded bg-zinc-800"
            :class="i === 6 && 'rounded-full'"
          />
          <div v-else class="size-6 rounded bg-zinc-800/70" :class="i === 6 && 'rounded-full'" />
        </template>
      </div>

      <div class="flex flex-col gap-0.5">
        <div v-for="(team, t) in lineup" :key="t" class="flex gap-0.5">
          <img
            v-for="p in team"
            :key="p.puuid"
            :src="gd.championIcon(p.championId)"
            :title="gd.championName(p.championId)"
            class="size-5 rounded bg-zinc-800"
            :class="p.puuid === puuid && 'ring-1 ring-amber-400'"
          />
        </div>
      </div>
    </div>
    <GameDetailPanel v-if="expanded" :game-id="game.gameId" :puuid="puuid" />
  </div>
</template>
