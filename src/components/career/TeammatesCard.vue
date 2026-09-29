<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { api, type CareerRange, type Teammates } from "../../api";

const props = defineProps<{ puuid: string; queue: number | null; range: CareerRange }>();

const data = ref<Teammates | null>(null);
const error = ref<string | null>(null);
let generation = 0;

watch(
  () => [props.puuid, props.queue, props.range] as const,
  async ([puuid, queue, range]) => {
    const gen = ++generation;
    try {
      const found = await api.teammates(puuid, queue, range);
      if (gen !== generation) return;
      data.value = found;
      error.value = null;
    } catch (err) {
      if (gen === generation) error.value = String(err);
    }
  },
  { immediate: true },
);

function rate(wins: number, games: number): number {
  return games ? Math.round((wins / games) * 100) : 0;
}

function tone(percent: number): string {
  if (percent >= 55) return "bg-emerald-500";
  if (percent <= 45) return "bg-red-500";
  return "bg-amber-500";
}

const withRate = computed(() => rate(data.value?.withRegulars.wins ?? 0, data.value?.withRegulars.games ?? 0));
const withoutRate = computed(() =>
  rate(data.value?.withoutRegulars.wins ?? 0, data.value?.withoutRegulars.games ?? 0),
);
</script>

<template>
  <div v-if="data || error" class="card p-5">
    <div class="text-sm font-semibold text-zinc-100">常一起玩的人</div>
    <div class="mt-0.5 text-xs text-zinc-500">同队至少 3 场才会列出；不含重开、斗魂竞技场</div>

    <p v-if="error" class="mt-3 text-sm break-all text-red-400">{{ error }}</p>
    <p v-else-if="data && data.games === 0" class="mt-3 text-sm text-zinc-400">
      这些对局里没有队友信息（只有从 SGP 取到的战绩才带队友）。
    </p>
    <p v-else-if="data && !data.regulars.length" class="mt-3 text-sm text-zinc-400">
      这 {{ data.games }} 场里没有反复遇到的队友。
    </p>

    <template v-else-if="data">
      <div class="mt-3 flex flex-col gap-2.5">
        <div v-for="t in data.regulars" :key="t.puuid">
          <div class="flex items-center justify-between text-xs text-zinc-400">
            <span class="truncate text-zinc-200">{{ t.name || "未知玩家" }}</span>
            <span class="shrink-0 tabular-nums">{{ t.games }} 场 · 胜率 {{ rate(t.wins, t.games) }}%</span>
          </div>
          <div class="mt-1 h-1.5 overflow-hidden rounded-full bg-white/[0.06]">
            <div
              class="h-1.5 rounded-full transition-[width] duration-700 ease-out-expo"
              :class="tone(rate(t.wins, t.games))"
              :style="{ width: `${rate(t.wins, t.games)}%` }"
            />
          </div>
        </div>
      </div>
      <div class="mt-4 grid grid-cols-2 gap-3 text-sm">
        <div class="rounded-lg bg-white/[0.03] p-3">
          <div class="text-xs text-zinc-500">和他们任意一人同队</div>
          <div class="mt-1 font-semibold text-zinc-100 tabular-nums">
            {{ data.withRegulars.games }} 场 · 胜率 {{ withRate }}%
          </div>
        </div>
        <div class="rounded-lg bg-white/[0.03] p-3">
          <div class="text-xs text-zinc-500">其余对局</div>
          <div class="mt-1 font-semibold text-zinc-100 tabular-nums">
            {{ data.withoutRegulars.games }} 场 · 胜率 {{ withoutRate }}%
          </div>
        </div>
      </div>
    </template>
  </div>
</template>
