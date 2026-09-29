<script setup lang="ts">
import { computed, ref } from "vue";
import type { GameResult, GameSummary } from "../../api";
import { useGameDataStore } from "../../stores/gameData";
import { formatDuration, kdaRatio, timeAgo } from "../../utils/format";
import ExpandTransition from "../common/ExpandTransition.vue";
import GameDetailPanel from "./GameDetailPanel.vue";

const props = defineProps<{
  game: GameSummary;
  /** Whose history this row belongs to. */
  puuid: string;
}>();
const gd = useGameDataStore();
const expanded = ref(false);

const RESULT: Record<GameResult, { label: string; bar: string; text: string; bg: string; glow: string }> = {
  win: {
    label: "胜利",
    bar: "bg-linear-to-b from-emerald-300 to-emerald-500 shadow-[0_0_10px_1px_rgb(16_185_129/0.55)]",
    text: "text-emerald-400",
    bg: "bg-linear-to-r from-emerald-500/[0.12] via-emerald-500/[0.02] to-transparent",
    glow: "hover:shadow-[0_14px_34px_-18px_rgb(16_185_129/0.55)]",
  },
  loss: {
    label: "失败",
    bar: "bg-linear-to-b from-red-300 to-red-500 shadow-[0_0_10px_1px_rgb(239_68_68/0.5)]",
    text: "text-red-400",
    bg: "bg-linear-to-r from-red-500/[0.12] via-red-500/[0.02] to-transparent",
    glow: "hover:shadow-[0_14px_34px_-18px_rgb(239_68_68/0.5)]",
  },
  remake: { label: "重开", bar: "bg-zinc-500", text: "text-zinc-400", bg: "", glow: "" },
  abort: { label: "中止", bar: "bg-zinc-600", text: "text-zinc-500", bg: "", glow: "" },
};
const BADGE = {
  mvp: {
    label: "MVP",
    tip: { title: "MVP", body: "胜方表现最好的玩家（KDA、参团、伤害、承伤、经济、视野综合）。" },
    class: "bg-amber-400/20 text-amber-200 ring-amber-300/50 shadow-[0_0_10px_-2px_rgb(245_158_11/0.7)]",
  },
  svp: {
    label: "SVP",
    tip: { title: "SVP", body: "败方表现最好的玩家（KDA、参团、伤害、承伤、经济、视野综合）。" },
    class: "bg-sky-400/15 text-sky-200 ring-sky-300/40",
  },
} as const;
const AUGMENT_RING: Record<string, string> = {
  prismatic: "ring-fuchsia-300/60",
  gold: "ring-amber-300/60",
  silver: "ring-zinc-300/40",
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
const multiKill = computed(() => MULTI_KILL[props.game.largestMultiKill] ?? null);

/** KDA colour steps: great, good, poor. */
const kdaClass = computed(() => {
  const value = kda.value === "Perfect" ? Infinity : Number(kda.value);
  if (value >= 5) return "bg-amber-400/15 text-amber-300 ring-amber-400/30";
  if (value >= 3) return "bg-emerald-400/10 text-emerald-300 ring-emerald-400/25";
  if (value < 1.5) return "bg-red-400/10 text-red-300 ring-red-400/25";
  return "bg-white/5 text-zinc-300 ring-white/10";
});

/** Labelled numbers of the stats column. */
const stats = computed(() => {
  const g = props.game;
  const kp = g.metrics?.killParticipation;
  const list = [
    { label: "补刀", value: String(g.cs), hint: `${(g.cs / minutes.value).toFixed(1)}/分` },
    { label: "伤害", value: k(g.damageToChampions), hint: share(g.metrics?.damageShare) },
    { label: "参团", value: kp === undefined ? "-" : `${Math.round(kp * 100)}%`, hint: "" },
    { label: "视野", value: String(g.visionScore), hint: "" },
  ];
  return list;
});

/** Damage per minute against the lane opponent, when the game says who that was. */
const versus = computed(() => {
  const c = props.game.comparison;
  const reference = c?.opponent ?? c?.peers;
  if (!c || !reference || reference.damage <= 0) return null;
  const diff = Math.round((c.me.damage / reference.damage - 1) * 100);
  return { diff, against: c.opponent ? "对位" : "同局均值" };
});

function k(value: number): string {
  return value >= 1000 ? `${(value / 1000).toFixed(1)}k` : String(value);
}

function share(value: number | undefined): string {
  return value === undefined ? "" : `占 ${Math.round(value * 100)}%`;
}

/** Both teams' champions, the player's own team first. Arena's many duos do not fit. */
const lineup = computed(() => {
  if (props.game.participants.length > 10) return [];
  const mine = props.game.participants.filter((p) => p.teamId === props.game.teamId);
  const theirs = props.game.participants.filter((p) => p.teamId !== props.game.teamId);
  return [mine, theirs].filter((team) => team.length > 0);
});
</script>

<template>
  <div
    class="@container overflow-hidden rounded-xl border bg-zinc-900/70 backdrop-blur-sm transition-[border-color,box-shadow,transform] duration-300 ease-out-expo"
    :class="[
      result.bg,
      expanded
        ? 'border-white/10 shadow-xl shadow-black/40'
        : ['border-white/[0.05] hover:-translate-y-0.5 hover:border-white/[0.12]', result.glow],
    ]"
  >
    <!-- Every block has a fixed width and never shrinks; only the stats absorb spare
         room. Narrow windows drop whole blocks (container queries) instead of
         squeezing icons. -->
    <div
      class="group/row flex cursor-pointer items-center gap-2 py-2 pr-3 transition-colors hover:bg-white/[0.025]"
      @click="expanded = !expanded"
    >
      <div class="my-1 w-[3px] shrink-0 self-stretch rounded-r-full" :class="result.bar" />

      <!-- Result, badge, queue, time -->
      <div class="w-[6.25rem] shrink-0 text-xs leading-5">
        <div class="flex items-center gap-1 whitespace-nowrap">
          <span class="font-semibold" :class="result.text">{{ result.label }}</span>
          <span v-if="POSITIONS[game.position]" class="text-zinc-500">{{ POSITIONS[game.position] }}</span>
          <span
            v-if="game.badge"
            v-tip="BADGE[game.badge].tip"
            class="rounded px-1 text-[10px] leading-4 font-bold tracking-wide ring-1 ring-inset"
            :class="BADGE[game.badge].class"
          >
            {{ BADGE[game.badge].label }}
          </span>
        </div>
        <div class="truncate text-zinc-300" v-tip="gd.queueName(game.queueId, game.gameMode)">
          {{ gd.queueName(game.queueId, game.gameMode) }}
        </div>
        <div class="truncate text-zinc-500">{{ timeAgo(game.createdAt) }} · {{ formatDuration(game.duration) }}</div>
      </div>

      <!-- Champion, spells, runes -->
      <div class="flex shrink-0 items-center gap-1.5">
        <div class="relative shrink-0" v-tip="gd.championName(game.championId)">
          <img
            :src="gd.championIcon(game.championId)"
            class="size-12 max-w-none rounded-xl bg-zinc-800 object-cover ring-1 ring-white/10 transition-transform duration-500 ease-spring group-hover/row:scale-110 group-hover/row:-rotate-3"
          />
          <span
            class="absolute -right-1 -bottom-1 rounded-md border border-white/10 bg-zinc-950 px-1 text-[10px] text-zinc-300 tabular-nums"
          >
            {{ game.champLevel }}
          </span>
        </div>
        <div class="grid shrink-0 grid-cols-[repeat(2,22px)] gap-0.5">
          <template v-for="(spell, i) in game.spells" :key="i">
            <img
              v-if="gd.spellIcon(spell)"
              v-tip="gd.spellTip(spell)"
              :src="gd.spellIcon(spell)"
              class="icon-hover size-[22px] max-w-none rounded-md bg-zinc-800"
            />
            <div v-else class="size-[22px] rounded-md bg-zinc-800" />
          </template>
          <img
            v-if="gd.perkIcon(game.keystone)"
            v-tip="gd.perkTip(game.keystone)"
            :src="gd.perkIcon(game.keystone)"
            class="icon-hover size-[22px] max-w-none rounded-full bg-zinc-800"
          />
          <div v-else class="size-[22px] rounded-full bg-zinc-800/60" />
          <img
            v-if="gd.perkIcon(game.subStyle)"
            v-tip="gd.perkTip(game.subStyle)"
            :src="gd.perkIcon(game.subStyle)"
            class="icon-hover size-[22px] max-w-none p-0.5"
          />
          <div v-else class="size-[22px]" />
        </div>
      </div>

      <!-- KDA -->
      <div class="w-[6.25rem] shrink-0 text-center">
        <div class="text-[15px] font-semibold tracking-tight whitespace-nowrap tabular-nums">
          {{ game.kills }}<span class="text-zinc-600"> / </span><span class="text-red-400">{{ game.deaths }}</span
          ><span class="text-zinc-600"> / </span>{{ game.assists }}
        </div>
        <div class="mt-1 flex items-center justify-center gap-1 whitespace-nowrap">
          <span class="rounded-md px-1.5 py-px text-[11px] font-medium ring-1 ring-inset tabular-nums" :class="kdaClass">
            {{ kda === "Perfect" ? "完美" : `${kda} KDA` }}
          </span>
          <span
            v-if="multiKill"
            class="rounded-md bg-linear-to-r from-rose-500 via-orange-500 to-amber-400 bg-[length:200%_100%] px-1.5 py-px text-[11px] font-semibold text-white shadow-[0_0_12px_-2px_rgb(244_63_94/0.7)] animate-shimmer"
          >
            {{ multiKill }}
          </span>
        </div>
      </div>

      <!-- Labelled stats: the only flexible block -->
      <div class="grid max-w-[19rem] min-w-[10rem] flex-1 grid-cols-4 gap-1.5">
        <div v-for="s in stats" :key="s.label" class="min-w-0">
          <div class="text-[10px] text-zinc-500">{{ s.label }}</div>
          <div class="truncate text-sm font-medium text-zinc-100 tabular-nums">{{ s.value }}</div>
          <div class="h-4 truncate text-[10px] text-zinc-500 tabular-nums">{{ s.hint }}</div>
        </div>
      </div>

      <!-- One slot: augments in Arena / ARAM: Mayhem, otherwise the lane comparison -->
      <div class="ml-auto flex w-16 shrink-0 justify-center @max-[46rem]:hidden">
        <!-- Usually four (2×2); up to six drop to smaller icons so the slot keeps its width -->
        <div
          v-if="game.augments.length"
          class="grid gap-0.5"
          :class="game.augments.length > 4 ? 'grid-cols-[repeat(3,1.25rem)]' : 'grid-cols-[repeat(2,1.625rem)]'"
        >
          <template v-for="a in game.augments" :key="a">
            <img
              v-if="gd.augmentIcon(a)"
              v-tip="gd.augmentTip(a)"
              :src="gd.augmentIcon(a)"
              class="icon-hover max-w-none rounded-md bg-zinc-950 object-cover ring-1 ring-inset"
              :class="[
                game.augments.length > 4 ? 'size-5' : 'size-[1.625rem]',
                AUGMENT_RING[gd.data?.augments?.[a]?.rarity ?? ''] ?? 'ring-white/10',
              ]"
            />
            <div
              v-else
              v-tip="gd.augmentTip(a)"
              class="rounded-md bg-zinc-800 ring-1 ring-white/10 ring-inset"
              :class="game.augments.length > 4 ? 'size-5' : 'size-[1.625rem]'"
            />
          </template>
        </div>
        <div
          v-else-if="versus"
          v-tip="{
            title: `每分钟伤害比${versus.against}${versus.diff >= 0 ? '高' : '低'} ${Math.abs(versus.diff)}%`,
            body: versus.against === '对位' ? '和本局同位置的对手比。' : '本局没有位置信息，和其他玩家的平均比。',
          }"
          class="w-full rounded-lg py-1 text-center ring-1 ring-inset"
          :class="
            versus.diff >= 0 ? 'bg-emerald-400/10 ring-emerald-400/20' : 'bg-red-400/10 ring-red-400/20'
          "
        >
          <div class="text-[10px] whitespace-nowrap text-zinc-500">{{ versus.against }}</div>
          <div
            class="text-xs font-semibold tabular-nums"
            :class="versus.diff >= 0 ? 'text-emerald-300' : 'text-red-300'"
          >
            {{ versus.diff >= 0 ? "+" : "" }}{{ versus.diff }}%
          </div>
        </div>
      </div>

      <!-- Items: six in a 3×2 grid, trinket beside them -->
      <div class="flex shrink-0 items-center gap-1">
        <div class="grid grid-cols-[repeat(3,1.625rem)] gap-0.5">
          <template v-for="(item, i) in game.items.slice(0, 6)" :key="i">
            <img
              v-if="gd.itemIcon(item)"
              v-tip="gd.itemTip(item)"
              :src="gd.itemIcon(item)"
              class="icon-hover size-[1.625rem] max-w-none rounded-md bg-zinc-800 object-cover"
            />
            <div v-else class="size-[1.625rem] rounded-md bg-zinc-800/60" />
          </template>
        </div>
        <img
          v-if="gd.itemIcon(game.items[6])"
          v-tip="gd.itemTip(game.items[6])"
          :src="gd.itemIcon(game.items[6])"
          class="icon-hover size-[1.625rem] max-w-none rounded-full bg-zinc-800 object-cover"
        />
        <div v-else class="size-[1.625rem] rounded-full bg-zinc-800/60" />
      </div>

      <!-- Both teams -->
      <div class="flex w-[6.75rem] shrink-0 flex-col gap-0.5 @max-[54rem]:hidden">
        <div v-for="(team, t) in lineup" :key="t" class="flex gap-0.5">
          <img
            v-for="p in team"
            :key="p.puuid"
            v-tip="gd.championName(p.championId)"
            :src="gd.championIcon(p.championId)"
            class="icon-hover size-5 max-w-none rounded bg-zinc-800 object-cover"
            :class="p.puuid === puuid && 'ring-1 ring-amber-400'"
          />
        </div>
      </div>

      <svg
        viewBox="0 0 24 24"
        class="size-4 shrink-0 text-zinc-500 transition-transform duration-300 ease-out-expo group-hover/row:text-zinc-300"
        :class="expanded && 'rotate-180'"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="m6 9 6 6 6-6" />
      </svg>
    </div>
    <ExpandTransition>
      <GameDetailPanel v-if="expanded" :game-id="game.gameId" :puuid="puuid" />
    </ExpandTransition>
  </div>
</template>
