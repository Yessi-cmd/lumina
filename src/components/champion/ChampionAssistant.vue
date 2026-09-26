<script setup lang="ts">
import { computed, ref, shallowRef, watch } from "vue";
import {
  api,
  type ChampionBuild,
  type Matchup,
  type MatchupReport,
  type TierEntry,
  type Verdict,
} from "../../api";
import { useGameDataStore } from "../../stores/gameData";

const props = defineProps<{
  /** Assigned position (TOP / JUNGLE / MIDDLE / BOTTOM / UTILITY); empty outside role queues. */
  position: string;
  /** The local player's hovered or locked champion; 0 when none. */
  championId: number;
  /** Champions the opponents have locked so far. */
  enemyChampions: number[];
}>();
const gd = useGameDataStore();

const POSITIONS = [
  { id: "TOP", label: "上单" },
  { id: "JUNGLE", label: "打野" },
  { id: "MIDDLE", label: "中单" },
  { id: "BOTTOM", label: "下路" },
  { id: "UTILITY", label: "辅助" },
];
const TIER_CLASS: Record<string, string> = {
  S: "bg-rose-600 text-white",
  A: "bg-orange-500 text-white",
  B: "bg-amber-500/80 text-zinc-950",
  C: "bg-zinc-600 text-white",
  D: "bg-zinc-700 text-zinc-300",
};

const lane = ref(props.position || "MIDDLE");
const ownedOnly = ref(true);
const tierList = shallowRef<TierEntry[] | null>(null);
const listError = ref<string | null>(null);

const selected = ref(0);
const build = shallowRef<ChampionBuild | null>(null);
const buildError = ref<string | null>(null);
const variantIndex = ref(0);
const matchups = shallowRef<MatchupReport | null>(null);
/** Opponent chosen by hand; 0 means infer it from the enemy picks. */
const pinnedOpponent = ref(0);
const opponentQuery = ref("");
const matchupError = ref<string | null>(null);
const applying = ref(false);
const applyMessage = ref<string | null>(null);

watch(
  () => props.position,
  (p) => {
    if (p) lane.value = p;
  },
);
// Follow the player's own hover / lock until they pick something from the list.
watch(
  () => props.championId,
  (id) => {
    if (id > 0) selected.value = id;
  },
  { immediate: true },
);

watch(
  lane,
  async (position) => {
    tierList.value = null;
    listError.value = null;
    try {
      const list = await api.championTierList(position);
      if (position === lane.value) tierList.value = list;
    } catch (err) {
      if (position === lane.value) listError.value = String(err);
    }
  },
  { immediate: true },
);

watch(
  () => [selected.value, lane.value] as const,
  async ([championId, position]) => {
    build.value = null;
    buildError.value = null;
    applyMessage.value = null;
    variantIndex.value = 0;
    if (championId <= 0) return;
    try {
      const b = await api.championBuild(championId, position);
      if (championId === selected.value && position === lane.value) build.value = b;
    } catch (err) {
      if (championId === selected.value) buildError.value = String(err);
    }
  },
  { immediate: true },
);

watch(
  () => [selected.value, lane.value, props.enemyChampions.join(",")] as const,
  async ([championId, position]) => {
    matchups.value = null;
    matchupError.value = null;
    if (championId <= 0) return;
    try {
      const r = await api.championMatchups(championId, position, props.enemyChampions);
      if (championId === selected.value && position === lane.value) matchups.value = r;
    } catch (err) {
      if (championId === selected.value) matchupError.value = String(err);
    }
  },
  { immediate: true },
);

const VERDICT: Record<Verdict, { label: string; class: string }> = {
  counters: { label: "克制", class: "text-emerald-400" },
  countered: { label: "被克制", class: "text-red-400" },
  even: { label: "均势", class: "text-zinc-300" },
  tooFewGames: { label: "样本不足", class: "text-zinc-500" },
};
const TIER_NAMES: Record<string, string> = {
  emerald_plus: "翡翠+",
  diamond_plus: "钻石+",
  master_plus: "大师+",
};

function signed(value: number): string {
  return (value > 0 ? "+" : "") + value.toFixed(1);
}

function matchupTitle(m: Matchup): string {
  const name = gd.championName(m.championId);
  const rate = m.winRate.toFixed(1) + "%";
  return (
    name + "：对位胜率 " + rate + "，" + m.games + " 场。" +
    "扣除双方整体强度后优势 " + signed(m.advantage) + "，误差 ±" + m.margin.toFixed(1) + "。"
  );
}

