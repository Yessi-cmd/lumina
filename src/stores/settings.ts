import { defineStore } from "pinia";
import { ref, shallowRef } from "vue";
import { api, events, type PendingAccept, type Settings, type TiltWarning } from "../api";

export const useSettingsStore = defineStore("settings", () => {
  const settings = shallowRef<Settings | null>(null);
  const pendingAccept = shallowRef<PendingAccept | null>(null);
  const tilt = shallowRef<TiltWarning | null>(null);
  const saveError = ref<string | null>(null);
  let started = false;

  async function start() {
    if (started) return;
    started = true;
    let gotEvent = false;
    await events.onAutoAccept((p) => {
      gotEvent = true;
      pendingAccept.value = p;
    });
    let gotTilt = false;
    await events.onTilt((w) => {
      gotTilt = true;
      tilt.value = w;
    });
    const [loaded, pending, warning] = await Promise.all([
      api.settings(),
      api.autoAcceptState(),
      api.tiltState(),
    ]);
    settings.value = loaded;
    if (!gotEvent) pendingAccept.value = pending;
    if (!gotTilt) tilt.value = warning;
  }

  async function update(patch: Partial<Settings>) {
    if (!settings.value) return;
    saveError.value = null;
    try {
      settings.value = await api.saveSettings({ ...settings.value, ...patch });
    } catch (err) {
      saveError.value = String(err);
    }
  }

  function cancelAutoAccept() {
    api.cancelAutoAccept().catch((err) => console.warn("Failed to cancel auto accept", err));
  }

  function dismissTilt() {
    tilt.value = null;
    api.dismissTilt().catch((err) => console.warn("Failed to dismiss tilt warning", err));
  }

  return { settings, pendingAccept, tilt, saveError, start, update, cancelAutoAccept, dismissTilt };
});
