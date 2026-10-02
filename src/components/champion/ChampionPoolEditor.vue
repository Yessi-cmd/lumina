<script setup lang="ts">
import { ref, watch } from "vue";
import type { PoolPreference } from "../../api";
import { useGameDataStore } from "../../stores/gameData";
import ChampionPicker from "./ChampionPicker.vue";
import { useChampionPool } from "./useChampionPool";

const props = defineProps<{ position: string }>();
const gd = useGameDataStore();
const { account, entries, saving, setPreference, store } = useChampionPool(() => props.position);
const adding = ref(false);
const choices: { value: PoolPreference; label: string }[] = [
  { value: "familiar", label: "会玩" },
  { value: "practice", label: "想练" },
  { value: "excluded", label: "不推荐" },
];
watch(() => [props.position, account.value], () => { adding.value = false; });
function change(id: number, event: Event) {
  void setPreference(id, (event.target as HTMLSelectElement).value as PoolPreference);
}
</script>

<template>
  <div class="flex flex-col gap-2 rounded-lg border border-white/10 bg-zinc-950/40 p-3">
    <div class="flex items-center justify-between gap-2">
      <h3 class="text-sm font-medium text-zinc-200">我的英雄池</h3>
      <button class="btn-ghost text-xs" :disabled="!account || !store.settings || saving || entries.length >= 30" @click="adding = !adding">
        添加会玩的英雄
      </button>
    </div>
    <p class="text-xs leading-relaxed text-zinc-500">按当前账号与分路保存。未标记的英雄仍会参考近期战绩；小号或久未玩的英雄可以手动补充。</p>
    <p v-if="!account" class="text-xs text-zinc-400">连接并登录客户端后管理英雄池。</p>
    <p v-else-if="!entries.length" class="text-xs text-zinc-400">还没有手动标记。先添加你的拿手英雄，也可以在选人时从推荐卡标记。</p>
    <p v-if="entries.length >= 30" class="text-xs text-amber-300">本分路已达 30 个标记，移除后可继续添加。</p>
    <div v-for="entry in entries" :key="entry.championId" class="flex items-center gap-2">
      <img :src="gd.championIcon(entry.championId)" alt="" class="size-7 rounded-md" />
      <span class="min-w-0 flex-1 text-sm">{{ gd.championName(entry.championId) }}</span>
      <select class="field py-1 text-xs" :aria-label="`${gd.championName(entry.championId)}的熟练度`" :value="entry.preference" :disabled="saving" @change="change(entry.championId, $event)">
        <option v-for="choice in choices" :key="choice.value" :value="choice.value">{{ choice.label }}</option>
      </select>
      <button class="btn-ghost text-xs" :disabled="saving" :aria-label="`移除${gd.championName(entry.championId)}的标记`" @click="setPreference(entry.championId, null)">移除</button>
    </div>
    <div v-if="adding" :class="saving && 'pointer-events-none opacity-50'" :inert="saving">
      <ChampionPicker :exclude="entries.map((e) => e.championId)" @pick="setPreference($event, 'familiar')" @close="adding = false" />
    </div>
    <p v-if="store.saveError" role="alert" class="text-xs text-red-400">保存失败：{{ store.saveError }}</p>
    <p v-if="saving" role="status" class="text-xs text-zinc-500">正在保存…</p>
  </div>
</template>
