<script setup lang="ts">
import { computed, ref, shallowRef, watch } from "vue";
import { useRouter } from "vue-router";
import { api, type GameDetail, type PlayerBuild, type PlayerLine, type TeamDetail } from "../../api";
import { useGameDataStore } from "../../stores/gameData";

const props = defineProps<{
  gameId: number;
  /** Player whose history this is; their row is highlighted. */
  puuid: string;
}>();
const gd = useGameDataStore();
const router = useRouter();

type Tab = "overview" | "stats" | "builds" | "runes";
const TABS: { id: Tab; label: string }[] = [
  { id: "overview", label: "概要" },
  { id: "stats", label: "数据" },
  { id: "builds", label: "出装加点" },
  { id: "runes", label: "符文" },
];
const tab = ref<Tab>("overview");

const detail = shallowRef<GameDetail | null>(null);
const error = shallowRef<string | null>(null);
const builds = shallowRef<PlayerBuild[] | null>(null);
const buildsError = shallowRef<string | null>(null);

watch(
  () => props.gameId,
  async (gameId) => {
    detail.value = null;
    error.value = null;
    builds.value = null;
    buildsError.value = null;
    try {
      const d = await api.gameDetail(gameId);
      if (gameId === props.gameId) detail.value = d;
    } catch (err) {
      if (gameId === props.gameId) error.value = String(err);
    }
  },
  { immediate: true },
);

// The timeline is large, so builds load only when their tab is opened.
watch(tab, async (t) => {
  if (t !== "builds" || builds.value || buildsError.value) return;
  const gameId = props.gameId;
  try {
    const b = await api.gameBuilds(gameId);
    if (gameId === props.gameId) builds.value = b;
  } catch (err) {
    if (gameId === props.gameId) buildsError.value = String(err);
  }
});

const players = computed(() => detail.value?.teams.flatMap((t) => t.players) ?? []);
const maxDamage = computed(() => Math.max(1, ...players.value.map((p) => p.damageToChampions)));
const minutes = computed(() => Math.max(1, (detail.value?.duration ?? 60) / 60));

function name(p: PlayerLine): string {
  if (!p.gameName) return "未知玩家";
  return p.tagLine ? `${p.gameName}#${p.tagLine}` : p.gameName;
}

function kda(p: PlayerLine): string {
  return ((p.kills + p.assists) / Math.max(1, p.deaths)).toFixed(1);
}

function kp(p: PlayerLine, team: TeamDetail): string {
  return `${Math.round(((p.kills + p.assists) / Math.max(1, team.kills)) * 100)}%`;
}

function k(value: number): string {
  return value >= 1000 ? `${(value / 1000).toFixed(1)}k` : String(value);
}

function openPlayer(p: PlayerLine) {
  if (p.puuid && p.puuid !== props.puuid) {
    router.push({ path: "/match-history", query: { puuid: p.puuid } });
  }
}

function objectives(team: TeamDetail): string {
  const o = team.objectives;
  const parts = [
    ["大龙", o.baron],
    ["小龙", o.dragon],
    ["先锋", o.herald],
    ["虚空虫", o.grubs],
    ["阿塔坎", o.atakhan],
    ["塔", o.tower],
    ["水晶", o.inhibitor],
  ] as const;
  return parts
    .filter(([, n]) => n > 0)
    .map(([label, n]) => `${label} ${n}`)
    .join(" · ");
}

