<script setup lang="ts">
import { computed, shallowRef, watch } from "vue";
import { useRouter } from "vue-router";
import { api, type GameDetail, type PlayerLine } from "../../api";
import { useGameDataStore } from "../../stores/gameData";

const props = defineProps<{
  gameId: number;
  /** Player whose history this is; their row is highlighted. */
  puuid: string;
}>();
const gd = useGameDataStore();
const router = useRouter();

const detail = shallowRef<GameDetail | null>(null);
const error = shallowRef<string | null>(null);

watch(
  () => props.gameId,
  async (gameId) => {
    detail.value = null;
    error.value = null;
    try {
      const d = await api.gameDetail(gameId);
      if (gameId === props.gameId) detail.value = d;
    } catch (err) {
      if (gameId === props.gameId) error.value = String(err);
    }
  },
  { immediate: true },
);

/** Largest champion damage in the game, for the damage bars. */
const maxDamage = computed(() => {
  const all = detail.value?.teams.flatMap((t) => t.players) ?? [];
  return Math.max(1, ...all.map((p) => p.damageToChampions));
});

function name(p: PlayerLine): string {
  if (!p.gameName) return "未知玩家";
  return p.tagLine ? `${p.gameName}#${p.tagLine}` : p.gameName;
}

function kda(p: PlayerLine): string {
  const ratio = (p.kills + p.assists) / Math.max(1, p.deaths);
  return ratio.toFixed(1);
}

function openPlayer(p: PlayerLine) {
  if (p.puuid && p.puuid !== props.puuid) {
    router.push({ path: "/match-history", query: { puuid: p.puuid } });
  }
}
</script>

<template>
  <div class="rounded-b-md border-t border-zinc-800 bg-zinc-950/60 px-3 py-2">
    <p v-if="error" class="text-xs text-red-400">{{ error }}</p>
    <p v-else-if="!detail" class="text-xs text-zinc-500">加载对局详情…</p>
    <div v-else class="flex flex-col gap-3">
      <div v-for="team in detail.teams" :key="team.teamId">
        <div class="mb-1 flex items-center gap-3 text-xs">
          <span :class="team.win ? 'text-emerald-400' : 'text-red-400'" class="font-medium">
            {{ team.win ? "胜利" : "失败" }}
          </span>
          <span class="text-zinc-500">{{ team.teamId === 100 ? "蓝方" : "红方" }}</span>
          <span class="text-zinc-500">击杀 {{ team.kills }} · 经济 {{ (team.gold / 1000).toFixed(1) }}k</span>
        </div>
        <table class="w-full text-xs">
          <tbody>
            <tr
              v-for="p in team.players"
              :key="p.puuid || p.championId"
              class="border-t border-zinc-900"
              :class="p.puuid === puuid && 'bg-amber-500/10'"
            >
              <td class="w-8 py-1">
                <img
                  :src="gd.championIcon(p.championId)"
                  :title="gd.championName(p.championId)"
                  class="size-7 rounded bg-zinc-800"
                />
              </td>
              <td class="w-5">
                <div class="flex flex-col gap-0.5">
                  <template v-for="(spell, i) in p.spells" :key="i">
                    <img v-if="gd.spellIcon(spell)" :src="gd.spellIcon(spell)" class="size-3.5 rounded" />
                  </template>
                </div>
              </td>
              <td class="max-w-40 truncate pr-2">
                <button
                  class="truncate text-left hover:text-amber-300"
                  :class="p.puuid === puuid ? 'text-amber-200' : 'text-zinc-300'"
                  :title="name(p)"
                  @click.stop="openPlayer(p)"
                >
                  {{ name(p) }}
                </button>
              </td>
              <td class="w-24 text-center tabular-nums">
                {{ p.kills }}/{{ p.deaths }}/{{ p.assists }}
                <span class="text-zinc-500">({{ kda(p) }})</span>
              </td>
              <td class="w-28">
                <div class="flex items-center gap-1" :title="`对英雄伤害 ${p.damageToChampions}`">
                  <div class="h-1.5 flex-1 rounded bg-zinc-800">
                    <div
                      class="h-1.5 rounded bg-rose-500/80"
                      :style="{ width: `${(p.damageToChampions / maxDamage) * 100}%` }"
                    />
                  </div>
                  <span class="w-9 text-right text-zinc-400 tabular-nums">
                    {{ (p.damageToChampions / 1000).toFixed(1) }}k
                  </span>
                </div>
              </td>
              <td class="w-16 text-right text-zinc-500 tabular-nums" :title="`承伤 ${p.damageTaken}`">
                承 {{ (p.damageTaken / 1000).toFixed(1) }}k
              </td>
              <td class="w-14 text-right text-zinc-500 tabular-nums">刀 {{ p.cs }}</td>
              <td class="w-12 text-right text-zinc-500 tabular-nums">眼 {{ p.visionScore }}</td>
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
  </div>
</template>
