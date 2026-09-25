import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { api, events, type LcuSnapshot } from "../api";

const initial: LcuSnapshot = {
  status: "disconnected",
  client: null,
  summoner: null,
  gameflowPhase: "None",
  lastError: null,
};

export const useLcuStore = defineStore("lcu", () => {
  const snapshot = ref<LcuSnapshot>(initial);
  let started = false;

  const connected = computed(() => snapshot.value.status === "connected");

  async function start() {
    if (started) return;
    started = true;
    // Listen first so no change is lost; an event is always newer than the initial fetch.
    let gotEvent = false;
    await events.onLcuSnapshot((s) => {
      gotEvent = true;
      snapshot.value = s;
    });
    const current = await api.lcuSnapshot();
    if (!gotEvent) snapshot.value = current;
  }

  return { snapshot, connected, start };
});

const PHASE_LABELS: Record<string, string> = {
  None: "空闲",
  Lobby: "房间中",
  Matchmaking: "匹配中",
  CheckedIntoTournament: "已加入锦标赛",
  ReadyCheck: "等待接受对局",
  ChampSelect: "英雄选择",
  GameStart: "游戏启动中",
  FailedToLaunch: "游戏启动失败",
  InProgress: "游戏中",
  Reconnect: "等待重连",
  WaitingForStats: "等待结算",
  PreEndOfGame: "对局结束",
  EndOfGame: "结算",
  TerminatedInError: "对局异常终止",
};

export function phaseLabel(phase: string): string {
  return PHASE_LABELS[phase] ?? phase;
}
