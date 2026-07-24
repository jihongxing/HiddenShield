<script setup lang="ts">
import type { SourceWarning } from "../lib/tauri-api";

defineProps<{ warnings: SourceWarning[] }>();
</script>

<template>
  <div v-if="warnings.length > 0" class="source-warnings">
    <div
      v-for="(w, i) in warnings"
      :key="i"
      class="source-warnings__item"
      :class="{
        'source-warnings__item--info': w.type === 'info',
        'source-warnings__item--warning': w.type === 'warning',
      }"
    >
      <span class="source-warnings__icon" aria-hidden="true">{{ w.type === 'info' ? 'i' : '!' }}</span>
      <span>{{ w.message }}</span>
    </div>
  </div>
</template>
