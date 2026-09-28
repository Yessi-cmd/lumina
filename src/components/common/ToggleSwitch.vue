<script setup lang="ts">
defineProps<{ modelValue: boolean; disabled?: boolean }>();
defineEmits<{ "update:modelValue": [value: boolean] }>();
</script>

<template>
  <button
    type="button"
    role="switch"
    :aria-checked="modelValue"
    :disabled="disabled"
    class="relative inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full border transition-colors duration-300 ease-out-expo disabled:cursor-not-allowed disabled:opacity-40"
    :class="
      modelValue
        ? 'border-amber-300/60 bg-linear-to-r from-amber-400 to-orange-500 shadow-[0_0_16px_-2px_rgb(245_158_11/0.6)]'
        : 'border-white/10 bg-zinc-700/80 hover:bg-zinc-600/80'
    "
    @click="$emit('update:modelValue', !modelValue)"
  >
    <span
      class="inline-block size-[18px] rounded-full bg-linear-to-b from-white to-zinc-200 shadow-[0_2px_6px_rgb(0_0_0/0.4)] transition-transform duration-400 ease-spring"
      :class="modelValue ? 'translate-x-[21px]' : 'translate-x-[2px]'"
    />
  </button>
</template>
