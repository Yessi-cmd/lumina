<script setup lang="ts">
import { computed, onMounted, onUnmounted, shallowRef } from "vue";
import { api, events, type Draft, type EnemySlot } from "../api";
import { useGameDataStore } from "../stores/gameData";
import { LANES, ROW_HEIGHT, rowTop } from "./layout";

const gd = useGameDataStore();
const draft = shallowRef<Draft | null>(null);
let unlisten: (() => void) | undefined;

onMounted(async () => {
  let gotEvent = false;
  unlisten = await events.onDraft((d) => {
    gotEvent = true;
    draft.value = d;
  });
  const current = await api.draftState();
  if (!gotEvent) draft.value = current;
});
onUnmounted(() => unlisten?.());

const enemies = computed(() => draft.value?.enemies.slice(0, 5) ?? []);

/** Worth showing: a pick or a ban says something about the lane. */
function showsLane(enemy: EnemySlot): boolean {
  return enemy.championId > 0 || enemy.bans.length > 0;
}

function percent(p: number): string {
  return `${Math.round(p * 100)}%`;
}

/** Lane guesses worth mentioning: the favourite, plus a close second. */
function laneGuesses(enemy: EnemySlot) {
  const [first, second] = enemy.lanes;
  if (!first) return [];
  return second && second.probability >= 0.25 ? [first, second] : [first];
}
</script>

<template>
  <TransitionGroup name="overlay-row" tag="div" class="relative h-full w-[240px]" appear>
    <div
      v-for="(enemy, i) in enemies"
      :key="enemy.floor"
      class="overlay-card overlay-card-right"
      :style="{ top: `${rowTop(i)}px`, height: `${ROW_HEIGHT}px`, transitionDelay: `${i * 40}ms` }"
    >
      <div class="flex items-center gap-1.5">
        <img
          v-if="enemy.championId > 0"
          :src="gd.championIcon(enemy.championId)"
          class="size-4 shrink-0 rounded-[3px] bg-zinc-800 ring-1 ring-red-400/40"
        />
        <span class="truncate font-semibold text-red-300">
          {{ enemy.floor }} 楼 ·
          {{ enemy.championId > 0 ? gd.championName(enemy.championId) : "未锁定" }}
        </span>
        <span v-if="enemy.bans.length" class="ml-auto flex shrink-0 items-center gap-0.5">
          <span class="text-[9px] text-zinc-500">禁</span>
          <img
            v-for="ban in enemy.bans"
            :key="ban"
            :src="gd.championIcon(ban)"
            class="size-3.5 rounded-[3px] bg-zinc-800 opacity-70 grayscale"
          />
        </span>
      </div>

      <div v-if="showsLane(enemy)" class="mt-0.5 flex items-center gap-1 text-[9px] text-zinc-400">
        <span>{{ enemy.championId > 0 ? "分路" : "可能走" }}</span>
        <span
          v-for="(lane, n) in laneGuesses(enemy)"
          :key="lane.position"
          class="rounded px-1 py-px"
          :class="n === 0 ? 'bg-sky-400/15 text-sky-200' : 'bg-white/5 text-zinc-400'"
        >
          {{ LANES[lane.position] }} {{ percent(lane.probability) }}
        </span>
      </div>

      <div class="mt-1 text-[10px]">
        <p v-if="enemy.advice.kind === 'waiting'" class="text-zinc-500">锁定英雄后给出 counter 建议</p>
        <template v-else-if="enemy.advice.kind === 'loading'">
          <div class="flex gap-1.5">
            <div v-for="n in 4" :key="n" class="skeleton rounded size-[22px]" />
          </div>
        </template>
        <p v-else-if="enemy.advice.kind === 'noPicksLeft'" class="text-zinc-400">
          我方已全部锁定，无法再 counter
        </p>
        <p v-else-if="enemy.advice.kind === 'unavailable'" class="text-zinc-500">
          {{ enemy.advice.reason }}
        </p>
        <div v-else-if="enemy.advice.kind === 'allyLocked'" class="flex items-center gap-1.5">
          <span class="text-zinc-400">我方已锁</span>
          <img
            :src="gd.championIcon(enemy.advice.allyChampionId)"
            class="size-[18px] rounded-[3px] bg-zinc-800 ring-1 ring-sky-400/40"
          />
          <span class="truncate text-zinc-200">{{ gd.championName(enemy.advice.allyChampionId) }}</span>
          <span
            v-if="enemy.advice.winRate !== null"
            class="ml-auto tabular-nums"
            :class="enemy.advice.winRate >= 50 ? 'text-emerald-300' : 'text-red-300'"
          >
            对位 {{ enemy.advice.winRate.toFixed(1) }}%
          </span>
          <span v-else class="ml-auto text-zinc-500">对位样本不足</span>
        </div>
        <template v-else-if="enemy.advice.kind === 'counters'">
          <p v-if="enemy.advice.picks.length === 0" class="text-zinc-500">没有明显的 counter，看个人熟练度</p>
          <div v-else class="flex items-start gap-1.5">
            <div
              v-for="pick in enemy.advice.picks"
              :key="pick.championId"
              class="flex w-[40px] flex-col items-center gap-0.5"
              :class="!pick.significant && 'opacity-60'"
              :title="`${gd.championName(pick.championId)}：对位胜率 ${pick.winRate.toFixed(1)}%，${pick.games} 场`"
            >
              <img
                :src="gd.championIcon(pick.championId)"
                class="size-[22px] rounded-[4px] bg-zinc-800"
                :class="pick.significant && 'ring-1 ring-emerald-400/60'"
              />
              <span class="text-[9px] leading-none text-emerald-300 tabular-nums">
                {{ pick.winRate.toFixed(1) }}%
              </span>
            </div>
            <span v-if="enemy.advice.allyFloor" class="ml-auto self-center text-[9px] text-zinc-500">
              给 {{ enemy.advice.allyFloor }} 楼
            </span>
          </div>
        </template>
      </div>
    </div>
  </TransitionGroup>
</template>
