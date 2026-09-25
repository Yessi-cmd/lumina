import { defineStore } from "pinia";
import { shallowRef } from "vue";
import { api, events, type Roster } from "../api";

/** Players of the current champ select or game, maintained by the backend. */
export const useOngoingStore = defineStore("ongoing", () => {
  const roster = shallowRef<Roster | null>(null);
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

  return { roster, start };
});