/** Rows of the stats tab; the best value in each row is highlighted. */
const STAT_ROWS: { label: string; value: (p: PlayerLine) => number; format?: (n: number) => string }[] = [
  { label: "对英雄伤害", value: (p) => p.damageToChampions, format: k },
  { label: "物理伤害", value: (p) => p.physicalDamage, format: k },
  { label: "魔法伤害", value: (p) => p.magicDamage, format: k },
  { label: "真实伤害", value: (p) => p.trueDamage, format: k },
  { label: "承受伤害", value: (p) => p.damageTaken, format: k },
  { label: "自我减伤", value: (p) => p.damageMitigated, format: k },
  { label: "治疗队友", value: (p) => p.healing, format: k },
  { label: "护盾队友", value: (p) => p.shielding, format: k },
  { label: "建筑伤害", value: (p) => p.buildingDamage, format: k },
  { label: "经济", value: (p) => p.gold, format: k },
  { label: "补刀", value: (p) => p.cs },
  { label: "视野得分", value: (p) => p.visionScore },
  { label: "插眼", value: (p) => p.wardsPlaced },
  { label: "排眼", value: (p) => p.wardsKilled },
  { label: "真眼", value: (p) => p.controlWards },
  { label: "控制时长(秒)", value: (p) => p.ccSeconds },
  { label: "双杀", value: (p) => p.multiKills[0] ?? 0 },
  { label: "三杀", value: (p) => p.multiKills[1] ?? 0 },
  { label: "四杀", value: (p) => p.multiKills[2] ?? 0 },
  { label: "五杀", value: (p) => p.multiKills[3] ?? 0 },
];

function rowMax(row: (typeof STAT_ROWS)[number]): number {
  return Math.max(...players.value.map(row.value));
}

const SKILL_KEYS = ["", "Q", "W", "E", "R"];

function buildOf(p: PlayerLine): PlayerBuild | undefined {
  return builds.value?.find((b) => b.puuid === p.puuid);
}

/** Purchases grouped by the minute they happened in. */
function purchaseGroups(build: PlayerBuild): { minute: number; items: number[] }[] {
  const groups: { minute: number; items: number[] }[] = [];
  for (const purchase of build.items) {
    const minute = Math.floor(purchase.at / 60);
    const last = groups[groups.length - 1];
    if (last && last.minute === minute) last.items.push(purchase.itemId);
    else groups.push({ minute, items: [purchase.itemId] });
  }
  return groups;
}
</script>

