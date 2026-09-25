import { defineStore } from "pinia";
import { shallowRef, watch } from "vue";
import { api, assetUrl, type GameData } from "../api";
import { useLcuStore } from "./lcu";

/** Champion/item/spell/queue tables, loaded once per client connection. */
export const useGameDataStore = defineStore("gameData", () => {
  const data = shallowRef<GameData | null>(null);
  const lcu = useLcuStore();
  let loading = false;

  async function load() {
    if (data.value || loading) return;
    loading = true;
    try {
      data.value = await api.gameData();
    } catch (err) {
      console.warn("Failed to load game data", err);
    } finally {
      loading = false;
    }
  }

  watch(
    () => lcu.connected,
    (connected) => {
      if (connected) load();
      else data.value = null;
    },
    { immediate: true },
  );

  function championName(id: number): string {
    return data.value?.champions[id]?.name ?? `英雄 ${id}`;
  }

  function championIcon(id: number): string {
    const path = data.value?.champions[id]?.icon || `/lol-game-data/assets/v1/champion-icons/${id}.png`;
    return assetUrl(path);
  }

  /** Empty string for empty slots or unknown ids. */
  function itemIcon(id: number): string {
    const path = id > 0 ? data.value?.itemIcons[id] : undefined;
    return path ? assetUrl(path) : "";
  }

  function spellIcon(id: number): string {
    const path = data.value?.spellIcons[id];
    return path ? assetUrl(path) : "";
  }

  function queueName(queueId: number, gameMode: string): string {
    return data.value?.queueNames[queueId] || gameMode || `队列 ${queueId}`;
  }

  return { data, championName, championIcon, itemIcon, spellIcon, queueName };
});

export function profileIconUrl(iconId: number): string {
  return assetUrl(`/lol-game-data/assets/v1/profile-icons/${iconId}.jpg`);
}
