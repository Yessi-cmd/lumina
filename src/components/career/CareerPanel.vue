<script setup lang="ts">
import { computed, ref, shallowRef, watch } from "vue";
import { api, type Career, type CareerRange, type Rates } from "../../api";
import { useGameDataStore } from "../../stores/gameData";
import RadarChart from "./RadarChart.vue";
import TrendChart from "./TrendChart.vue";

const props = defineProps<{ puuid: string }>();
const gd = useGameDataStore();

const RANGES: { id: CareerRange; label: string; hint: string }[] = [
  { id: "recent", label: "近期", hint: "最近 20 场" },
  { id: "season", label: "本赛季", hint: "今年以来，最多 400 场" },
  { id: "career", label: "生涯", hint: "战绩能查到的全部对局，最多 400 场" },
];
const MODES: { id: number | null; label: string }[] = [
  { id: null, label: "综合" },
  { id: 420, label: "单双排" },
  { id: 440, label: "灵活排位" },
  { id: 490, label: "快速匹配" },
  { id: 430, label: "匹配（自选）" },
  { id: 450, label: "极地大乱斗" },
];
const TIERS: Record<string, { name: string; color: string }> = {
  IRON: { name: "黑铁", color: "text-zinc-400" },
  BRONZE: { name: "青铜", color: "text-orange-300" },
  SILVER: { name: "白银", color: "text-slate-300" },
  GOLD: { name: "黄金", color: "text-amber-300" },
  PLATINUM: { name: "铂金", color: "text-teal-300" },
  EMERALD: { name: "翡翠", color: "text-emerald-300" },
  DIAMOND: { name: "钻石", color: "text-sky-300" },
  MASTER: { name: "大师", color: "text-fuchsia-300" },
  GRANDMASTER: { name: "宗师", color: "text-red-300" },
  CHALLENGER: { name: "王者", color: "text-yellow-200" },
};
const POSITIONS: Record<string, string> = {
  TOP: "上单",
  JUNGLE: "打野",
  MIDDLE: "中单",
  BOTTOM: "下路",
  UTILITY: "辅助",
};

const range = ref<CareerRange>("recent");
const mode = ref<number | null>(null);
const career = shallowRef<Career | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);
let generation = 0;

watch(
  () => [props.puuid, range.value, mode.value] as const,
  async ([puuid, r, m]) => {
    const gen = ++generation;
    loading.value = true;
    error.value = null;
    try {
      const result = await api.career(puuid, m, r);
      if (gen === generation) career.value = result;
    } catch (err) {
      if (gen === generation) error.value = String(err);
    } finally {
      if (gen === generation) loading.value = false;
    }
  },
  { immediate: true },
);

const c = computed(() => career.value);
const winRate = computed(() => (c.value && c.value.games ? c.value.wins / c.value.games : 0));

function pct(v: number): string {
  return `${Math.round(v * 100)}%`;
}

function kdaText(k: number, d: number, a: number): string {
  return ((k + a) / Math.max(1, d)).toFixed(2);
}

function tierText(tier: string, division: string): string {
  if (!tier) return "未定级";
  const name = TIERS[tier]?.name ?? tier;
  const apex = ["MASTER", "GRANDMASTER", "CHALLENGER"].includes(tier);
  return apex || !division || division === "NA" ? name : `${name} ${division}`;
}

function points(p: number): string {
  return p >= 10000 ? `${(p / 10000).toFixed(1)}万` : String(p);
}

const referenceLabel = computed(() => (c.value?.reference === "opponent" ? "对位平均" : "同局平均"));

/** What each radar axis measures, in the reader's words. */
const AXIS_UNITS: Record<string, (v: number) => string> = {
  damage: (v) => `每分钟伤害 ${Math.round(v)}`,
  tanking: (v) => `每分钟承伤 ${Math.round(v)}`,
  economy: (v) => `每分钟经济 ${Math.round(v)}`,
  teamfight: (v) => `参团率 ${pct(v)}`,
  vision: (v) => `每分钟视野 ${v.toFixed(2)}`,
  survival: (v) => `每 10 分钟死亡 ${v.toFixed(1)} 次`,
};

const radarAxes = computed(() =>
  (c.value?.radar ?? []).map((a) => {
    const unit = AXIS_UNITS[a.key] ?? ((v: number) => v.toFixed(1));
    return {
      label: a.label,
      score: a.score,
      tip: `我：${unit(a.me)}\n${referenceLabel.value}：${unit(a.reference)}`,
    };
  }),
);

