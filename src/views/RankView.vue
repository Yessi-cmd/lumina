<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, shallowRef, watch } from "vue";
import { api, events, type RankHistory, type RankQueue } from "../api";
import RankChart from "../components/rank/RankChart.vue";
import { useGameDataStore } from "../stores/gameData";
import { useLcuStore } from "../stores/lcu";
import { timeAgo } from "../utils/format";
import { TIERS, deltaColor, signed, summarize, tierText } from "../utils/rank";

const lcu = useLcuStore();
const gd = useGameDataStore();

const QUEUES: { id: RankQueue; label: string }[] = [
  { id: "solo", label: "单双排" },
  { id: "flex", label: "灵活排位" },
];
/** How many recent points the chart shows; 0 is all of them. */
const WINDOWS = [
  { size: 30, label: "近 30 次" },
  { size: 100, label: "近 100 次" },
  { size: 0, label: "全部" },
];
const CHAMPIONS_SHOWN = 8;
const RECENT_SHOWN = 20;

const history = shallowRef<RankHistory | null>(null);
const loaded = ref(false);
const error = ref<string | null>(null);
const queue = ref<RankQueue>("solo");
const windowSize = ref(30);
let picked = false;
let disposed = false;
let unlisten: (() => void) | null = null;

async function load() {
  try {
    history.value = await api.rankHistory(lcu.snapshot.summoner?.puuid ?? null);
    error.value = null;
  } catch (err) {
    error.value = String(err);
  } finally {
    loaded.value = true;
  }
  // Someone who only plays flex should not land on an empty solo tab.
  const all = history.value?.points ?? [];
  if (!picked && !all.some((p) => p.queue === "solo") && all.some((p) => p.queue === "flex")) {
    queue.value = "flex";
  }
}

function pick(id: RankQueue) {
  picked = true;
  queue.value = id;
}

onMounted(async () => {
  const stop = await events.onRankUpdated(() => load());
  if (disposed) stop();
  else unlisten = stop;
  await load();
});
onUnmounted(() => {
  disposed = true;
  unlisten?.();
});
watch(() => lcu.snapshot.summoner?.puuid, load);

const points = computed(() => (history.value?.points ?? []).filter((p) => p.queue === queue.value));
const shown = computed(() => (windowSize.value ? points.value.slice(-windowSize.value) : points.value));
const current = computed(() => points.value[points.value.length - 1] ?? null);
const summary = computed(() => summarize(points.value, Date.now()));
const winRate = computed(() => {
  const c = current.value;
  return c && c.wins + c.losses > 0 ? Math.round((c.wins / (c.wins + c.losses)) * 100) : null;
});
const champions = computed(() => summary.value.champions.slice(0, CHAMPIONS_SHOWN));
const recent = computed(() => points.value.slice(-RECENT_SHOWN).reverse());

const streakText = computed(() => {
  const run = summary.value.currentStreak;
  if (run === 0) return "-";
  return run > 0 ? `${run} 连胜` : `${-run} 连败`;
});

function average(value: number | null): string {
  return value === null ? "-" : signed(Math.round(value));
}

function whole(value: number | null): number {
  return value === null ? 0 : Math.round(value);
}
</script>

