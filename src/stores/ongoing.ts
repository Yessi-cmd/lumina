import { defineStore } from "pinia";
import { shallowRef, watch } from "vue";
import { api, events, type Roster, type RosterRelations } from "../api";

/** Players of the current champ select or game, maintained by the backend. */
export const useOngoingStore = defineStore("ongoing", () => {
  const roster = shallowRef<Roster | null>(null);
  const relations = shallowRef<RosterRelations | null>(null);
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

  // Premades and "met before" depend only on who is in the game, not on picks.
  const playersKey = (r: Roster | null) =>
    r ? [r.gameId, ...[...r.allies, ...r.enemies].map((p) => p.puuid).sort()].join(",") : "";

  watch(
    () => playersKey(roster.value),
    async (key) => {
      relations.value = null;
      if (!key) return;
      try {
        // Waits for every player's history; the backend has usually prefetched them.
        const result = await api.rosterRelations();
        if (key === playersKey(roster.value)) relations.value = result;
      } catch (err) {
        console.warn("Failed to load roster relations", err);
      }
    },
  );

  return { roster, relations, start };
});
