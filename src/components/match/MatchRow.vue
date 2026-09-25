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
const expanded = ref(false);
const gd = useGameDataStore();

const RESULT: Record<GameResult, { label: string; bar: string; text: string }> = {
  win: { label: "胜利", bar: "bg-emerald-500", text: "text-emerald-400" },
  loss: { label: "失败", bar: "bg-red-500", text: "text-red-400" },
  remake: { label: "重开", bar: "bg-zinc-500", text: "text-zinc-400" },
  abort: { label: "中止", bar: "bg-zinc-600", text: "text-zinc-500" },
};

const result = computed(() => RESULT[props.game.result]);
const kda = computed(() => kdaRatio(props.game.kills, props.game.deaths, props.game.assists));
const csPerMin = computed(() =>
  props.game.duration > 0 ? (props.game.cs / (props.game.duration / 60)).toFixed(1) : "0",
);
</script>

<template>
  <div class="overflow-hidden rounded-md bg-zinc-900">
  <div
    class="flex cursor-pointer items-center gap-3 pr-3 hover:bg-zinc-800/60"
    :title="expanded ? '收起对局详情' : '展开对局详情'"
    @click="expanded = !expanded"
  >
    <div class="w-1 self-stretch" :class="result.bar" />

    <div class="w-24 shrink-0 py-2 text-xs">
      <div class="font-medium" :class="result.text">{{ result.label }}</div>
      <div class="truncate text-zinc-400" :title="gd.queueName(game.queueId, game.gameMode)">
        {{ gd.queueName(game.queueId, game.gameMode) }}
      </div>
      <div class="text-zinc-500">{{ timeAgo(game.createdAt) }} · {{ formatDuration(game.duration) }}</div>
    </div>

    <div class="relative shrink-0">
      <img
        :src="gd.championIcon(game.championId)"
        :alt="gd.championName(game.championId)"
        :title="gd.championName(game.championId)"
        class="size-11 rounded-md bg-zinc-800"
      />
      <span class="absolute -right-1 -bottom-1 rounded bg-zinc-950 px-1 text-[10px] text-zinc-300">
        {{ game.champLevel }}
      </span>
    </div>

    <div class="flex shrink-0 flex-col gap-0.5">
      <template v-for="(spell, i) in game.spells" :key="i">
        <img v-if="gd.spellIcon(spell)" :src="gd.spellIcon(spell)" class="size-5 rounded bg-zinc-800" />
        <div v-else class="size-5 rounded bg-zinc-800" />
      </template>
    </div>

    <div class="w-28 shrink-0 text-center">
      <div class="text-sm font-medium">
        {{ game.kills }} / <span class="text-red-400">{{ game.deaths }}</span> / {{ game.assists }}
      </div>
      <div class="text-xs text-zinc-400">KDA {{ kda }}</div>
    </div>

    <div class="w-20 shrink-0 text-xs text-zinc-400">
      <div>补刀 {{ game.cs }} ({{ csPerMin }})</div>
      <div>伤害 {{ (game.damageToChampions / 1000).toFixed(1) }}k</div>
    </div>

    <div class="ml-auto flex gap-0.5">
      <template v-for="(item, i) in game.items" :key="i">
        <img v-if="gd.itemIcon(item)" :src="gd.itemIcon(item)" class="size-7 rounded bg-zinc-800" />
        <div v-else class="size-7 rounded bg-zinc-800" />
      </template>
    </div>
    </div>
    <GameDetailPanel v-if="expanded" :game-id="game.gameId" :puuid="puuid" />
  </div>
</template>
