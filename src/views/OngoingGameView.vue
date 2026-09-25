<script setup lang="ts">
import { computed } from "vue";
import PlayerCard from "../components/player/PlayerCard.vue";
import { useGameDataStore } from "../stores/gameData";
import { useLcuStore } from "../stores/lcu";
import { useOngoingStore } from "../stores/ongoing";

const lcu = useLcuStore();
const ongoing = useOngoingStore();
const gd = useGameDataStore();

const roster = computed(() => ongoing.roster);
const title = computed(() => {
  const r = roster.value;
  if (!r) return "对局";
  const stage = r.stage === "champSelect" ? "英雄选择" : "对局中";
  return r.queueId > 0 ? `${stage} · ${gd.queueName(r.queueId, "")}` : stage;
});
</script>

<template>
  <section class="flex max-w-5xl flex-col gap-4">
    <h1 class="text-xl font-semibold">{{ title }}</h1>

    <p v-if="!lcu.connected" class="text-sm text-zinc-400">连接英雄联盟客户端后显示对局信息。</p>
    <p v-else-if="!roster" class="text-sm text-zinc-400">
      进入英雄选择后会自动显示队友近期战绩，进入加载界面后显示敌方。
    </p>

    <div v-else class="grid grid-cols-2 gap-6">
      <div class="flex flex-col gap-2">
        <h2 class="text-sm font-medium text-sky-300">我方</h2>
        <PlayerCard
          v-for="p in roster.allies"
          :key="p.puuid"
          :player="p"
          :queue-id="roster.queueId"
          :relation-tags="ongoing.relations?.tags[p.puuid]"
        />
      </div>

      <div class="flex flex-col gap-2">
        <h2 class="text-sm font-medium text-red-300">敌方</h2>
        <PlayerCard
          v-for="p in roster.enemies"
          :key="p.puuid"
          :player="p"
          :queue-id="roster.queueId"
          :relation-tags="ongoing.relations?.tags[p.puuid]"
        />
        <div
          v-if="roster.hiddenEnemies > 0"
          class="rounded-lg border border-dashed border-zinc-800 p-4 text-center text-sm text-zinc-500"
        >
          <template v-if="roster.stage === 'champSelect'">
            英雄选择阶段客户端不提供敌方身份，进入加载界面后自动显示。
          </template>
          <template v-else>{{ roster.hiddenEnemies }} 名敌方玩家身份不可见。</template>
        </div>
      </div>
    </div>
  </section>
</template>
