<script setup lang="ts">
import { trackClick } from "../lib/analytics";

defineProps<{
  label: string;
  disabled?: boolean;
}>();

function handleClick() {
  trackClick("upgrade_pro_click");
  alert("Pro 版即将上线，敬请期待！🙌");
}
</script>

<template>
  <button
    class="pro-badge"
    :class="{ 'pro-badge--disabled': disabled }"
    type="button"
    :disabled="disabled"
    @click="handleClick"
  >
    <span class="pro-badge__icon">⚡</span>
    <span class="pro-badge__label">{{ label }}</span>
    <span class="pro-badge__tag">Pro</span>
  </button>
</template>