/** An enemy pick that usually plays our lane, when there is exactly one. */
const inferredOpponent = computed(() => {
  const sameLane = matchups.value?.againstPicks.filter((m) => m.usualPosition === lane.value) ?? [];
  return sameLane.length === 1 ? sameLane[0].championId : 0;
});
const opponentId = computed(() => pinnedOpponent.value || inferredOpponent.value);
const opponentMatchup = computed(
  () => matchups.value?.all.find((m) => m.championId === opponentId.value) ?? null,
);
const opponentResults = computed(() => {
  const query = opponentQuery.value.trim();
  const champions = gd.data?.champions ?? {};
  if (!query) return [];
  return Object.entries(champions)
    .filter(([, c]) => c.name.includes(query) || c.alias.toLowerCase().includes(query.toLowerCase()))
    .slice(0, 8)
    .map(([id]) => Number(id));
});

function positionLabel(position: string): string {
  return POSITIONS.find((p) => p.id === position)?.label ?? "未知";
}

function pinOpponent(championId: number) {
  pinnedOpponent.value = championId;
  opponentQuery.value = "";
}

const visible = computed(() => {
  const list = tierList.value ?? [];
  const anyOwned = list.some((e) => e.owned);
  const filtered = ownedOnly.value && anyOwned ? list.filter((e) => e.owned) : list;
  return filtered.slice(0, 20);
});
const variant = computed(() => build.value?.variants[variantIndex.value] ?? null);
const laneLabel = computed(() => POSITIONS.find((p) => p.id === lane.value)?.label ?? "");

function tierClass(tier: string): string {
  return TIER_CLASS[tier.charAt(0)] ?? TIER_CLASS.D;
}

async function apply() {
  const v = variant.value;
  if (!v || !build.value) return;
  applying.value = true;
  applyMessage.value = null;
  try {
    const title = `${gd.championName(build.value.championId)} ${laneLabel.value}`;
    await api.applyBuild(title, v);
    applyMessage.value = "已应用符文和召唤师技能";
  } catch (err) {
    applyMessage.value = `应用失败：${String(err)}`;
  } finally {
    applying.value = false;
  }
}
</script>

