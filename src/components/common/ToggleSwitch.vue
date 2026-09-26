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
    :class="modelValue ? 'border-amber-400/60 bg-amber-500' : 'border-white/10 bg-zinc-700'"
    @click="$emit('update:modelValue', !modelValue)"
  >
    <span
      class="inline-block size-[18px] rounded-full bg-white shadow-md transition-transform duration-300 ease-spring"
      :class="modelValue ? 'translate-x-[21px]' : 'translate-x-[2px]'"
    />
  </button>
</template>
