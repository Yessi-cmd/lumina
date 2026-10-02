import { computed, ref } from "vue";
import type { PoolEntry, PoolPreference } from "../../api";
import { useLcuStore } from "../../stores/lcu";
import { useSettingsStore } from "../../stores/settings";

export function useChampionPool(position: () => string) {
  const lcu = useLcuStore();
  const store = useSettingsStore();
  const saving = ref(false);
  const account = computed(() => {
    const s = lcu.snapshot;
    return lcu.connected && s.summoner?.puuid
      ? `${s.client?.platformId ?? ""}:${s.summoner.puuid}`
      : "";
  });
  const entries = computed<PoolEntry[]>(
    () => store.settings?.championPools[account.value]?.[position()] ?? [],
  );
  async function setPreference(championId: number, preference: PoolPreference | null) {
    if (!account.value || !store.settings || saving.value) return;
    const updated = entries.value.filter((e) => e.championId !== championId);
    if (preference) {
      if (updated.length >= 30) return;
      updated.push({ championId, preference });
    }
    const pools = store.settings.championPools;
    saving.value = true;
    try {
      await store.update({
        championPools: {
          ...pools,
          [account.value]: { ...pools[account.value], [position()]: updated },
        },
      });
    } finally {
      saving.value = false;
    }
  }
  return { account, entries, saving, setPreference, store };
}
