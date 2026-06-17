<!--
  AdvancedReportTab.vue - 高级报表引擎 Tab
  来源：原 advanced/index.vue 中 report tab
  拆分日期：2026-06-17 P1-3-Batch-6
-->
<template>
  <div>
    <div class="page-header">
      <h2 class="page-title">报表引擎</h2>
      <el-button type="primary" @click="emit('design-report')">
        <el-icon><Plus /></el-icon>
        设计报表
      </el-button>
    </div>

    <el-row :gutter="20">
      <el-col v-for="tpl in templates" :key="tpl.id" :span="8">
        <el-card shadow="hover" class="template-card">
          <div class="template-icon">
            <el-icon><Document /></el-icon>
          </div>
          <h3>{{ tpl.name }}</h3>
          <p>{{ tpl.description }}</p>
          <div class="template-actions">
            <el-button type="primary" link size="small" @click="emit('preview', tpl)"
              >预览</el-button
            >
            <el-button type="success" link size="small" @click="emit('generate', tpl)"
              >生成</el-button
            >
            <el-button type="warning" link size="small" @click="emit('edit', tpl)">编辑</el-button>
          </div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { Document } from '@element-plus/icons-vue'

defineProps<{
  templates: any[]
}>()

defineEmits<{
  'design-report': []
  preview: [tpl: any]
  generate: [tpl: any]
  edit: [tpl: any]
}>()
</script>

<style scoped>
.template-card {
  text-align: center;
  margin-bottom: 20px;
}

.template-icon {
  font-size: 48px;
  color: #409eff;
  margin-bottom: 12px;
}

.template-card h3 {
  margin: 8px 0;
}

.template-card p {
  color: #909399;
  font-size: 14px;
  margin-bottom: 16px;
}

.template-actions {
  display: flex;
  justify-content: center;
  gap: 8px;
}
</style>
