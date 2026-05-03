<script setup lang="ts">
import { trackClick } from "../lib/analytics";

defineProps<{
  label: string;
  disabled?: boolean;
}>();

function handleClick() {
  trackClick("upgrade_subscription_click");
  alert("订阅权益即将上线，敬请期待！🙌");
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
    <span class="pro-badge__tag">订阅</span>
  </button>
</template>
