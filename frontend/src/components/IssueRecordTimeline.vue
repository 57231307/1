<template>
  <div class="issue-record-timeline">
    <el-empty v-if="records.length === 0" :description="t('components.issueRecordTimeline.empty')" />
    <el-timeline v-else :aria-label="t('components.issueRecordTimeline.ariaLabel')">
      <el-timeline-item
        v-for="record in records"
        :key="record.id"
        :timestamp="formatDate(record.issued_at)"
        :type="timelineType(record.status)"
        placement="top"
      >
        <el-card shadow="hover">
          <div class="record-header">
            <div>
              <strong>{{ t('components.issueRecordTimeline.colorCard', { id: record.color_card_id }) }}</strong>
              <el-tag size="small" :type="tagType(record.status)" style="margin-left: 8px">
                {{ ISSUE_STATUS[record.status as keyof typeof ISSUE_STATUS] || record.status }}
              </el-tag>
            </div>
            <div class="record-id">{{ t('components.issueRecordTimeline.recordId', { id: record.id }) }}</div>
          </div>
          <div class="record-body">
            <div class="row">
              <span class="label">{{ t('components.issueRecordTimeline.customerId') }}</span>
              <span>{{ record.customer_id }}</span>
              <span class="label" style="margin-left: 24px">{{ t('components.issueRecordTimeline.issueQty') }}</span>
              <span>{{ record.issue_qty }}</span>
              <span class="label" style="margin-left: 24px">{{ t('components.issueRecordTimeline.operator') }}</span>
              <span>{{ record.issued_by }}</span>
            </div>
            <div v-if="record.dye_lot_no" class="row">
              <span class="label">{{ t('components.issueRecordTimeline.dyeLotNo') }}</span>
              <span>{{ record.dye_lot_no }}</span>
            </div>
            <div v-if="record.expected_return_date" class="row">
              <span class="label">{{ t('components.issueRecordTimeline.expectedReturn') }}</span>
              <span>{{ formatDate(record.expected_return_date) }}</span>
            </div>
            <div v-if="record.actual_return_date" class="row">
              <span class="label">{{ t('components.issueRecordTimeline.actualReturn') }}</span>
              <span>{{ formatDate(record.actual_return_date) }}</span>
            </div>
            <div v-if="record.purpose" class="row">
              <span class="label">{{ t('components.issueRecordTimeline.purpose') }}</span>
              <span>{{ record.purpose }}</span>
            </div>
            <div v-if="record.compensation_amount" class="row">
              <span class="label">{{ t('components.issueRecordTimeline.compensationAmount') }}</span>
              <span style="color: #f56c6c; font-weight: bold">¥{{ record.compensation_amount }}</span>
            </div>
            <div v-if="record.remark" class="row notes">
              <span class="label">{{ t('components.issueRecordTimeline.remark') }}</span>
              <span>{{ record.remark }}</span>
            </div>
          </div>
        </el-card>
      </el-timeline-item>
    </el-timeline>
  </div>
</template>

<script setup lang="ts">
import { ISSUE_STATUS, ISSUE_STATUS_COLORS, type IssueRecordInfo } from '@/api/color-card'
import { useI18n } from 'vue-i18n'

const { t } = useI18n({ useScope: 'global' })

defineProps<{ records: IssueRecordInfo[] }>()

const formatDate = (s?: string) => (s ? new Date(s).toLocaleString('zh-CN') : '-')

/** el-tag 类型联合（与 element-plus TagProps.type 对齐） */
type TagType = '' | 'success' | 'warning' | 'info' | 'danger'

/** 发放状态对应的 el-tag 类型 */
const tagType = (status: string): TagType =>
  (ISSUE_STATUS_COLORS[status] || '') as TagType

const timelineType = (status: string): 'primary' | 'success' | 'warning' | 'danger' => {
  switch (status) {
    case 'issued':
      return 'warning'
    case 'returned':
      return 'success'
    case 'lost':
    case 'damaged':
      return 'danger'
    case 'cancelled':
      return 'primary'
    default:
      return 'primary'
  }
}
</script>

<style scoped>
.issue-record-timeline { padding: 16px 0; }
.record-header { display: flex; justify-content: space-between; margin-bottom: 8px; }
.record-id { color: #909399; font-size: 12px; }
.record-body .row {
  margin: 6px 0;
  font-size: 14px;
  color: #303133;
}
.label { color: #909399; margin-right: 8px; }
.notes { background: #f5f7fa; padding: 6px 10px; border-radius: 4px; }
</style>
