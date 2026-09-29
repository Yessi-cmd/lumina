<script setup lang="ts">
import { computed, ref } from "vue";
import type { AutoSelect, Preset } from "../api";
import ChampionPicker from "../components/champion/ChampionPicker.vue";
import ToggleSwitch from "../components/common/ToggleSwitch.vue";
import { useGameDataStore } from "../stores/gameData";
import { useSettingsStore } from "../stores/settings";

const store = useSettingsStore();
const gd = useGameDataStore();

/** Same limit as the backend keeps per list. */
const MAX_PER_LIST = 10;
const POSITIONS = [
  { id: "ANY", label: "通用" },
  { id: "TOP", label: "上单" },
  { id: "JUNGLE", label: "打野" },
  { id: "MIDDLE", label: "中单" },
  { id: "BOTTOM", label: "下路" },
  { id: "UTILITY", label: "辅助" },
];
type Kind = keyof Preset;
const LISTS: { kind: Kind; title: string; hint: string }[] = [
  { kind: "bans", title: "禁用", hint: "轮到你禁用时，按顺序取第一个可以禁的" },
  { kind: "picks", title: "选择", hint: "轮到你选人时，按顺序取第一个可以选的" },
];

const position = ref("MIDDLE");
const adding = ref<Kind | null>(null);

const auto = computed<AutoSelect | null>(() => store.settings?.autoSelect ?? null);

function presetOf(id: string): Preset {
  return auto.value?.presets[id] ?? { picks: [], bans: [] };
}
const current = computed(() => presetOf(position.value));

function save(patch: Partial<AutoSelect>) {
  if (auto.value) store.update({ autoSelect: { ...auto.value, ...patch } });
}

function setList(kind: Kind, ids: number[]) {
  if (!auto.value) return;
  const presets = { ...auto.value.presets, [position.value]: { ...current.value, [kind]: ids } };
  save({ presets });
}

function add(kind: Kind, id: number) {
  const ids = current.value[kind];
  if (ids.length >= MAX_PER_LIST) return;
  setList(kind, [...ids, id]);
  if (ids.length + 1 >= MAX_PER_LIST) adding.value = null;
}

function remove(kind: Kind, index: number) {
  setList(
    kind,
    current.value[kind].filter((_, i) => i !== index),
  );
}

function move(kind: Kind, index: number, step: -1 | 1) {
  const ids = [...current.value[kind]];
  const target = index + step;
  if (target < 0 || target >= ids.length) return;
  [ids[index], ids[target]] = [ids[target], ids[index]];
  setList(kind, ids);
}

function selectPosition(id: string) {
  position.value = id;
  adding.value = null;
}

function count(id: string): number {
  const preset = presetOf(id);
  return preset.bans.length + preset.picks.length;
}

function onDelayInput(event: Event) {
  save({ delaySecs: Number((event.target as HTMLInputElement).value) });
}
</script>