<template>
  <div class="border-t border-white/[0.06] bg-zinc-950/40 px-3 py-3" @click.stop>
    <div class="segmented mb-3">
      <button
        v-for="t in TABS"
        :key="t.id"
        class="segment"
        :class="tab === t.id && 'segment-active'"
        @click="tab = t.id"
      >
        {{ t.label }}
      </button>
      <span v-if="detail" class="ml-auto text-xs text-zinc-600">
        对局 {{ detail.gameId }} · 数据来源 {{ detail.source.toUpperCase() }}
      </span>
    </div>

    <Transition name="fade" mode="out-in">
    <p v-if="error" class="text-xs text-red-400">{{ error }}</p>
    <p v-else-if="!detail" class="text-xs text-zinc-500">加载对局详情…</p>

    <!-- 概要 -->
    <div v-else-if="tab === 'overview'" class="flex flex-col gap-3">
      <div v-for="team in detail.teams" :key="team.teamId">
        <div class="mb-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs">
          <span :class="team.win ? 'text-emerald-400' : 'text-red-400'" class="font-medium">
            {{ team.win ? "胜利" : "失败" }}
          </span>
          <span class="text-zinc-500">{{ team.teamId === 100 ? "蓝方" : "红方" }}</span>
          <span class="text-zinc-400">击杀 {{ team.kills }} · 经济 {{ k(team.gold) }}</span>
          <span v-if="objectives(team)" class="text-zinc-500">{{ objectives(team) }}</span>
          <span v-if="team.bans.length" class="ml-auto flex items-center gap-1 text-zinc-500">
            禁用
            <img
              v-for="(ban, i) in team.bans"
              :key="i"
              :src="gd.championIcon(ban)"
              :title="gd.championName(ban)"
              class="size-5 rounded opacity-70 grayscale"
            />
          </span>
        </div>
        <table class="w-full table-fixed text-xs">
          <colgroup>
            <col class="w-9" />
            <col class="w-10" />
            <col />
            <col class="w-24" />
            <col class="w-12" />
            <col class="w-32" />
            <col class="w-16" />
            <col class="w-14" />
            <col class="w-16" />
            <col class="w-12" />
            <col class="w-44" />
          </colgroup>
          <tbody>
            <tr
              v-for="p in team.players"
              :key="p.puuid || p.championId"
              class="border-t border-white/[0.04]"
              :class="p.puuid === puuid && 'bg-amber-500/10'"
            >
              <td class="py-1">
                <div class="relative w-fit">
                  <img
                    :src="gd.championIcon(p.championId)"
                    :title="gd.championName(p.championId)"
                    class="size-7 rounded bg-zinc-800"
                  />
                  <span class="absolute -right-1 -bottom-1 rounded bg-zinc-950 px-0.5 text-[9px]">
                    {{ p.champLevel }}
                  </span>
                </div>
              </td>
              <td>
                <div class="grid w-fit grid-cols-2 gap-0.5">
                  <img
                    v-for="(spell, i) in p.spells"
                    :key="'s' + i"
                    :src="gd.spellIcon(spell)"
                    class="size-3.5 rounded bg-zinc-800"
                  />
                  <img
                    :src="gd.perkIcon(p.runes.perks[0] ?? 0)"
                    :title="gd.perkName(p.runes.perks[0] ?? 0)"
                    class="size-3.5 rounded-full bg-zinc-800"
                  />
                  <img
                    :src="gd.perkIcon(p.runes.subStyle)"
                    :title="gd.perkName(p.runes.subStyle)"
                    class="size-3.5"
                  />
                </div>
              </td>
              <td class="truncate pr-2">
                <button
                  class="max-w-full truncate text-left hover:text-amber-300"
                  :class="p.puuid === puuid ? 'text-amber-200' : 'text-zinc-300'"
                  :title="name(p)"
                  @click="openPlayer(p)"
                >
                  {{ name(p) }}
                </button>
              </td>
              <td class="text-center tabular-nums">
                {{ p.kills }}/{{ p.deaths }}/{{ p.assists }}
                <span class="text-zinc-500">({{ kda(p) }})</span>
              </td>
              <td class="text-right text-zinc-400 tabular-nums" title="参团率">{{ kp(p, team) }}</td>
              <td class="pl-3">
                <div class="flex items-center gap-1" :title="`对英雄伤害 ${p.damageToChampions}`">
                  <div class="h-1.5 flex-1 overflow-hidden rounded-full bg-white/[0.06]">
                    <div
                      class="h-1.5 rounded-full bg-linear-to-r from-rose-500 to-orange-400"
                      :style="{ width: `${(p.damageToChampions / maxDamage) * 100}%` }"
                    />
                  </div>
                  <span class="w-10 text-right text-zinc-400 tabular-nums">{{ k(p.damageToChampions) }}</span>
                </div>
              </td>
              <td class="text-right text-zinc-500 tabular-nums" title="承受伤害">{{ k(p.damageTaken) }}</td>
              <td class="text-right text-zinc-500 tabular-nums" title="经济">{{ k(p.gold) }}</td>
              <td class="text-right text-zinc-500 tabular-nums" title="补刀（每分钟）">
                {{ p.cs }} <span class="text-zinc-600">{{ (p.cs / minutes).toFixed(1) }}</span>
              </td>
              <td class="text-right text-zinc-500 tabular-nums" title="视野得分">{{ p.visionScore }}</td>
              <td class="pl-2">
                <div class="flex justify-end gap-0.5">
                  <template v-for="(item, i) in p.items" :key="i">
                    <img v-if="gd.itemIcon(item)" :src="gd.itemIcon(item)" class="size-5 rounded bg-zinc-800" />
                    <div v-else class="size-5 rounded bg-zinc-900" />
                  </template>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- 数据 -->
    <div v-else-if="tab === 'stats'" class="overflow-x-auto">
      <table class="w-full table-fixed text-xs tabular-nums">
        <colgroup>
          <col class="w-24" />
          <col v-for="p in players" :key="p.puuid || p.championId" />
        </colgroup>
        <thead>
          <tr>
            <th />
            <th v-for="(p, i) in players" :key="i" class="pb-1">
              <img
                :src="gd.championIcon(p.championId)"
                :title="name(p)"
                class="mx-auto size-7 rounded"
                :class="[
                  p.puuid === puuid && 'ring-2 ring-amber-400',
                  i === 5 && 'ml-2',
                ]"
              />
            </th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in STAT_ROWS" :key="row.label" class="border-t border-white/[0.04]">
            <td class="py-1 text-zinc-500">{{ row.label }}</td>
            <td
              v-for="(p, i) in players"
              :key="i"
              class="text-center"
              :class="[
                row.value(p) > 0 && row.value(p) === rowMax(row) ? 'font-medium text-amber-300' : 'text-zinc-400',
                i < 5 ? 'bg-sky-950/20' : 'bg-red-950/20',
              ]"
            >
              {{ row.format ? row.format(row.value(p)) : row.value(p) }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- 出装加点 -->
    <div v-else-if="tab === 'builds'">
      <p v-if="buildsError" class="text-xs text-red-400">{{ buildsError }}</p>
      <p v-else-if="!builds" class="text-xs text-zinc-500">读取对局时间线…</p>
      <div v-else class="flex flex-col gap-2">
        <div
          v-for="p in players"
          :key="p.puuid"
          class="grid grid-cols-[8rem_1fr] items-start gap-2 border-t border-white/[0.04] pt-2 text-xs"
          :class="p.puuid === puuid && 'rounded-md bg-amber-500/10'"
        >
          <div class="flex items-center gap-1.5">
            <img :src="gd.championIcon(p.championId)" class="size-7 rounded bg-zinc-800" />
            <span class="truncate text-zinc-300" :title="name(p)">{{ p.gameName || "未知玩家" }}</span>
          </div>
          <div v-if="buildOf(p)" class="flex flex-col gap-1.5">
            <div class="flex flex-wrap gap-x-2 gap-y-1">
              <div v-for="(group, gi) in purchaseGroups(buildOf(p)!)" :key="gi" class="flex flex-col items-center">
                <div class="flex gap-0.5">
                  <img
                    v-for="(item, ii) in group.items"
                    :key="ii"
                    :src="gd.itemIcon(item)"
                    class="size-6 rounded bg-zinc-800"
                  />
                </div>
                <span class="text-[10px] text-zinc-500">{{ group.minute }} 分</span>
              </div>
            </div>
            <div class="flex gap-0.5">
              <span
                v-for="(slot, level) in buildOf(p)!.skills"
                :key="level"
                class="w-5 rounded text-center text-[11px] leading-5"
                :class="slot === 4 ? 'bg-amber-600 text-white' : 'bg-zinc-800 text-zinc-300'"
                :title="`${level + 1} 级`"
              >
                {{ SKILL_KEYS[slot] }}
              </span>
            </div>
          </div>
          <span v-else class="text-zinc-500">没有该玩家的时间线</span>
        </div>
      </div>
    </div>

    <!-- 符文 -->
    <div v-else class="grid grid-cols-2 gap-x-4 gap-y-1.5">
      <div
        v-for="p in players"
        :key="p.puuid || p.championId"
        class="flex items-center gap-2 text-xs"
        :class="p.puuid === puuid && 'rounded-md bg-amber-500/10'"
      >
        <img :src="gd.championIcon(p.championId)" class="size-7 rounded bg-zinc-800" />
        <span class="w-20 truncate text-zinc-300" :title="name(p)">{{ p.gameName || "未知玩家" }}</span>
        <img :src="gd.perkIcon(p.runes.primaryStyle)" class="size-4" :title="gd.perkName(p.runes.primaryStyle)" />
        <img
          v-for="(perk, i) in p.runes.perks"
          :key="i"
          :src="gd.perkIcon(perk)"
          :title="gd.perkName(perk)"
          class="rounded-full bg-zinc-800"
          :class="[i === 0 ? 'size-7' : 'size-5', i === 4 && 'ml-1.5']"
        />
        <img
          v-for="(shard, i) in p.runes.shards"
          :key="'shard' + i"
          :src="gd.perkIcon(shard)"
          :title="gd.perkName(shard)"
          class="size-4 rounded-full bg-zinc-800"
          :class="i === 0 && 'ml-1.5'"
        />
      </div>
    </div>
    </Transition>
  </div>
</template>
