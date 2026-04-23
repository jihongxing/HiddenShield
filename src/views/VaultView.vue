<script setup lang="ts">
import { onMounted, ref } from "vue";
import CopyrightCard from "../components/CopyrightCard.vue";
import ProBadge from "../components/ProBadge.vue";
import { checkFilesExist, listVaultRecords, type VaultRecord } from "../lib/tauri-api";

const records = ref<VaultRecord[]>([]);
const missingPaths = ref<Set<string>>(new Set());

onMounted(async () => {
  records.value = await listVaultRecords();

  // Lazily check which output files still exist
  const allPaths: string[] = [];
  for (const r of records.value) {
    if (r.outputDouyin) allPaths.push(r.outputDouyin);
    if (r.outputBilibili) allPaths.push(r.outputBilibili);
    if (r.outputXhs) allPaths.push(r.outputXhs);
  }
  if (allPaths.length > 0) {
    const missing = await checkFilesExist(allPaths);
    missingPaths.value = new Set(missing);
  }
});

function isRecordOffline(record: VaultRecord): boolean {
  const outputs = [record.outputDouyin, record.outputBilibili, record.outputXhs].filter(Boolean) as string[];
  if (outputs.length === 0) return false;
  return outputs.every(p => missingPaths.value.has(p));
}
</script>

<template>
  <div class="view-shell">
    <section class="hero-card hero-card--compact">
      <div>
        <p class="eyebrow">Vault</p>
        <h2>版权库</h2>
      </div>
    </section>

    <section class="panel">
      <div class="panel__header">
        <div>
          <h3>存证记录</h3>
        </div>
        <span class="pill">{{ records.length }} 条</span>
      </div>

      <!-- Pro features -->
      <div class="vault-pro-actions">
        <ProBadge label="导出版权库" :disabled="true" />
        <ProBadge label="批量处理" :disabled="true" />
      </div>

      <div class="vault-cards">
        <div v-for="(record, idx) in records" :key="record.id" class="vault-card-wrapper">
          <div v-if="isRecordOffline(record)" class="vault-offline-badge">
            已离线
          </div>
          <CopyrightCard
            :record="record"
            :highlight="idx === 0"
          />
        </div>
      </div>
    </section>
  </div>
</template>