<template>
  <section class="stagger flex max-w-4xl flex-col gap-5">
    <header class="page-header">
      <div class="eyebrow">Auto Ban &amp; Pick</div>
      <h1 class="page-title mt-1">自动 BP</h1>
    </header>

    <p v-if="!auto" class="text-sm text-zinc-400">加载中…</p>

    <template v-else>
      <div class="card divide-y divide-white/[0.05] overflow-hidden">
        <div class="flex items-center justify-between gap-6 p-4">
          <div>
            <div class="font-medium text-zinc-100">自动禁用</div>
            <div class="mt-0.5 text-sm text-zinc-400">
              轮到你禁用时，按下面的预设禁用。不会禁用你自己或队友正在预选、已经表态想玩的英雄。
            </div>
          </div>
          <ToggleSwitch :model-value="auto.ban" @update:model-value="save({ ban: $event })" />
        </div>
        <div class="flex items-center justify-between gap-6 p-4">
          <div>
            <div class="font-medium text-zinc-100">自动选择</div>
            <div class="mt-0.5 text-sm text-zinc-400">
              轮到你选人时，按下面的预设选择。如果你已经先点了某个英雄，就不会替你换。
            </div>
          </div>
          <ToggleSwitch :model-value="auto.pick" @update:model-value="save({ pick: $event })" />
        </div>
        <div class="flex items-center gap-4 p-4" :class="!auto.ban && !auto.pick && 'opacity-50'">
          <span class="w-20 shrink-0 text-sm text-zinc-300">锁定延迟</span>
          <input
            type="range"
            min="0"
            max="10"
            step="1"
            class="flex-1 cursor-pointer accent-amber-500"
            :value="auto.delaySecs"
            :style="{ '--fill': `${auto.delaySecs * 10}%` }"
            @change="onDelayInput"
          />
          <span
            class="w-14 rounded-md bg-amber-400/10 py-0.5 text-center text-sm font-medium text-amber-200 tabular-nums ring-1 ring-amber-400/20 ring-inset"
          >
            {{ auto.delaySecs }} 秒
          </span>
        </div>
        <p class="px-4 py-3 text-xs text-zinc-500">
          先亮起（预选）英雄，等上面的延迟之后再锁定。这几秒里你手动换了英雄，就不会再锁定。
        </p>
      </div>

      <div>
        <h2 class="eyebrow mb-2 px-1">按分路的预设</h2>
        <div class="segmented mb-3 flex-wrap">
          <button
            v-for="p in POSITIONS"
            :key="p.id"
            class="segment"
            :class="position === p.id && 'segment-active'"
            @click="selectPosition(p.id)"
          >
            {{ p.label }}
            <span v-if="count(p.id)" class="ml-1 text-[10px] text-amber-300 tabular-nums">{{ count(p.id) }}</span>
          </button>
        </div>
        <p class="mb-3 px-1 text-xs text-zinc-500">
          分路以客户端分配的位置为准。该分路的预设用完（都被禁、被选或没有）后，再用「通用」里的预设；位置不明时（如匹配模式）只用「通用」。
        </p>

        <div class="grid grid-cols-1 gap-4 md:grid-cols-2">
          <div v-for="list in LISTS" :key="list.kind" class="card flex flex-col gap-3 p-4">
            <div>
              <div class="text-sm font-semibold text-zinc-100">{{ list.title }}</div>
              <div class="mt-0.5 text-xs text-zinc-500">{{ list.hint }}</div>
            </div>

            <ol class="flex flex-col gap-1.5">
              <li
                v-for="(id, index) in current[list.kind]"
                :key="id"
                class="flex items-center gap-2.5 rounded-lg bg-white/[0.03] px-2.5 py-1.5"
              >
                <span class="w-4 text-center text-xs text-zinc-500 tabular-nums">{{ index + 1 }}</span>
                <img :src="gd.championIcon(id)" class="size-8 rounded-md bg-zinc-800" />
                <span class="min-w-0 flex-1 truncate text-sm text-zinc-100">{{ gd.championName(id) }}</span>
                <button
                  class="rounded px-1.5 text-zinc-400 hover:bg-white/10 hover:text-zinc-100 disabled:opacity-30"
                  :disabled="index === 0"
                  title="上移"
                  @click="move(list.kind, index, -1)"
                >
                  ↑
                </button>
                <button
                  class="rounded px-1.5 text-zinc-400 hover:bg-white/10 hover:text-zinc-100 disabled:opacity-30"
                  :disabled="index === current[list.kind].length - 1"
                  title="下移"
                  @click="move(list.kind, index, 1)"
                >
                  ↓
                </button>
                <button
                  class="rounded px-1.5 text-zinc-400 hover:bg-red-500/20 hover:text-red-300"
                  title="移除"
                  @click="remove(list.kind, index)"
                >
                  ✕
                </button>
              </li>
              <li v-if="!current[list.kind].length" class="px-1 text-sm text-zinc-500">还没有添加英雄。</li>
            </ol>

            <ChampionPicker
              v-if="adding === list.kind"
              :exclude="current[list.kind]"
              @pick="add(list.kind, $event)"
              @close="adding = null"
            />
            <button
              v-else
              class="btn btn-secondary self-start"
              :disabled="current[list.kind].length >= MAX_PER_LIST"
              @click="adding = list.kind"
            >
              添加英雄{{ current[list.kind].length >= MAX_PER_LIST ? `（最多 ${MAX_PER_LIST} 个）` : "" }}
            </button>
          </div>
        </div>
      </div>
      <p v-if="store.saveError" class="px-1 text-sm text-red-400">{{ store.saveError }}</p>
    </template>
  </section>
</template>
