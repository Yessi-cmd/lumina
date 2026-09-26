<script setup lang="ts">
import { computed } from "vue";
import type { RosterPlayer, TagTone } from "../api";
import ChampionAssistant from "../components/champion/ChampionAssistant.vue";
import PlayerCard from "../components/player/PlayerCard.vue";
import { useGameDataStore } from "../stores/gameData";
import { useLcuStore } from "../stores/lcu";
import { useOngoingStore } from "../stores/ongoing";

const lcu = useLcuStore();
const ongoing = useOngoingStore();
const gd = useGameDataStore();

const roster = computed(() => ongoing.roster);
const me = computed(() => roster.value?.allies.find((p) => p.isSelf));
const insights = computed(() => ongoing.insights);
const title = computed(() => {
  const r = roster.value;
  if (!r) return "对局";
  const STAGES = { lobby: "房间", champSelect: "英雄选择", inGame: "对局中" } as const;
  const stage = STAGES[r.stage];
  return r.queueId > 0 ? `${stage} · ${gd.queueName(r.queueId, "")}` : stage;
});

const ADVICE_TONE: Record<TagTone, string> = {
  positive: "border-emerald-700/60 bg-emerald-950/40",
  negative: "border-red-700/60 bg-red-950/40",
  warning: "border-amber-700/60 bg-amber-950/30",
  neutral: "border-zinc-700 bg-zinc-900",
};

function championOf(puuid: string): number {
  const r = roster.value;
  const player: RosterPlayer | undefined = r
    ? [...r.allies, ...r.enemies].find((p) => p.puuid === puuid)
    : undefined;
  return player?.championId ?? 0;
}
</script>

<template>
  <section class="flex max-w-5xl flex-col gap-4">
    <h1 class="text-xl font-semibold">{{ title }}</h1>

    <p v-if="!lcu.connected" class="text-sm text-zinc-400">连接英雄联盟客户端后显示对局信息。</p>
    <p v-else-if="!roster" class="text-sm text-zinc-400">
      进入房间后显示房间成员，英雄选择时显示队友，进入加载界面后显示敌方。
    </p>

    <template v-else>
      <div v-if="insights?.advice.length" class="grid grid-cols-2 gap-2">
        <div
          v-for="(a, i) in insights.advice"
          :key="i"
          class="flex items-start gap-2 rounded-lg border p-2.5"
          :class="ADVICE_TONE[a.tone]"
        >
          <img
            v-if="championOf(a.puuid) > 0"
            :src="gd.championIcon(championOf(a.puuid))"
            class="size-8 shrink-0 rounded bg-zinc-800"
          />
          <div class="min-w-0">
            <div class="text-sm font-medium">{{ a.title }}</div>
            <div class="text-xs leading-5 text-zinc-400">{{ a.detail }}</div>
          </div>
        </div>
      </div>
      <p v-else-if="!insights" class="text-xs text-zinc-500">正在分析双方战绩与对线数据…</p>

      <div class="grid grid-cols-2 gap-6">
        <div class="flex flex-col gap-2">
          <h2 class="text-sm font-medium text-sky-300">
            {{ roster.stage === "lobby" ? "房间成员" : "我方" }}
          </h2>
          <PlayerCard
            v-for="p in roster.allies"
            :key="p.puuid"
            :player="p"
            :queue-id="roster.queueId"
            :relation-tags="insights?.tags[p.puuid]"
            :power="insights?.powers[p.puuid]"
          />
        </div>

        <div class="flex flex-col gap-2">
          <ChampionAssistant
            v-if="roster.stage === 'champSelect'"
            :position="me?.position ?? ''"
            :champion-id="me?.championId ?? 0"
          />
          <h2 v-else class="text-sm font-medium text-red-300">敌方</h2>
          <PlayerCard
            v-for="p in roster.enemies"
            :key="p.puuid"
            :player="p"
            :queue-id="roster.queueId"
            :relation-tags="insights?.tags[p.puuid]"
            :power="insights?.powers[p.puuid]"
          />
          <div
            v-if="roster.stage === 'lobby'"
            class="rounded-lg border border-dashed border-zinc-800 p-4 text-center text-sm text-zinc-500"
          >
            开始排队并进入英雄选择后显示队友，进入加载界面后显示敌方。
          </div>
          <div
            v-if="roster.hiddenEnemies > 0 && roster.stage !== 'champSelect'"
            class="rounded-lg border border-dashed border-zinc-800 p-4 text-center text-sm text-zinc-500"
          >
            {{ roster.hiddenEnemies }} 名敌方玩家身份不可见。
          </div>
        </div>
      </div>
    </template>
  </section>
</template>
