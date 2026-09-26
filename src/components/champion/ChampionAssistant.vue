<script setup lang="ts">
import { computed, ref, shallowRef, watch } from "vue";
import { api, type ChampionBuild, type TierEntry } from "../../api";
import { useGameDataStore } from "../../stores/gameData";

const props = defineProps<{
  /** Assigned position (TOP / JUNGLE / MIDDLE / BOTTOM / UTILITY); empty outside role queues. */
  position: string;
  /** The local player's hovered or locked champion; 0 when none. */
  championId: number;
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
  <div class="flex flex-col gap-3 rounded-lg border border-zinc-800 bg-zinc-900 p-3">
    <div class="flex items-center gap-2">
      <h2 class="text-sm font-medium text-amber-300">选英雄</h2>
      <div class="ml-auto flex gap-1">
        <button
          v-for="p in POSITIONS"
          :key="p.id"
          class="rounded px-2 py-0.5 text-xs"
          :class="lane === p.id ? 'bg-amber-500 text-zinc-950' : 'bg-zinc-800 text-zinc-300 hover:bg-zinc-700'"
          @click="lane = p.id"
        >
          {{ p.label }}
        </button>
      </div>
    </div>

    <!-- Build of the selected champion -->
    <div v-if="selected > 0" class="rounded-md bg-zinc-950/60 p-2.5">
      <div class="flex items-center gap-2">
        <img :src="gd.championIcon(selected)" class="size-9 rounded bg-zinc-800" />
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
        <div class="mt-2 flex gap-1">
          <button
            v-for="(v, i) in build.variants"
            :key="v.label"
            class="rounded px-2 py-0.5 text-xs"
            :class="variantIndex === i ? 'bg-zinc-700 text-white' : 'text-zinc-400 hover:bg-zinc-800'"
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
              :title="gd.perkName(perk)"
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

          <span class="text-zinc-500">克制</span>
          <div class="flex gap-1">
            <img
              v-for="c in build.strongAgainst"
              :key="c"
              :src="gd.championIcon(c)"
              :title="gd.championName(c)"
              class="size-5 rounded"
            />
          </div>

          <span class="text-zinc-500">被克制</span>
          <div class="flex gap-1">
            <img
              v-for="c in build.weakAgainst"
              :key="c"
              :src="gd.championIcon(c)"
              :title="gd.championName(c)"
              class="size-5 rounded"
            />
          </div>
        </div>

        <div class="mt-2.5 flex items-center gap-2">
          <button
            class="rounded-md bg-amber-500 px-3 py-1 text-xs font-medium text-zinc-950 hover:bg-amber-400 disabled:opacity-50"
            :disabled="applying"
            @click="apply"
          >
            {{ applying ? "应用中…" : "应用符文和召唤师技能" }}
          </button>
          <span v-if="applyMessage" class="text-xs text-zinc-400">{{ applyMessage }}</span>
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
          class="flex items-center gap-1.5 rounded px-1.5 py-1 text-left text-xs hover:bg-zinc-800"
          :class="selected === e.championId && 'bg-zinc-800'"
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
