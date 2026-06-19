<template>
  <div class="borrow-record-timeline">
    <el-empty v-if="records.length === 0" description="暂无借出记录" />
    <el-timeline v-else>
      <el-timeline-item
        v-for="record in records"
        :key="record.id"
        :timestamp="formatDate(record.borrowed_at)"
        :type="timelineType(record.status)"
        placement="top"
      >
        <el-card shadow="hover">
          <div class="record-header">
            <div>
              <strong>色卡 #{{ record.color_card_id }}</strong>
              <el-tag size="small" :type="(BORROW_STATUS_COLORS[record.status] as any)" style="margin-left: 8px">
                {{ BORROW_STATUS[record.status as keyof typeof BORROW_STATUS] || record.status }}
              </el-tag>
            </div>
            <div class="record-id">记录 ID: {{ record.id }}</div>
          </div>
          <div class="record-body">
            <div class="row">
              <span class="label">客户 ID:</span>
              <span>{{ record.customer_id }}</span>
              <span class="label" style="margin-left: 24px">经办人:</span>
              <span>{{ record.borrowed_by }}</span>
            </div>
            <div v-if="record.expected_return_at" class="row">
              <span class="label">预计归还:</span>
              <span>{{ formatDate(record.expected_return_at) }}</span>
            </div>
            <div v-if="record.actual_return_at" class="row">
              <span class="label">实际归还:</span>
              <span>{{ formatDate(record.actual_return_at) }}</span>
            </div>
            <div v-if="record.purpose" class="row">
              <span class="label">用途:</span>
              <span>{{ record.purpose }}</span>
            </div>
            <div v-if="record.compensation_amount" class="row">
              <span class="label">赔付金额:</span>
              <span style="color: #f56c6c; font-weight: bold">¥{{ record.compensation_amount }}</span>
            </div>
            <div v-if="record.notes" class="row notes">
              <span class="label">备注:</span>
              <span>{{ record.notes }}</span>
            </div>
          </div>
        </el-card>
      </el-timeline-item>
    </el-timeline>
  </div>
</template>

<script setup lang="ts">
import { BORROW_STATUS, BORROW_STATUS_COLORS, type BorrowRecordInfo } from '@/api/color-card'

defineProps<{ records: BorrowRecordInfo[] }>()

const formatDate = (s?: string) => (s ? new Date(s).toLocaleString('zh-CN') : '-')

const timelineType = (status: string): 'primary' | 'success' | 'warning' | 'danger' => {
  switch (status) {
    case 'borrowed':
      return 'warning'
    case 'returned':
      return 'success'
    case 'lost':
    case 'damaged':
      return 'danger'
    default:
      return 'primary'
  }
}
</script>

<style scoped>
.borrow-record-timeline { padding: 16px 0; }
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
