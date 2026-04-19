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
        <h2>本地版权金库</h2>
        <p class="hero-card__copy">
          📁 数据仅存储在本机，不上传至任何服务器。
        </p>
      </div>
      <div class="hero-card__stats">
        <div>
          <span>保留策略</span>
          <strong>Free 30 天 / Pro 永久</strong>
        </div>
      </div>
    </section>

    <section class="panel">
      <div class="panel__header">
        <div>
          <h3>版权存证记录</h3>
          <p>每次处理自动生成存证卡片</p>
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
            📂 文件已归档/离线
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
