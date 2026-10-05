<script setup lang="ts">
import { computed, ref, shallowRef, watch } from "vue";
import { api, type RosterPlayer, type TagTone } from "../api";
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

// --- 敌方速报: one line per opponent, copied for the in-game chat ---------------------
const NEWLINE = "\n";
const LANE_ORDER = ["TOP", "JUNGLE", "MIDDLE", "BOTTOM", "UTILITY"];
const showReport = ref(false);
const reportLines = shallowRef<string[]>([]);
const reportLoading = ref(false);
const copied = ref<number | "all" | null>(null);
let copiedTimer: ReturnType<typeof setTimeout> | undefined;

/** "名字 上单 上等马 小代 胜率62% KDA3.4", opponents ordered top to support. */
async function buildReport(): Promise<string[]> {
  const r = roster.value;
  if (!r) return [];
  const rows = await Promise.all(
    r.enemies.map(async (p) => {
      const [summoner, profile] = await Promise.all([
        api.summonerByPuuid(p.puuid).catch(() => null),
        api.playerProfile(p.puuid, p.championId, r.queueId, p.position).catch(() => null),
      ]);
      const name = summoner?.gameName || summoner?.displayName || gd.championName(p.championId);
      const lane = p.position || profile?.position || "";
      const power = insights.value?.powers[p.puuid];
      const tier = !power
        ? "数据不足"
        : power.tier === "top"
          ? "上等马"
          : power.tier === "bottom"
            ? "下等马"
            : "中等马";
      const carry = insights.value?.tags[p.puuid]?.some((t) => t.id === "carry") ? " 小代" : "";
      const week = profile?.week;
      const rate = !week
        ? "胜率未知"
        : week.games === 0
          ? "本周无排位"
          : `本周胜率${Math.round((week.wins / week.games) * 100)}%(${week.wins}/${week.games}${week.games < 5 ? " 样本少" : ""})`;
      const kda = profile && profile.sampleGames > 0 ? `KDA${profile.avgKda.toFixed(1)}` : "近期无战绩";
      const stats = `${rate} ${kda}`;
      const text = `${name} ${POSITIONS[lane] ?? "未知位置"} ${tier}${carry} ${stats}`;
      const order = LANE_ORDER.indexOf(lane);
      return { text, order: order < 0 ? LANE_ORDER.length : order };
    }),
  );
  return rows.sort((a, b) => a.order - b.order).map((row) => row.text);
}

async function refreshReport() {
  reportLoading.value = true;
  try {
    reportLines.value = await buildReport();
  } finally {
    reportLoading.value = false;
  }
}

function toggleReport() {
  showReport.value = !showReport.value;
  if (showReport.value) refreshReport();
}

// Tiers arrive with the detailed analysis; keep an open report up to date.
watch(insights, () => {
  if (showReport.value) refreshReport();
});

async function copy(text: string, which: number | "all") {
  try {
    await navigator.clipboard.writeText(text);
    copied.value = which;
    clearTimeout(copiedTimer);
    copiedTimer = setTimeout(() => (copied.value = null), 1500);
  } catch (err) {
    console.warn("Failed to copy", err);
  }
}

function championOf(puuid: string): number {
  const r = roster.value;
  const player: RosterPlayer | undefined = r
    ? [...r.allies, ...r.enemies].find((p) => p.puuid === puuid)
    : undefined;
  return player?.championId ?? 0;
}
</script>

