<script setup lang="ts">
import { computed, ref } from "vue";
import { useGameDataStore } from "../../stores/gameData";

/** A searchable grid of every champion; `exclude` ids are left out. */
const props = defineProps<{ exclude?: number[] }>();
const emit = defineEmits<{ pick: [championId: number]; close: [] }>();
const gd = useGameDataStore();
const query = ref("");

const champions = computed(() => {
  const table = gd.data?.champions ?? {};
  const skip = new Set(props.exclude ?? []);
  const needle = query.value.trim().toLowerCase();
  return Object.entries(table)
    .map(([id, c]) => ({ id: Number(id), name: c.name, alias: c.alias }))
    .filter((c) => c.id > 0 && !skip.has(c.id))
    .filter((c) => !needle || c.name.toLowerCase().includes(needle) || c.alias.toLowerCase().includes(needle))
    .sort((a, b) => a.name.localeCompare(b.name, "zh-CN"));
});
</script>

<template>
  <div class="rounded-xl border border-white/[0.08] bg-zinc-950/70 p-3">
    <div class="flex items-center gap-2">
      <input
        v-model="query"
        class="field min-w-0 flex-1"
        placeholder="搜索英雄名称或英文名"
        autocomplete="off"
        spellcheck="false"
        autofocus
      />
      <button class="btn btn-secondary py-1" @click="emit('close')">关闭</button>
    </div>
    <p v-if="!gd.data" class="mt-3 text-sm text-zinc-400">连接客户端后才有英雄列表。</p>
    <div v-else class="mt-3 grid max-h-64 grid-cols-[repeat(auto-fill,minmax(44px,1fr))] gap-1.5 overflow-y-auto">
      <button
        v-for="c in champions"
        :key="c.id"
        v-tip="c.name"
        class="rounded-lg p-0.5 transition-transform duration-200 hover:scale-110 hover:bg-white/10"
        @click="emit('pick', c.id)"
      >
        <img :src="gd.championIcon(c.id)" class="size-10 rounded-md bg-zinc-800" loading="lazy" />
      </button>
      <p v-if="!champions.length" class="col-span-full text-sm text-zinc-500">没有匹配的英雄。</p>
    </div>
  </div>
</template>
