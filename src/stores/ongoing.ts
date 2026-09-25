import { defineStore } from "pinia";
import { shallowRef, watch } from "vue";
import { api, events, type Roster, type RosterInsights } from "../api";

/** Players of the current champ select or game, maintained by the backend. */
export const useOngoingStore = defineStore("ongoing", () => {
  const roster = shallowRef<Roster | null>(null);
  const insights = shallowRef<RosterInsights | null>(null);
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
    const puuids = [...r.allies, ...r.enemies].map((p) => p.puuid).sort();
    return [r.gameId, r.queueId, ...puuids].join(",");
  };

  watch(
    () => playersKey(roster.value),
    async (key) => {
      insights.value = null;
      if (!key) return;
      try {
        // Waits for every history (usually prefetched) and a few timelines per player.
        const result = await api.rosterInsights();
        if (key === playersKey(roster.value)) insights.value = result;
      } catch (err) {
        console.warn("Failed to load roster insights", err);
      }
    },
  );

  return { roster, insights, start };
});
