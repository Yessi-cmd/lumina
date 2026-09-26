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

const POSITIONS: Record<string, string> = {
  TOP: "上单",
  JUNGLE: "打野",
  MIDDLE: "中单",
  BOTTOM: "下路",
  UTILITY: "辅助",
};

const ADVICE_TONE: Record<TagTone, string> = {
  positive: "border-emerald-400/20 bg-linear-to-br from-emerald-500/10 to-emerald-500/[0.02]",
  negative: "border-red-400/20 bg-linear-to-br from-red-500/10 to-red-500/[0.02]",
  warning: "border-amber-400/20 bg-linear-to-br from-amber-500/10 to-amber-500/[0.02]",
  neutral: "border-white/[0.06] bg-zinc-900/75",
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
  <section class="flex max-w-6xl flex-col gap-5">
    <header>
      <div class="eyebrow">Live game</div>
      <h1 class="page-title mt-1 flex items-center gap-3">
        {{ title }}
        <span
          v-if="roster && roster.stage !== 'lobby'"
          class="inline-flex items-center gap-1.5 rounded-full border border-emerald-400/25 bg-emerald-400/10 px-2 py-0.5 text-xs font-medium tracking-normal text-emerald-300"
        >
          <span class="size-1.5 animate-pulse rounded-full bg-emerald-400" />
          实时
        </span>
      </h1>
    </header>

    <p v-if="!lcu.connected" class="empty-state">连接英雄联盟客户端后显示对局信息。</p>
    <p v-else-if="!roster" class="empty-state">
      进入房间后显示房间成员，英雄选择时显示队友，进入加载界面后显示敌方。
    </p>

    <template v-else>
      <TransitionGroup v-if="insights?.advice.length" name="list" tag="div" class="grid grid-cols-2 gap-2" appear>
        <div
          v-for="(a, i) in insights.advice"
          :key="a.title + a.puuid"
          :style="{ transitionDelay: `${i * 50}ms` }"
          class="flex items-start gap-3 rounded-xl border p-3"
          :class="ADVICE_TONE[a.tone]"
        >
          <img
            v-if="championOf(a.puuid) > 0"
            :src="gd.championIcon(championOf(a.puuid))"
            class="size-9 shrink-0 rounded-lg bg-zinc-800 ring-1 ring-white/10"
          />
          <div class="min-w-0">
            <div class="text-sm font-semibold text-zinc-100">{{ a.title }}</div>
            <div class="text-xs leading-5 text-zinc-400">{{ a.detail }}</div>
          </div>
        </div>
      </TransitionGroup>
      <p v-else-if="!insights" class="flex items-center gap-2 text-xs text-zinc-500">
        <span class="size-3 animate-spin rounded-full border-2 border-zinc-600 border-t-amber-400" />
        正在分析双方战绩与对线数据…
      </p>

      <div class="grid grid-cols-2 gap-6">
        <div class="flex flex-col gap-2">
          <h2 class="flex items-center gap-2 px-1 text-sm font-semibold text-sky-300">
            <span class="h-3.5 w-1 rounded-full bg-sky-400" />
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
          <div
            v-for="(a, i) in roster.anonymousAllies"
            :key="`anonymous-${i}`"
            class="flex items-center gap-3 rounded-xl border border-dashed border-white/10 bg-zinc-900/40 p-3"
          >
            <img
              v-if="a.championId > 0"
              :src="gd.championIcon(a.championId)"
              class="size-11 shrink-0 rounded-lg bg-zinc-800 opacity-80 ring-1 ring-white/10"
            />
            <div v-else class="size-11 shrink-0 rounded-lg bg-zinc-800 ring-1 ring-white/10" />
            <div class="min-w-0">
              <div class="text-sm font-medium text-zinc-300">
                匿名队友
                <span v-if="POSITIONS[a.position]" class="ml-1 text-xs text-zinc-500">{{ POSITIONS[a.position] }}</span>
              </div>
              <div class="text-xs text-zinc-500">暂时无法解析该玩家身份，获取到有效身份后会自动显示战绩。</div>
            </div>
          </div>
        </div>

        <div class="flex flex-col gap-2">
          <ChampionAssistant
            v-if="roster.stage === 'champSelect'"
            :position="me?.position ?? ''"
            :champion-id="me?.championId ?? 0"
            :enemy-champions="roster.enemyChampions"
          />
          <h2 v-else class="flex items-center gap-2 px-1 text-sm font-semibold text-red-300">
            <span class="h-3.5 w-1 rounded-full bg-red-400" />
            敌方
          </h2>
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
            class="empty-state"
          >
            开始排队并进入英雄选择后显示队友，进入加载界面后显示敌方。
          </div>
          <div
            v-if="roster.hiddenEnemies > 0 && roster.stage !== 'champSelect'"
            class="empty-state"
          >
            {{ roster.hiddenEnemies }} 名敌方玩家身份不可见。
          </div>
        </div>
      </div>
    </template>
  </section>
</template>
