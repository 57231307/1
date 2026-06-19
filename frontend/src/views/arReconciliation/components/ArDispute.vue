<!--
  ArDispute.vue - AR 对账争议处理对话框
  拆分自 arReconciliation/enhanced.vue（P14 批 1 B3 I-2）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-dialog
    :model-value="visible"
    title="争议处理"
    width="900px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form :model="form" label-width="100px">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="争议类型">
            <el-select
              :model-value="form.dispute_type"
              @update:model-value="(v: string) => (form.dispute_type = v as any)"
            >
              <el-option
                v-for="o in DISPUTE_TYPE_OPTIONS"
                :key="o.value"
                :label="o.label"
                :value="o.value"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="争议金额">
            <el-input-number
              :model-value="form.dispute_amount"
              :min="0"
              :precision="2"
              style="width: 100%"
              @update:model-value="(v: number) => (form.dispute_amount = v ?? 0)"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="争议描述">
        <el-input
          :model-value="form.description"
          type="textarea"
          :rows="3"
          placeholder="请详细描述争议内容"
          @update:model-value="(v: string) => (form.description = v ?? '')"
        />
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="emit('submit')">提交争议</el-button>
      </el-form-item>
    </el-form>

    <el-divider>争议记录</el-divider>
    <el-table :data="disputes" border style="width: 100%">
      <el-table-column label="争议类型" width="100">
        <template #default="scope">
          {{ getDisputeTypeLabel(scope.row.dispute_type) }}
        </template>
      </el-table-column>
      <el-table-column prop="dispute_amount" label="争议金额" width="120" align="right">
        <template #default="scope">{{ scope.row.dispute_amount.toFixed(2) }}</template>
      </el-table-column>
      <el-table-column label="状态" width="100">
        <template #default="scope">
          <el-tag size="small" :type="getDisputeType(scope.row.status)">
            {{ getDisputeStatusLabel(scope.row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="description" label="描述" show-overflow-tooltip />
      <el-table-column prop="created_at" label="创建时间" width="160" />
      <el-table-column label="操作" width="100" align="center">
        <template #default="scope">
          <el-button
            v-if="scope.row.status !== 'resolved' && scope.row.status !== 'closed'"
            size="small"
            type="primary"
            @click="emit('resolve', scope.row)"
          >
            解决
          </el-button>
        </template>
      </el-table-column>
    </el-table>
  </el-dialog>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import type { DisputeRecord } from '@/api/ar-reconciliation-enhanced'
import { DISPUTE_TYPE_OPTIONS, getDisputeType } from '../composables/arRecFmts'

/**
 * 争议类型 → 中文标签（根据 DISPUTE_TYPE_OPTIONS 查找）
 */
const getDisputeTypeLabel = (type: string) => {
  return DISPUTE_TYPE_OPTIONS.find(o => o.value === type)?.label || type
}

/**
 * 争议状态 → 中文标签
 */
const getDisputeStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    open: '待处理',
    investigating: '调查中',
    resolved: '已解决',
    closed: '已关闭',
  }
  return map[status] || status
}

const props = defineProps<{
  visible: boolean
  form: Partial<DisputeRecord>
  disputes: DisputeRecord[]
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  submit: []
  resolve: [row: DisputeRecord]
}>()

void props
</script>