/** Side-by-side numbers under the radar. */
const COMPARE: { label: string; value: (r: Rates) => number; format: (v: number) => string; lowerIsBetter?: boolean }[] = [
  { label: "分均伤害", value: (r) => r.damage, format: (v) => Math.round(v).toString() },
  { label: "分均经济", value: (r) => r.gold, format: (v) => Math.round(v).toString() },
  { label: "分均补刀", value: (r) => r.cs, format: (v) => v.toFixed(1) },
  { label: "参团率", value: (r) => r.killParticipation, format: pct },
  { label: "分均视野", value: (r) => r.vision, format: (v) => v.toFixed(2) },
  { label: "10 分钟死亡", value: (r) => r.deaths, format: (v) => v.toFixed(1), lowerIsBetter: true },
];

function diff(row: (typeof COMPARE)[number], me: Rates, ref: Rates): number {
  const theirs = row.value(ref);
  if (theirs <= 0) return 0;
  const d = (row.value(me) / theirs - 1) * 100;
  return row.lowerIsBetter ? -d : d;
}

const reference = computed(() => (c.value ? (c.value.opponents ?? c.value.peers) : null));
const maxPositionGames = computed(() => Math.max(1, ...(c.value?.positions.map((p) => p.games) ?? [])));
</script>

<template>
  <div class="flex flex-col gap-4">
    <!-- Filters: one row above everything. -->
    <div class="flex flex-wrap items-center gap-3">
      <div class="segmented">
        <button
          v-for="r in RANGES"
          :key="r.id"
          v-tip="r.hint"
          class="segment"
          :class="range === r.id && 'segment-active'"
          @click="range = r.id"
        >
          {{ r.label }}
        </button>
      </div>
      <div class="segmented">
        <button
          v-for="m in MODES"
          :key="String(m.id)"
          class="segment"
          :class="mode === m.id && 'segment-active'"
          @click="mode = m.id"
        >
          {{ m.label }}
        </button>
      </div>
      <span v-if="loading" class="flex items-center gap-2 text-xs text-zinc-500">
        <span class="size-3 animate-spin rounded-full border-2 border-zinc-600 border-t-amber-400" />
        {{ range === "recent" ? "读取中…" : "正在翻阅战绩，最多 400 场，可能需要几秒…" }}
      </span>
    </div>

    <p v-if="error" class="text-sm break-all text-red-400">{{ error }}</p>

    <div v-if="!c && loading" class="grid grid-cols-3 gap-3">
      <div v-for="i in 6" :key="i" class="skeleton h-24 rounded-xl" />
    </div>

    <Transition name="fade" mode="out-in">
      <div v-if="c" :key="`${range}-${mode}`" class="flex flex-col gap-4" :class="loading && 'opacity-60'">
        <p v-if="c.games === 0" class="empty-state">这个范围里没有对局。</p>
        <template v-else>
          <!-- Headline numbers -->
          <div class="grid grid-cols-3 gap-3 lg:grid-cols-6">
            <div class="card p-3">
              <div class="eyebrow">场次</div>
              <div class="mt-1 text-2xl font-semibold text-zinc-50 tabular-nums">{{ c.games }}</div>
              <div class="text-xs text-zinc-500 tabular-nums">{{ c.wins }} 胜 {{ c.games - c.wins }} 负</div>
            </div>
            <div class="card p-3">
              <div class="eyebrow">胜率</div>
              <div
                class="mt-1 text-2xl font-semibold tabular-nums"
                :class="winRate >= 0.5 ? 'text-emerald-300' : 'text-red-300'"
              >
                {{ pct(winRate) }}
              </div>
              <div class="mt-1.5 h-1 overflow-hidden rounded-full bg-white/[0.06]">
                <div
                  class="h-1 rounded-full bg-emerald-500 transition-[width] duration-700 ease-out-expo"
                  :style="{ width: pct(winRate) }"
                />
              </div>
            </div>
            <div class="card p-3">
              <div class="eyebrow">KDA</div>
              <div class="mt-1 text-2xl font-semibold text-zinc-50 tabular-nums">
                {{ kdaText(c.avgKills, c.avgDeaths, c.avgAssists) }}
              </div>
              <div class="text-xs text-zinc-500 tabular-nums">
                {{ c.avgKills.toFixed(1) }} / {{ c.avgDeaths.toFixed(1) }} / {{ c.avgAssists.toFixed(1) }}
              </div>
            </div>
            <div class="card p-3">
              <div class="eyebrow">分均伤害</div>
              <div class="mt-1 text-2xl font-semibold text-zinc-50 tabular-nums">{{ Math.round(c.me.damage) }}</div>
              <div class="text-xs text-zinc-500">对英雄</div>
            </div>
            <div class="card p-3">
              <div class="eyebrow">分均补刀</div>
              <div class="mt-1 text-2xl font-semibold text-zinc-50 tabular-nums">{{ c.me.cs.toFixed(1) }}</div>
              <div class="text-xs text-zinc-500">平均 {{ c.avgMinutes.toFixed(0) }} 分钟一局</div>
            </div>
            <div class="card p-3">
              <div class="eyebrow">参团率</div>
              <div class="mt-1 text-2xl font-semibold text-zinc-50 tabular-nums">{{ pct(c.me.killParticipation) }}</div>
              <div class="text-xs text-zinc-500">分均视野 {{ c.me.vision.toFixed(2) }}</div>
            </div>
          </div>
          <p v-if="c.truncated" class="-mt-2 text-xs text-zinc-500">只统计了最近 400 场，更早的对局没有读取。</p>

          <!-- Ranked -->
          <div v-if="c.ranked.length" class="grid grid-cols-2 gap-3">
            <div v-for="q in c.ranked" :key="q.queue" class="card flex items-center gap-4 p-4">
              <div class="min-w-0 flex-1">
                <div class="eyebrow">{{ q.queue === "solo" ? "单双排" : "灵活排位" }}</div>
                <div class="mt-1 text-xl font-semibold" :class="TIERS[q.tier]?.color ?? 'text-zinc-400'">
                  {{ tierText(q.tier, q.division) }}
                  <span v-if="q.tier" class="text-sm font-normal text-zinc-400 tabular-nums">{{ q.leaguePoints }} 胜点</span>
                </div>
                <div v-if="q.wins + q.losses > 0" class="mt-0.5 text-xs text-zinc-500 tabular-nums">
                  本赛季 {{ q.wins }} 胜 {{ q.losses }} 负 · 胜率 {{ pct(q.wins / (q.wins + q.losses)) }}
                </div>
              </div>
              <div class="shrink-0 text-right text-xs leading-5">
                <div class="text-zinc-500">
                  最高
                  <span :class="TIERS[q.highestTier]?.color ?? 'text-zinc-400'">
                    {{ tierText(q.highestTier, q.highestDivision) }}
                  </span>
                </div>
                <div class="text-zinc-500">
                  上赛季
                  <span :class="TIERS[q.previousTier]?.color ?? 'text-zinc-400'">
                    {{ tierText(q.previousTier, q.previousDivision) }}
                  </span>
                </div>
              </div>
            </div>
          </div>

          <!-- Radar and comparison -->
          <div v-if="radarAxes.length && reference" class="card grid grid-cols-[minmax(0,1fr)_minmax(0,1.1fr)] gap-6 p-5">
            <div>
              <div class="text-sm font-semibold text-zinc-100">能力雷达</div>
              <div class="mt-0.5 text-xs text-zinc-500">
                和{{ c.reference === "opponent" ? "同位置对手" : "同局其他玩家" }}比：排位匹配段位相近，这就是同段位对比。
                50 = 持平，100 = 两倍。
              </div>
              <RadarChart class="mt-3" :axes="radarAxes" :reference-label="referenceLabel" />
            </div>
            <div>
              <div class="text-sm font-semibold text-zinc-100">同段位对比</div>
              <div class="mt-0.5 text-xs text-zinc-500">基于 {{ c.comparedGames }} 场有完整数据的对局</div>
              <table class="mt-3 w-full text-sm tabular-nums">
                <thead>
                  <tr class="text-[11px] text-zinc-500">
                    <th class="pb-1.5 text-left font-medium">指标</th>
                    <th class="pb-1.5 text-right font-medium">我</th>
                    <th class="pb-1.5 text-right font-medium">{{ referenceLabel }}</th>
                    <th class="w-28 pb-1.5 text-right font-medium">差距</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="row in COMPARE" :key="row.label" class="border-t border-white/[0.04]">
                    <td class="py-2 text-zinc-400">{{ row.label }}</td>
                    <td class="py-2 text-right text-zinc-100">{{ row.format(row.value(c.me)) }}</td>
                    <td class="py-2 text-right text-zinc-400">{{ row.format(row.value(reference)) }}</td>
                    <td class="py-2 text-right">
                      <span
                        class="rounded-md px-1.5 py-0.5 text-xs font-medium"
                        :class="
                          diff(row, c.me, reference) >= 0
                            ? 'bg-emerald-400/10 text-emerald-300'
                            : 'bg-red-400/10 text-red-300'
                        "
                      >
                        {{ diff(row, c.me, reference) >= 0 ? "▲" : "▼" }}
                        {{ Math.abs(Math.round(diff(row, c.me, reference))) }}%
                      </span>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>

          <!-- Trend -->
          <div v-if="c.trend.length >= 5" class="card p-5 pl-14">
            <div class="-ml-9 text-sm font-semibold text-zinc-100">状态走势</div>
            <div class="-ml-9 mt-0.5 mb-3 text-xs text-zinc-500">最近 {{ c.trend.length }} 场，从旧到新</div>
            <TrendChart :trend="c.trend" />
          </div>

          <div class="grid grid-cols-[minmax(0,1.6fr)_minmax(0,1fr)] gap-4">
            <!-- Champions -->
            <div class="card p-5">
              <div class="text-sm font-semibold text-zinc-100">常用英雄</div>
              <table class="mt-3 w-full text-sm tabular-nums">
                <thead>
                  <tr class="text-[11px] text-zinc-500">
                    <th class="pb-1.5 text-left font-medium">英雄</th>
                    <th class="pb-1.5 text-right font-medium">场次</th>
                    <th class="pb-1.5 pl-4 text-left font-medium">胜率</th>
                    <th class="pb-1.5 text-right font-medium">KDA</th>
                    <th class="pb-1.5 text-right font-medium">分均伤害</th>
                    <th v-tip="'英雄成就点，代表生涯玩了多少'" class="cursor-help pb-1.5 text-right font-medium">熟练度</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="ch in c.champions"
                    :key="ch.championId"
                    class="border-t border-white/[0.04] transition-colors hover:bg-white/[0.025]"
                  >
                    <td class="py-1.5">
                      <div class="flex items-center gap-2">
                        <img :src="gd.championIcon(ch.championId)" class="icon-hover size-7 rounded-lg bg-zinc-800" />
                        <span class="truncate text-zinc-200">{{ gd.championName(ch.championId) }}</span>
                      </div>
                    </td>
                    <td class="text-right text-zinc-300">{{ ch.games }}</td>
                    <td class="pl-4">
                      <div class="flex items-center gap-2">
                        <div class="h-1.5 w-16 overflow-hidden rounded-full bg-white/[0.06]">
                          <div
                            class="h-1.5 rounded-full"
                            :class="ch.wins / ch.games >= 0.5 ? 'bg-emerald-500' : 'bg-red-500/80'"
                            :style="{ width: pct(ch.wins / ch.games) }"
                          />
                        </div>
                        <span :class="ch.wins / ch.games >= 0.5 ? 'text-emerald-300' : 'text-red-300'">
                          {{ pct(ch.wins / ch.games) }}
                        </span>
                      </div>
                    </td>
                    <td class="text-right text-zinc-300">
                      {{ kdaText(ch.avgKills, ch.avgDeaths, ch.avgAssists) }}
                    </td>
                    <td class="text-right text-zinc-400">{{ Math.round(ch.damage) }}</td>
                    <td class="text-right text-zinc-500">
                      {{ ch.masteryPoints !== null ? points(ch.masteryPoints) : "-" }}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>

            <div class="flex flex-col gap-4">
              <!-- Positions -->
              <div v-if="c.positions.length" class="card p-5">
                <div class="text-sm font-semibold text-zinc-100">位置分布</div>
                <div class="mt-3 flex flex-col gap-2.5">
                  <div v-for="p in c.positions" :key="p.position" class="text-xs">
                    <div class="flex justify-between text-zinc-400">
                      <span class="text-zinc-200">{{ POSITIONS[p.position] ?? p.position }}</span>
                      <span class="tabular-nums">{{ p.games }} 场 · 胜率 {{ pct(p.wins / p.games) }}</span>
                    </div>
                    <div class="mt-1 h-1.5 overflow-hidden rounded-full bg-white/[0.06]">
                      <div
                        class="h-1.5 rounded-full bg-amber-500 transition-[width] duration-700 ease-out-expo"
                        :style="{ width: `${(p.games / maxPositionGames) * 100}%` }"
                      />
                    </div>
                  </div>
                </div>
              </div>

              <!-- Mastery: the whole career -->
              <div v-if="c.mastery.length" class="card p-5">
                <div class="text-sm font-semibold text-zinc-100">生涯熟练度</div>
                <div class="mt-0.5 text-xs text-zinc-500">所有模式、所有赛季累计</div>
                <div class="mt-3 grid grid-cols-5 gap-2">
                  <div
                    v-for="m in c.mastery"
                    :key="m.championId"
                    v-tip="{
                      title: gd.championName(m.championId),
                      subtitle: `熟练度 ${m.championLevel} 级`,
                      body: `英雄成就点 ${m.championPoints}`,
                    }"
                    class="flex flex-col items-center gap-1"
                  >
                    <img :src="gd.championIcon(m.championId)" class="icon-hover size-10 rounded-lg bg-zinc-800" />
                    <span class="text-[10px] text-zinc-400 tabular-nums">{{ points(m.championPoints) }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </template>
      </div>
    </Transition>
  </div>
</template>
