<script setup lang="ts">
import { computed } from "vue";
import { useAppStore } from "../../stores/app";

const app = useAppStore();
const shown = computed(() => {
  const u = app.update;
  return u?.available && app.dismissed !== u.latest ? u : null;
});
const installing = computed(() => app.installProgress !== null);
</script>

<template>
  <Transition
    enter-from-class="-translate-y-3 opacity-0"
    leave-to-class="-translate-y-2 opacity-0"
    enter-active-class="transition-all duration-500 ease-spring"
    leave-active-class="transition-all duration-150"
  >
    <div
      v-if="shown"
      class="relative mx-6 mb-2 flex items-center gap-3 overflow-hidden rounded-xl border border-sky-400/25 bg-linear-to-r from-sky-500/15 via-sky-500/[0.06] to-transparent px-4 py-2 text-sm backdrop-blur-md"
    >
      <!-- Download progress -->
      <span
        v-if="installing"
        class="absolute bottom-0 left-0 h-0.5 bg-linear-to-r from-sky-300 to-sky-500 shadow-[0_0_8px_rgb(56_189_248/0.8)] transition-[width] duration-200"
        :style="{ width: `${(app.installProgress ?? 0) * 100}%` }"
      />
      <span class="relative flex size-2 shrink-0">
        <span class="absolute inline-flex size-full animate-ping rounded-full bg-sky-400 opacity-60" />
        <span class="relative inline-flex size-2 rounded-full bg-sky-400" />
      </span>
      <span class="font-semibold whitespace-nowrap text-sky-200">发现新版本 v{{ shown.latest }}</span>
      <span class="text-xs whitespace-nowrap text-zinc-400">当前 v{{ shown.current }}</span>
      <span v-if="installing" class="text-xs text-sky-200/80 tabular-nums">
        {{
          (app.installProgress ?? 0) < 1
            ? `正在下载 ${Math.round((app.installProgress ?? 0) * 100)}%`
            : "下载完成，正在启动安装程序…"
        }}
      </span>
      <span v-else-if="app.installError" class="truncate text-xs text-red-300" v-tip="app.installError">
        更新失败：{{ app.installError }}
      </span>
      <div class="ml-auto flex shrink-0 gap-2">
        <template v-if="!installing">
          <button
            v-if="shown.installable"
            class="btn btn-primary py-1"
            v-tip="'下载安装包并自动安装，完成后重新打开 Lumina'"
            @click="app.installUpdate()"
          >
            立即更新
          </button>
          <button
            :class="shown.installable ? 'btn btn-ghost py-1' : 'btn btn-primary py-1'"
            v-tip="shown.installable ? '在浏览器打开发布页' : '免安装版请下载新的压缩包替换'"
            @click="app.openRelease()"
          >
            {{ shown.installable ? "查看详情" : "前往下载" }}
          </button>
          <button class="btn btn-ghost py-1" v-tip="'这个版本不再提示'" @click="app.dismissUpdate()">
            忽略
          </button>
        </template>
      </div>
    </div>
  </Transition>
</template>
