<!--
  ArDispute.vue - AR 对账争议处理对话框
  拆分自 arReconciliation/enhanced.vue（P14 批 1 B3 I-2）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    title="争议处理"
    width="900px"
    aria-label="争议处理对话框"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form :model="localForm" label-width="100px" aria-label="争议处理表单">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="争议类型">
            <el-select
              :model-value="localForm.dispute_type"
              @update:model-value="(v: string) => (localForm.dispute_type = v as DisputeRecord['dispute_type'])"
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
              :model-value="localForm.dispute_amount"
              :min="0"
              :precision="2"
              style="width: 100%"
              @update:model-value="(v: number) => (localForm.dispute_amount = v ?? 0)"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="争议描述">
        <el-input
          :model-value="localForm.description"
          type="textarea"
          :rows="3"
          placeholder="请详细描述争议内容"
          @update:model-value="(v: string) => (localForm.description = v ?? '')"
        />
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="emit('submit')">提交争议</el-button>
      </el-form-item>
    </el-form>

    <el-divider>争议记录</el-divider>
    <el-table :data="disputes" border style="width: 100%" aria-label="争议记录列表">
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
import { ref, watch, nextTick } from 'vue'
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
  // 表单数据（由父组件管理，子组件通过 emit 回写）
  form: Partial<DisputeRecord>
  disputes: DisputeRecord[]
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  submit: []
  resolve: [row: DisputeRecord]
  // 整体回写表单数据（父组件监听此事件并 Object.assign 到自己的 form）
  'update:form': [v: Partial<DisputeRecord>]
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localForm = ref<Partial<DisputeRecord>>({ ...props.form })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local（如父组件重新打开对话框时填充数据）
watch(
  () => props.form,
  (newForm) => {
    if (syncing) return
    syncing = true
    localForm.value = { ...newForm }
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 本地变化时通知父组件（用户输入）
watch(
  localForm,
  (newForm) => {
    if (syncing) return
    syncing = true
    emit('update:form', { ...newForm })
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)
</script>