<template>
  <section class="stagger flex max-w-4xl flex-col gap-5">
    <header class="page-header">
      <div class="eyebrow">Rank</div>
      <h1 class="page-title mt-1">段位</h1>
    </header>

    <p v-if="error" class="text-sm break-all text-red-400">{{ error }}</p>

    <div class="flex flex-wrap items-center gap-3">
      <div class="segmented">
        <button
          v-for="q in QUEUES"
          :key="q.id"
          class="segment"
          :class="queue === q.id && 'segment-active'"
          @click="pick(q.id)"
        >
          {{ q.label }}
        </button>
      </div>
      <span v-if="history" class="text-xs text-zinc-500">{{ history.name }}</span>
    </div>

    <div v-if="loaded && !current" class="card p-6 text-sm text-zinc-400">
      <p>这个队列还没有记录。</p>
      <p class="mt-1">
        Lumina 会在客户端连接时记下段位，之后每打完一局排位就记录一次胜点变化，从现在开始积累。
        已定级的队列才会有记录。
      </p>
    </div>

    <template v-if="current">
      <div class="card overflow-hidden p-6">
        <div
          class="pointer-events-none absolute -top-24 -right-16 size-64 rounded-full bg-[radial-gradient(closest-side,rgb(245_158_11/0.16),transparent)]"
        />
        <div class="relative flex flex-wrap items-end justify-between gap-4">
          <div>
            <div class="text-xs text-zinc-500">当前段位</div>
            <div class="mt-1 text-3xl font-semibold" :class="TIERS[current.tier]?.color ?? 'text-zinc-400'">
              {{ tierText(current.tier, current.division) }}
              <span class="text-lg font-normal text-zinc-300 tabular-nums">{{ current.lp }} 点</span>
            </div>
            <div class="mt-1 text-sm text-zinc-400 tabular-nums">
              {{ current.wins }} 胜 {{ current.losses }} 负
              <template v-if="winRate !== null"> · 胜率 {{ winRate }}%</template>
            </div>
          </div>
          <div class="flex gap-6">
            <div>
              <div class="text-xs text-zinc-500">今日</div>
              <div class="mt-1 text-2xl font-semibold tabular-nums" :class="deltaColor(summary.today)">
                {{ signed(summary.today) }}
              </div>
            </div>
            <div>
              <div class="text-xs text-zinc-500">近 7 天</div>
              <div class="mt-1 text-2xl font-semibold tabular-nums" :class="deltaColor(summary.week)">
                {{ signed(summary.week) }}
              </div>
            </div>
            <div v-tip="'从开始记录起，所有胜点变化相加'">
              <div class="text-xs text-zinc-500">累计</div>
              <div class="mt-1 text-2xl font-semibold tabular-nums" :class="deltaColor(summary.total)">
                {{ signed(summary.total) }}
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="card p-5">
        <div class="mb-4 flex items-center justify-between">
          <h2 class="eyebrow">胜点走势</h2>
          <div class="segmented">
            <button
              v-for="w in WINDOWS"
              :key="w.size"
              class="segment"
              :class="windowSize === w.size && 'segment-active'"
              @click="windowSize = w.size"
            >
              {{ w.label }}
            </button>
          </div>
        </div>
        <RankChart v-if="shown.length > 1" :points="shown" />
        <p v-else class="text-sm text-zinc-400">再打几局排位，这里就会画出走势。</p>
      </div>

      <div class="grid grid-cols-2 gap-3 lg:grid-cols-4">
        <div class="card p-4">
          <div class="text-xs text-zinc-500">记录的对局</div>
          <div class="mt-2 text-lg font-semibold text-zinc-50 tabular-nums">
            {{ summary.wins }} 胜 {{ summary.losses }} 负
          </div>
        </div>
        <div v-tip="'胜利时平均加多少点，失败时平均扣多少点'" class="card p-4">
          <div class="text-xs text-zinc-500">平均每胜 / 每负</div>
          <div class="mt-2 text-lg font-semibold tabular-nums">
            <span :class="deltaColor(whole(summary.avgWin))">{{ average(summary.avgWin) }}</span>
            <span class="text-zinc-600"> / </span>
            <span :class="deltaColor(whole(summary.avgLoss))">{{ average(summary.avgLoss) }}</span>
          </div>
        </div>
        <div class="card p-4">
          <div class="text-xs text-zinc-500">最长连胜 / 连败</div>
          <div class="mt-2 text-lg font-semibold tabular-nums">
            <span class="text-emerald-400">{{ summary.longestWinStreak }}</span>
            <span class="text-zinc-600"> / </span>
            <span class="text-red-400">{{ summary.longestLossStreak }}</span>
          </div>
        </div>
        <div class="card p-4">
          <div class="text-xs text-zinc-500">当前</div>
          <div
            class="mt-2 text-lg font-semibold"
            :class="summary.currentStreak > 0 ? 'text-emerald-400' : summary.currentStreak < 0 ? 'text-red-400' : 'text-zinc-400'"
          >
            {{ streakText }}
          </div>
        </div>
      </div>

      <div v-if="champions.length" class="card overflow-hidden">
        <h2 class="eyebrow px-5 pt-4 pb-2">英雄加分 / 掉分</h2>
        <table class="w-full text-sm">
          <thead class="text-xs text-zinc-500">
            <tr>
              <th class="px-5 py-1.5 text-left font-normal">英雄</th>
              <th class="px-3 py-1.5 text-right font-normal">场次</th>
              <th class="px-3 py-1.5 text-right font-normal">胜率</th>
              <th class="px-3 py-1.5 text-right font-normal">场均</th>
              <th class="px-5 py-1.5 text-right font-normal">净胜点</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-white/[0.04]">
            <tr v-for="c in champions" :key="c.championId" class="transition-colors hover:bg-white/[0.02]">
              <td class="px-5 py-2">
                <div class="flex items-center gap-2.5">
                  <img :src="gd.championIcon(c.championId)" class="size-7 rounded-md bg-zinc-800" />
                  <span class="text-zinc-100">{{ gd.championName(c.championId) }}</span>
                </div>
              </td>
              <td class="px-3 py-2 text-right text-zinc-300 tabular-nums">{{ c.games }}</td>
              <td class="px-3 py-2 text-right text-zinc-300 tabular-nums">
                {{ Math.round((c.wins / c.games) * 100) }}%
              </td>
              <td class="px-3 py-2 text-right tabular-nums" :class="deltaColor(c.net)">
                {{ signed(Math.round(c.net / c.games)) }}
              </td>
              <td class="px-5 py-2 text-right font-medium tabular-nums" :class="deltaColor(c.net)">
                {{ signed(c.net) }}
              </td>
            </tr>
          </tbody>
        </table>
        <p class="px-5 py-3 text-xs text-zinc-500">
          只统计装上了对局信息的记录。场次少的英雄，净胜点更多是运气而不是英雄本身。
        </p>
      </div>

      <div class="card overflow-hidden">
        <h2 class="eyebrow px-5 pt-4 pb-2">最近记录</h2>
        <ul class="divide-y divide-white/[0.04]">
          <li v-for="p in recent" :key="p.at" class="flex items-center gap-3 px-5 py-2.5 text-sm">
            <img
              v-if="p.championId !== null"
              :src="gd.championIcon(p.championId)"
              class="size-8 rounded-md bg-zinc-800"
            />
            <div v-else class="size-8 rounded-md bg-zinc-800/60" />
            <div class="w-12 shrink-0 font-medium" :class="p.outcome === 'win' ? 'text-emerald-400' : p.outcome === 'loss' ? 'text-red-400' : 'text-zinc-400'">
              {{ p.outcome === "win" ? "胜利" : p.outcome === "loss" ? "失败" : "变动" }}
            </div>
            <div class="min-w-0 flex-1 truncate text-zinc-400">
              <template v-if="p.championId !== null">{{ gd.championName(p.championId) }}</template>
              <template v-if="p.kda"> · {{ p.kda[0] }}/{{ p.kda[1] }}/{{ p.kda[2] }}</template>
            </div>
            <div class="w-14 text-right font-semibold tabular-nums" :class="p.delta === null ? 'text-zinc-500' : deltaColor(p.delta)">
              {{ p.delta === null ? "起点" : signed(p.delta) }}
            </div>
            <div class="w-28 text-right text-zinc-400 tabular-nums">
              {{ tierText(p.tier, p.division) }} {{ p.lp }}
            </div>
            <div class="w-20 text-right text-xs text-zinc-500">{{ timeAgo(p.at) }}</div>
          </li>
        </ul>
      </div>
    </template>
  </section>
</template>
