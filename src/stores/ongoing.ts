import { defineStore } from "pinia";
import { shallowRef, watch } from "vue";
import { api, events, type Roster, type RosterInsights } from "../api";

/** Players of the current champ select or game, maintained by the backend. */
export const useOngoingStore = defineStore("ongoing", () => {
  const roster = shallowRef<Roster | null>(null);
  const insights = shallowRef<RosterInsights | null>(null);
  const loading = shallowRef(false);
  const analysisError = shallowRef<string | null>(null);
  let requestId = 0;
  let started = false;

  async function start() {
    if (started) return;
    started = true;
    let gotEvent = false;
    await events.onRoster((r) => {
      gotEvent = true;
      roster.value = r;
    });
    const current = await api.ongoingRoster();
    if (!gotEvent) roster.value = current;
  }

  // Roster-wide analysis depends on who is in the game and the queue, not on picks.
  const playersKey = (r: Roster | null) => {
    if (!r) return "";
    const members = (players: Roster["allies"]) =>
      players.map((p) => `${p.puuid}:${p.position}`).sort().join(",");
    return [r.stage, r.gameId, r.queueId, members(r.allies), members(r.enemies)].join("|");
  };

  async function refreshInsights() {
    const id = ++requestId;
    const key = playersKey(roster.value);
    const current = () => id === requestId && key === playersKey(roster.value);
    insights.value = null;
    analysisError.value = null;
    loading.value = !!key;
    if (!key) return;
    try {
      const basic = await api.rosterInsights(false);
      if (!current()) return;
      insights.value = basic;
      const detailed = await api.rosterInsights(true);
      if (current()) insights.value = detailed;
    } catch (err) {
      if (current()) {
        analysisError.value = insights.value ? "详细分析加载失败，已保留基础结果" : String(err);
        console.warn("Failed to load roster insights", err);
      }
    } finally {
      if (current()) loading.value = false;
    }
  }

  watch(() => playersKey(roster.value), refreshInsights, { flush: "sync" });

  return { roster, insights, loading, analysisError, refreshInsights, start };
});