<template>
  <div class="card flex flex-col gap-3 p-4">
    <div class="flex items-center gap-2">
      <h2 class="text-sm font-medium text-amber-300">选英雄</h2>
      <div class="segmented ml-auto">
        <button
          v-for="p in POSITIONS"
          :key="p.id"
          class="segment"
          :class="lane === p.id && 'bg-amber-500! text-zinc-950! shadow-sm'"
          @click="lane = p.id"
        >
          {{ p.label }}
        </button>
      </div>
    </div>

    <!-- Build of the selected champion -->
    <div v-if="selected > 0" class="rounded-lg border border-white/[0.05] bg-zinc-950/50 p-3">
      <div class="flex items-center gap-2">
        <img :src="gd.championIcon(selected)" class="size-10 rounded-lg bg-zinc-800 ring-1 ring-white/10" />
        <div class="min-w-0">
          <div class="text-sm font-medium">{{ gd.championName(selected) }} · {{ laneLabel }}</div>
          <div v-if="build" class="text-xs text-zinc-400">
            <span class="rounded px-1 font-medium" :class="tierClass(build.tier)">{{ build.tier }}</span>
            胜率 {{ build.winRate.toFixed(1) }}% · 选取 {{ build.pickRate.toFixed(1) }}% · 禁用
            {{ build.banRate.toFixed(1) }}% · 排名 {{ build.rank }}/{{ build.rankTotal }}
          </div>
        </div>
      </div>

      <p v-if="buildError" class="mt-2 text-xs text-red-400">{{ buildError }}</p>
      <p v-else-if="!build" class="mt-2 text-xs text-zinc-500">加载出装数据…</p>
      <template v-else-if="variant">
        <div class="segmented mt-2.5">
          <button
            v-for="(v, i) in build.variants"
            :key="v.label"
            class="segment"
            :class="variantIndex === i && 'segment-active'"
            @click="variantIndex = i"
          >
            {{ v.label }}
          </button>
        </div>

        <div class="mt-2 grid grid-cols-[3.5rem_1fr] items-center gap-y-1.5 text-xs">
          <span class="text-zinc-500">符文</span>
          <div class="flex flex-wrap items-center gap-1">
            <img
              v-for="(perk, i) in variant.runes.perks"
              :key="i"
              :src="gd.perkIcon(perk)"
              v-tip="gd.perkTip(perk)"
              class="rounded-full bg-zinc-800"
              :class="i === 0 ? 'size-7' : i < 6 ? 'size-5' : 'size-4'"
            />
            <span class="ml-1 text-zinc-500">
              胜率 {{ variant.runeWinRate.toFixed(1) }}%
            </span>
          </div>

          <span class="text-zinc-500">召唤师</span>
          <div class="flex gap-1">
            <img v-for="s in variant.spells" :key="s" :src="gd.spellIcon(s)" class="size-5 rounded" />
          </div>

          <span class="text-zinc-500">加点</span>
          <span class="text-zinc-300">{{ variant.skillPriority }}（前 6 级 {{ variant.skillOrder }}）</span>

          <span class="text-zinc-500">出门装</span>
          <div class="flex gap-1">
            <img v-for="(item, i) in variant.startItems" :key="i" :src="gd.itemIcon(item)" class="size-6 rounded" />
          </div>

          <span class="text-zinc-500">核心装</span>
          <div class="flex gap-1">
            <img v-for="(item, i) in variant.coreItems" :key="i" :src="gd.itemIcon(item)" class="size-6 rounded" />
          </div>
        </div>

        <div class="mt-2.5 flex items-center gap-2">
          <button
            class="btn btn-primary py-1 text-xs"
            :disabled="applying"
            @click="apply"
          >
            {{ applying ? "应用中…" : "应用符文和召唤师技能" }}
          </button>
          <span v-if="applyMessage" class="text-xs text-zinc-400">{{ applyMessage }}</span>
        </div>
      </template>
    </div>

    <!-- Matchups from high-rank games -->
    <div v-if="selected > 0" class="rounded-lg border border-white/[0.05] bg-zinc-950/50 p-3 text-xs">
      <div class="mb-1.5 flex items-center gap-2 text-zinc-500">
        <span>对位克制（{{ TIER_NAMES[matchups?.tier ?? ""] ?? "高分段" }}，同位置）</span>
        <span class="ml-auto" title="扣除双方英雄整体强度后的胜率差；超出误差范围才判定克制">
          优势 = 归一化胜率差
        </span>
      </div>
      <p v-if="matchupError" class="text-red-400">{{ matchupError }}</p>
      <p v-else-if="!matchups" class="text-zinc-500">加载对位数据…</p>
      <template v-else>
        <!-- The lane opponent: picked by hand, or the one enemy pick that usually plays this lane -->
        <div class="mb-2 rounded-lg border border-white/[0.06] p-2.5">
          <div class="flex items-center gap-2">
            <span class="text-zinc-400">我的对位</span>
            <span v-if="pinnedOpponent" class="text-zinc-600">手动指定</span>
            <span v-else-if="inferredOpponent" class="text-zinc-600">按敌方已选推断</span>
            <button
              v-if="pinnedOpponent"
              class="ml-auto text-zinc-500 hover:text-zinc-300"
              @click="pinnedOpponent = 0"
            >
              清除
            </button>
          </div>

          <div v-if="opponentId" class="mt-1.5 flex items-center gap-2">
            <img :src="gd.championIcon(opponentId)" class="size-8 rounded bg-zinc-800" />
            <div class="min-w-0">
              <div class="text-sm">{{ gd.championName(opponentId) }}</div>
              <div v-if="opponentMatchup" v-tip="matchupTitle(opponentMatchup)">
                <span class="font-medium" :class="VERDICT[opponentMatchup.verdict].class">
                  {{ VERDICT[opponentMatchup.verdict].label }}
                </span>
                <span class="ml-1.5 text-zinc-300 tabular-nums">{{ signed(opponentMatchup.advantage) }}</span>
                <span class="ml-1 text-zinc-500">
                  ±{{ opponentMatchup.margin.toFixed(1) }} · 对位胜率 {{ opponentMatchup.winRate.toFixed(1) }}% ·
                  {{ opponentMatchup.games }} 场
                </span>
              </div>
              <div v-else class="text-zinc-500">这个对手在{{ laneLabel }}几乎没有对位数据</div>
            </div>
          </div>
          <p v-else class="mt-1 text-zinc-500">
            点下方敌方英雄，或搜索任意英雄来指定对位（对面摇摆时手动选）。
          </p>

          <input
            v-model="opponentQuery"
            placeholder="搜索英雄，指定对位…"
            class="field mt-2 w-full py-1 text-xs"
          />
          <div v-if="opponentResults.length" class="mt-1 flex flex-wrap gap-1">
            <button
              v-for="id in opponentResults"
              :key="id"
              class="flex items-center gap-1 rounded bg-zinc-800 px-1.5 py-0.5 hover:bg-zinc-700"
              @click="pinOpponent(id)"
            >
              <img :src="gd.championIcon(id)" class="size-4 rounded" />
              {{ gd.championName(id) }}
            </button>
          </div>
        </div>

        <div v-if="matchups.againstPicks.length" class="mb-2 flex flex-col gap-1">
          <div class="text-zinc-400">对上敌方已选（点击设为我的对位）</div>
          <button
            v-for="m in matchups.againstPicks"
            :key="m.championId"
            class="flex items-center gap-2 rounded px-1 py-0.5 text-left hover:bg-white/[0.05]"
            :class="[
              m.championId === opponentId && 'bg-zinc-800',
              m.usualPosition !== lane && 'opacity-60',
            ]"
            v-tip="matchupTitle(m)"
            @click="pinOpponent(m.championId)"
          >
            <img :src="gd.championIcon(m.championId)" class="size-6 rounded bg-zinc-800" />
            <span class="w-20 truncate">{{ gd.championName(m.championId) }}</span>
            <span class="w-14 font-medium" :class="VERDICT[m.verdict].class">
              {{ VERDICT[m.verdict].label }}
            </span>
            <span class="text-zinc-300 tabular-nums">{{ signed(m.advantage) }}</span>
            <span class="text-zinc-500">±{{ m.margin.toFixed(1) }} · {{ m.games }} 场</span>
            <span v-if="m.usualPosition !== lane" class="ml-auto text-amber-500/80">
              常走{{ positionLabel(m.usualPosition) }}
            </span>
          </button>
        </div>
        <div class="grid grid-cols-[3.5rem_1fr] items-center gap-y-1.5">
          <span class="text-zinc-500">克制</span>
          <div class="flex flex-wrap gap-1">
            <img
              v-for="m in matchups.best"
              :key="m.championId"
              :src="gd.championIcon(m.championId)"
              v-tip="matchupTitle(m)"
              class="size-6 rounded ring-1 ring-emerald-600/60"
            />
            <span v-if="!matchups.best.length" class="text-zinc-500">没有明显克制的对位</span>
          </div>
          <span class="text-zinc-500">被克制</span>
          <div class="flex flex-wrap gap-1">
            <img
              v-for="m in matchups.worst"
              :key="m.championId"
              :src="gd.championIcon(m.championId)"
              v-tip="matchupTitle(m)"
              class="size-6 rounded ring-1 ring-red-600/60"
            />
            <span v-if="!matchups.worst.length" class="text-zinc-500">没有明显被克制的对位</span>
          </div>
        </div>
      </template>
    </div>

    <!-- Tier list for the lane -->
    <div>
      <div class="mb-1 flex items-center gap-2 text-xs text-zinc-500">
        <span>{{ laneLabel }}英雄强度（lolalytics）</span>
        <label class="ml-auto flex cursor-pointer items-center gap-1">
          <input v-model="ownedOnly" type="checkbox" class="accent-amber-500" />
          只看已拥有
        </label>
      </div>
      <p v-if="listError" class="text-xs text-red-400">{{ listError }}</p>
      <p v-else-if="!tierList" class="text-xs text-zinc-500">加载中…</p>
      <div v-else class="grid grid-cols-2 gap-1">
        <button
          v-for="e in visible"
          :key="e.championId"
          class="flex items-center gap-1.5 rounded px-1.5 py-1 text-left text-xs hover:bg-white/[0.05]"
          :class="selected === e.championId && 'bg-white/[0.07]'"
          @click="selected = e.championId"
        >
          <img :src="gd.championIcon(e.championId)" class="size-6 rounded bg-zinc-800" />
          <span class="min-w-0 flex-1 truncate">{{ gd.championName(e.championId) }}</span>
          <span class="w-6 rounded text-center font-medium" :class="tierClass(e.tier)">{{ e.tier }}</span>
          <span class="w-11 text-right text-zinc-400 tabular-nums">{{ e.winRate.toFixed(1) }}%</span>
        </button>
      </div>
    </div>

    <p class="text-[11px] leading-4 text-zinc-600">
      数据来自 lolalytics（外服排位，近 30 天），仅供参考。
    </p>
  </div>
</template>
