<script setup lang="ts">
import { trackClick } from "../lib/analytics";

defineProps<{
  label: string;
  disabled?: boolean;
}>();

function handleClick() {
  trackClick("upgrade_subscription_click");
  alert("请在年度授权页面查看当前基础权益。");
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
    <span class="pro-badge__icon" aria-hidden="true">PRO</span>
    <span class="pro-badge__label">{{ label }}</span>
    <span class="pro-badge__tag">年费</span>
  </button>
</template>
