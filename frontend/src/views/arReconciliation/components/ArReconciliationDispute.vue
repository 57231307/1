<!--
  ArReconciliationDispute.vue - AR 对账争议处理对话框
  拆分自 arReconciliation/enhanced.vue（P14 批 1 B3 I-2）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="$t('arReconciliationModule.disputeHandling')"
    width="900px"
    :aria-label="$t('arReconciliationModule.disputeDialogAria')"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form :model="localForm" label-width="100px" :aria-label="$t('arReconciliationModule.disputeFormAria')">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="$t('arReconciliationModule.disputeType')">
            <el-select
              :model-value="localForm.dispute_type"
              @update:model-value="(v: string) => (localForm.dispute_type = v as DisputeRecord['dispute_type'])"
            >
              <el-option
                v-for="o in DISPUTE_TYPE_OPTIONS"
                :key="o.value"
                :label="$t(o.label)"
                :value="o.value"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="$t('arReconciliationModule.disputeAmount')">
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
      <el-form-item :label="$t('arReconciliationModule.disputeDescription')">
        <el-input
          :model-value="localForm.description"
          type="textarea"
          :rows="3"
          :placeholder="$t('arReconciliationModule.disputeDescriptionPlaceholder')"
          @update:model-value="(v: string) => (localForm.description = v ?? '')"
        />
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="emit('submit')">{{ $t('arReconciliationModule.submitDispute') }}</el-button>
      </el-form-item>
    </el-form>

    <el-divider>{{ $t('arReconciliationModule.disputeRecords') }}</el-divider>
    <el-table :data="disputes" border style="width: 100%" :aria-label="$t('arReconciliationModule.disputeRecordsAria')">
      <el-table-column :label="$t('arReconciliationModule.disputeType')" width="100">
        <template #default="scope">
          {{ getDisputeTypeLabel(scope.row.dispute_type) }}
        </template>
      </el-table-column>
      <el-table-column prop="dispute_amount" :label="$t('arReconciliationModule.disputeAmount')" width="120" align="right">
        <template #default="scope">{{ scope.row.dispute_amount.toFixed(2) }}</template>
      </el-table-column>
      <el-table-column :label="$t('common.status')" width="100">
        <template #default="scope">
          <el-tag size="small" :type="getDisputeType(scope.row.status)">
            {{ getDisputeStatusLabel(scope.row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="description" :label="$t('common.description')" show-overflow-tooltip />
      <el-table-column prop="created_at" :label="$t('common.createTime')" width="160" />
      <el-table-column :label="$t('common.operation')" width="100" align="center">
        <template #default="scope">
          <el-button
            v-if="scope.row.status !== 'resolved' && scope.row.status !== 'closed'"
            size="small"
            type="primary"
            @click="emit('resolve', scope.row)"
          >
            {{ $t('arReconciliationModule.resolve') }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import type { DisputeRecord } from '@/api/ar-reconciliation-enhanced'
import { DISPUTE_TYPE_OPTIONS, getDisputeType } from '../composables/arRecFmts'

const { t } = useI18n({ useScope: 'global' })

/**
 * 争议类型 → 翻译文本（根据 DISPUTE_TYPE_OPTIONS 查找 i18n key 后翻译）
 */
const getDisputeTypeLabel = (type: string) => {
  const key = DISPUTE_TYPE_OPTIONS.find(o => o.value === type)?.label
  return key ? t(key) : type
}

/**
 * 争议状态 → 翻译文本
 */
const getDisputeStatusLabel = (status: string) => {
  const keyMap: Record<string, string> = {
    open: 'arReconciliationModule.disputeStatusOpen',
    investigating: 'arReconciliationModule.disputeStatusInvestigating',
    resolved: 'arReconciliationModule.disputeStatusResolved',
    closed: 'arReconciliationModule.disputeStatusClosed',
  }
  const key = keyMap[status]
  return key ? t(key) : status
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
