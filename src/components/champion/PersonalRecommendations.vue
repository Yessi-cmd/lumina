<script setup lang="ts">
import { computed, onUnmounted, ref, shallowRef, watch } from "vue";
import { api, type RecommendationReport, type Verdict } from "../../api";
import { useGameDataStore } from "../../stores/gameData";
import { useLcuStore } from "../../stores/lcu";
import { useOngoingStore } from "../../stores/ongoing";
import ChampionPoolEditor from "./ChampionPoolEditor.vue";
import { useChampionPool } from "./useChampionPool";

const props = defineProps<{ position: string; opponentId: number }>();
const emit = defineEmits<{ select: [championId: number]; opponent: [championId: number] }>();
const gd = useGameDataStore();
const lcu = useLcuStore();
const ongoing = useOngoingStore();
const { account, entries, saving, setPreference, store } = useChampionPool(() => props.position);
const report = shallowRef<RecommendationReport | null>(null);
const loading = ref(false);
const error = ref("");
const editing = ref(false);
let generation = 0;
let disposed = false;
let inFlight = false;
let queued = false;
let debounce: ReturnType<typeof setTimeout> | undefined;
const active = computed(() => lcu.connected && lcu.snapshot.gameflowPhase === "ChampSelect");
const opponents = computed(() => [...new Set([
  ...(ongoing.roster?.enemyChampions ?? []), props.opponentId,
])].filter((id) => id > 0));
function chooseOpponent(event: Event) {
  emit("opponent", Number((event.target as HTMLSelectElement).value));
}
const labels: Record<Verdict, string> = { counters: "对位有利", countered: "对位不利", even: "未见显著差异", tooFewGames: "样本不足" };

async function refresh() {
  if (!active.value || disposed) return;
  if (inFlight) { queued = true; return; }
  inFlight = true;
  const request = ++generation;
  loading.value = true;
  error.value = "";
  try {
    const result = await api.personalRecommendations(props.position, props.opponentId);
    if (request === generation && !disposed) report.value = result;
  } catch (err) {
    if (request === generation && !disposed) {
      report.value = null;
      error.value = String(err);
    }
  } finally {
    inFlight = false;
    if (request === generation && !disposed) loading.value = false;
    if (queued && !disposed) {
      queued = false;
      void refresh();
    }
  }
}

watch(
  () => [props.position, props.opponentId, account.value, active.value,
    JSON.stringify(entries.value), JSON.stringify(ongoing.roster),
    store.settings?.statsTier, store.settings?.matchupTier] as const,
  () => {
    generation++;
    report.value = null;
    error.value = "";
    loading.value = active.value;
    clearTimeout(debounce);
    if (active.value) debounce = setTimeout(() => void refresh(), 350);
  },
  { immediate: true },
);
// Bans can change without changing the visible roster. Recheck availability periodically.
const timer = setInterval(() => { if (!loading.value) void refresh(); }, 12000);
onUnmounted(() => { disposed = true; generation++; clearInterval(timer); clearTimeout(debounce); });
</script>

<template>
  <section class="flex flex-col gap-3 rounded-xl border border-amber-400/20 bg-amber-400/[0.035] p-3" aria-label="个人 BP 推荐" :aria-busy="loading">
    <div class="flex flex-wrap items-center gap-2">
      <h3 class="text-sm font-semibold text-amber-300">这局我选什么</h3>
      <span class="ml-auto text-[11px] text-zinc-500">熟练度优先 · 最多 3 个候选</span>
      <button class="btn-ghost text-xs" @click="editing = !editing">{{ editing ? "收起英雄池" : "管理英雄池" }}</button>
      <button class="btn-ghost text-xs" :disabled="loading || !active" @click="refresh">刷新</button>
    </div>
    <label v-if="opponents.length" class="flex items-center gap-2 text-xs text-zinc-400">
      我的对位
      <select class="field min-w-0 flex-1 py-1 text-xs" :value="opponentId" @change="chooseOpponent">
        <option :value="0">自动判断（摇摆位分别比较）</option>
        <option v-for="id in opponents" :key="id" :value="id">{{ gd.championName(id) }}</option>
      </select>
    </label>
    <ChampionPoolEditor v-if="editing" :position="position" />
    <p v-if="!active" class="text-xs text-zinc-400">进入英雄选择后，结合你的英雄池生成推荐。</p>
    <p v-if="loading" role="status" class="text-xs text-amber-200/70">正在比较你的英雄池、可选英雄与对位…</p>
    <p v-if="error" role="alert" class="text-xs text-red-300">{{ error }} <button class="btn-ghost" :disabled="loading" @click="refresh">重试</button></p>
    <template v-if="report">
      <p v-if="report.queueId" class="text-[11px] text-zinc-500">近 50 场{{ report.queueId === 440 ? "灵活排位" : "单双排" }}内，本分路有效样本 {{ report.sampleGames }} 场。胜负仅作参考，非本局胜率预测。</p>
      <p v-for="warning in report.warnings" :key="warning" class="text-xs leading-relaxed text-amber-200/70">{{ warning }}</p>
      <article v-for="candidate in report.candidates" :key="candidate.championId" class="rounded-lg border border-white/[0.07] bg-zinc-950/60 p-3">
        <div class="flex items-center gap-2">
          <img :src="gd.championIcon(candidate.championId)" alt="" class="size-9 rounded-lg" />
          <div class="flex-1">
            <div class="text-sm font-medium">{{ gd.championName(candidate.championId) }}</div>
            <span class="text-[11px] text-amber-300">{{ candidate.label }}</span>
          </div>
          <button class="btn btn-secondary px-2 py-1 text-xs" @click="emit('select', candidate.championId)">查看出装</button>
        </div>
        <ul class="mt-2 flex list-disc flex-col gap-1 pl-4 text-xs leading-relaxed text-zinc-400">
          <li v-for="reason in candidate.reasons" :key="reason">{{ reason }}</li>
        </ul>
        <div v-for="matchup in candidate.matchups" :key="matchup.championId" class="mt-2 text-xs text-zinc-400">
          对 {{ gd.championName(matchup.championId) }}：
          <span :class="matchup.verdict === 'countered' ? 'text-red-300' : matchup.verdict === 'counters' ? 'text-emerald-300' : ''">{{ labels[matchup.verdict] }}</span>
          <span class="text-zinc-500"> · {{ matchup.games }} 场 · 归一化优势 {{ matchup.advantage.toFixed(1) }} ± {{ matchup.margin.toFixed(1) }} 个百分点</span>
        </div>
        <div class="mt-2 flex gap-3 border-t border-white/5 pt-2">
          <button class="btn-ghost text-[11px]" :disabled="saving || entries.length >= 30" @click="setPreference(candidate.championId, 'familiar')">我会玩</button>
          <button class="btn-ghost text-[11px]" :disabled="saving || entries.length >= 30" @click="setPreference(candidate.championId, 'practice')">想练</button>
          <button class="btn-ghost text-[11px]" :disabled="saving || entries.length >= 30" @click="setPreference(candidate.championId, 'excluded')">不再推荐</button>
        </div>
      </article>
      <p v-if="report.candidates.length" class="text-[11px] leading-relaxed text-zinc-500">已过滤客户端不可选、已禁选和队友预选英雄。阵容标签为粗略参考；查看出装不会替你选定或锁定英雄。</p>
    </template>
    <p v-if="store.saveError && !editing" role="alert" class="text-xs text-red-300">保存失败：{{ store.saveError }}</p>
  </section>
</template>