<template>
  <section class="flex max-w-6xl flex-col gap-3">
    <!-- One compact row: the five players are the point of this page. -->
    <header class="flex flex-wrap items-center gap-x-3 gap-y-1.5">
      <h1 class="flex items-center gap-2 text-lg font-semibold tracking-tight text-zinc-100">
        {{ title }}
        <span
          v-if="roster && roster.stage !== 'lobby'"
          class="inline-flex items-center gap-1.5 rounded-full border border-emerald-400/25 bg-emerald-400/10 px-2 py-0.5 text-[11px] font-medium text-emerald-300"
        >
          <span class="relative flex size-1.5">
            <span class="absolute inline-flex size-full animate-ping rounded-full bg-emerald-400 opacity-70" />
            <span class="relative size-1.5 rounded-full bg-emerald-400" />
          </span>
          实时
        </span>
      </h1>
      <TransitionGroup
        v-if="roster && insights?.advice.length"
        name="list"
        tag="div"
        class="flex flex-wrap gap-1.5"
        appear
      >
        <div
          v-for="(a, i) in insights.advice"
          :key="a.title + a.puuid"
          v-tip="{ title: a.title, body: a.detail }"
          :style="{ transitionDelay: `${i * 50}ms` }"
          class="flex cursor-default items-center gap-1.5 rounded-lg border py-0.5 pr-2 pl-0.5 text-xs font-medium text-zinc-200 backdrop-blur-md"
          :class="ADVICE_TONE[a.tone]"
        >
          <img
            v-if="championOf(a.puuid) > 0"
            :src="gd.championIcon(championOf(a.puuid))"
            class="size-5 shrink-0 rounded-md bg-zinc-800"
          />
          {{ a.title }}
        </div>
      </TransitionGroup>
      <span v-if="roster && ongoing.loading" class="flex items-center gap-2 text-xs text-zinc-500">
        <span class="size-3 animate-spin rounded-full border-2 border-zinc-600 border-t-amber-400" />
        {{ insights ? "基础结果已显示，正在补充战力与对线数据…" : "正在分析近期战绩…" }}
      </span>
    </header>

    <div v-if="roster && ongoing.analysisError" role="alert" class="flex items-center gap-3 text-xs text-amber-300">
      <span>{{ ongoing.analysisError }}</span>
      <button class="btn-ghost" :disabled="ongoing.loading" @click="ongoing.refreshInsights()">重试分析</button>
    </div>
    <p v-for="warning in insights?.warnings ?? []" :key="warning" class="text-xs text-zinc-500">
      {{ warning }}
    </p>

    <p v-if="!lcu.connected" class="empty-state">连接英雄联盟客户端后显示对局信息。</p>
    <p v-else-if="!roster" class="empty-state">
      进入房间后显示房间成员，英雄选择时显示队友，进入加载界面后显示敌方。
    </p>

    <template v-else>
      <div class="grid grid-cols-2 gap-5">
        <div class="stagger flex flex-col gap-1.5">
          <h2 class="flex items-center gap-2 px-1 text-xs font-semibold text-sky-300">
            <span class="h-3 w-1 rounded-full bg-sky-400 shadow-[0_0_10px_rgb(56_189_248/0.7)]" />
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

        <div class="stagger flex flex-col gap-1.5">
          <ChampionAssistant
            v-if="roster.stage === 'champSelect'"
            :position="me?.position ?? ''"
            :champion-id="me?.championId ?? 0"
            :enemy-champions="roster.enemyChampions"
          />
          <h2 v-else class="flex items-center gap-2 px-1 text-xs font-semibold text-red-300">
            <span class="h-3 w-1 rounded-full bg-red-400 shadow-[0_0_10px_rgb(248_113_113/0.7)]" />
            敌方
            <button
              v-if="roster.stage === 'inGame' && roster.enemies.length"
              type="button"
              class="ml-auto rounded-md border border-white/10 bg-white/5 px-2 py-0.5 text-[11px] font-medium text-zinc-300 transition-colors hover:border-red-400/40 hover:text-red-200"
              @click="toggleReport"
            >
              {{ showReport ? "收起速报" : "敌方速报" }}
            </button>
          </h2>
          <div
            v-if="showReport && roster.stage === 'inGame'"
            class="flex flex-col gap-1 rounded-xl border border-red-400/20 bg-zinc-900/75 p-2 text-xs backdrop-blur-md"
          >
            <p v-if="reportLoading && !reportLines.length" class="px-1 text-zinc-500">正在整理敌方信息…</p>
            <button
              v-for="(line, i) in reportLines"
              :key="i"
              type="button"
              class="flex items-center gap-2 rounded-md px-2 py-1 text-left text-zinc-200 transition-colors hover:bg-white/5"
              v-tip="'点击复制这一行'"
              @click="copy(line, i)"
            >
              <span class="min-w-0 flex-1 truncate select-text">{{ line }}</span>
              <span class="shrink-0 text-[11px]" :class="copied === i ? 'text-emerald-400' : 'text-zinc-500'">
                {{ copied === i ? "已复制" : "复制" }}
              </span>
            </button>
            <div v-if="reportLines.length" class="flex items-center gap-2 px-1 pt-1">
              <span class="flex-1 text-[11px] text-zinc-500">复制后在游戏里按回车，Ctrl+V 粘贴发送。</span>
              <button type="button" class="btn btn-secondary px-2 py-0.5 text-xs" @click="copy(reportLines.join(NEWLINE), 'all')">
                {{ copied === "all" ? "已复制" : "复制全部" }}
              </button>
            </div>
          </div>
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
