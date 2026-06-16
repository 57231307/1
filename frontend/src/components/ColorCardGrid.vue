<template>
  <div class="color-card-grid">
    <div v-if="items.length === 0" class="empty">
      <el-empty description="暂无色号" />
    </div>
    <div v-else class="grid">
      <div
        v-for="item in items"
        :key="item.id"
        class="grid-item"
        @click="$emit('scan', item)"
      >
        <div class="swatch" :style="{ background: item.hex_value }">
          <span class="hex-code">{{ item.hex_value }}</span>
        </div>
        <div class="info">
          <div class="code">{{ item.color_code }}</div>
          <div class="name">{{ item.color_name }}</div>
          <div class="meta">
            <span>RGB {{ item.rgb_r }},{{ item.rgb_g }},{{ item.rgb_b }}</span>
            <span v-if="item.lab_l">L* {{ item.lab_l }}</span>
          </div>
        </div>
        <div class="actions" @click.stop>
          <el-button link type="primary" size="small" @click.stop="$emit('scan', item)">扫码</el-button>
          <el-button link type="danger" size="small" @click.stop="$emit('delete', item)">删除</el-button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { ColorItemInfo } from '@/api/color-card'

defineProps<{ items: ColorItemInfo[] }>()
defineEmits<{
  (e: 'scan', item: ColorItemInfo): void
  (e: 'delete', item: ColorItemInfo): void
}>()
</script>

<style scoped>
.color-card-grid { padding: 8px 0; }
.empty { padding: 40px 0; }
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 16px;
}
.grid-item {
  border: 1px solid #ebeef5;
  border-radius: 6px;
  overflow: hidden;
  cursor: pointer;
  transition: all 0.2s;
}
.grid-item:hover {
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.1);
  transform: translateY(-2px);
}
.swatch {
  height: 120px;
  display: flex;
  align-items: flex-end;
  padding: 8px;
}
.hex-code {
  background: rgba(255, 255, 255, 0.9);
  padding: 2px 6px;
  border-radius: 3px;
  font-size: 12px;
  font-family: monospace;
}
.info { padding: 8px 12px; }
.code { font-weight: bold; font-size: 14px; }
.name { color: #606266; font-size: 12px; margin-top: 2px; }
.meta {
  display: flex;
  justify-content: space-between;
  margin-top: 6px;
  font-size: 11px;
  color: #909399;
}
.actions {
  padding: 4px 8px;
  border-top: 1px solid #ebeef5;
  text-align: right;
}
</style>
