<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { useGameDataStore } from "../stores/gameData";
import { useLcuStore } from "../stores/lcu";
import AllyOverlay from "./AllyOverlay.vue";
import EnemyOverlay from "./EnemyOverlay.vue";

defineProps<{ side: "allies" | "enemies" }>();

/** Client height the layouts are drawn for; the window is as tall as the client. */
const DESIGN_HEIGHT = 720;

const lcu = useLcuStore();
// Champion names and icons.
useGameDataStore();

const zoom = ref(window.innerHeight / DESIGN_HEIGHT);
function onResize() {
  zoom.value = window.innerHeight / DESIGN_HEIGHT;
}

onMounted(() => {
  window.addEventListener("resize", onResize);
  lcu.start().catch((err) => console.error("Failed to start LCU store in overlay", err));
});
onUnmounted(() => window.removeEventListener("resize", onResize));
</script>

<template>
  <div class="h-[720px] overflow-hidden" :style="{ zoom }">
    <AllyOverlay v-if="side === 'allies'" />
    <EnemyOverlay v-else />
  </div>
</template>
