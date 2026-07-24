<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  ElTable,
  ElTableColumn,
  ElButton,
  ElForm,
  ElFormItem,
  ElInput,
  ElInputNumber,
  ElMessage,
  ElRow,
  ElCol,
  ElDescriptions,
  ElCard,
  ElDivider,
  ElSteps,
  ElStep,
  ElResult,
} from 'element-plus'
import { Search, ArrowRightBold, ArrowLeftBold, Camera } from '@element-plus/icons-vue'
import {
  getTraceByFiveDimension,
  forwardTrace,
  backwardTrace,
  createTraceSnapshot,
  type FullTraceChainResponse,
  type TraceChainResponse,
} from '@/api/business-trace'

const { t } = useI18n({ useScope: 'global' })

const traceMode = ref<'five_dimension' | 'forward' | 'backward'>('five_dimension')
const fiveDimensionId = ref('')
const traceResult = ref<FullTraceChainResponse | null>(null)
const forwardResult = ref<TraceChainResponse[]>([])
const backwardResult = ref<TraceChainResponse[]>([])
const loading = ref(false)
const snapshotMessage = ref('')

const forwardForm = ref({
  supplier_id: 0,
  batch_no: '',
})

const backwardForm = ref({
  customer_id: 0,
  batch_no: '',
})

const stageStatusMap: Record<string, string> = {
  PURCHASE_RECEIPT: 'success',
  INVENTORY_IN: 'success',
  PRODUCTION_INPUT: 'process',
  PRODUCTION_OUTPUT: 'process',
  INVENTORY_OUT: 'process',
  SALES_DELIVERY: 'success',
}

const handleFiveDimensionTrace = async () => {
  if (!fiveDimensionId.value.trim()) {
    ElMessage.warning(t('businessTrace.message.fiveDimensionIdRequired'))
    return
  }
  loading.value = true
  try {
    // v11 批次 182 P2-1 修复：const res: any 改为 as 具体类型
    const res = (await getTraceByFiveDimension(fiveDimensionId.value)) as { data?: FullTraceChainResponse }
    // 安全检查：防止后端返回 data 为 null 时崩溃
    if (res.data) traceResult.value = res.data
    snapshotMessage.value = ''
  } catch (error) {
    ElMessage.error(t('businessTrace.message.traceFailed'))
  } finally {
    loading.value = false
  }
}

const handleForwardTrace = async () => {
  if (!forwardForm.value.supplier_id || !forwardForm.value.batch_no) {
    ElMessage.warning(t('businessTrace.message.supplierAndBatchRequired'))
    return
  }
  loading.value = true
  try {
    // v11 批次 182 P2-1 修复：const res: any 改为 as 具体类型
    const res = (await forwardTrace({
      supplier_id: Number(forwardForm.value.supplier_id),
      batch_no: forwardForm.value.batch_no,
    })) as { data?: { traces?: TraceChainResponse[] } }
    forwardResult.value = res.data?.traces || []
  } catch (error) {
    ElMessage.error(t('businessTrace.message.forwardTraceFailed'))
  } finally {
    loading.value = false
  }
}

const handleBackwardTrace = async () => {
  if (!backwardForm.value.customer_id || !backwardForm.value.batch_no) {
    ElMessage.warning(t('businessTrace.message.customerAndBatchRequired'))
    return
  }
  loading.value = true
  try {
    // v11 批次 182 P2-1 修复：const res: any 改为 as 具体类型
    const res = (await backwardTrace({
      customer_id: Number(backwardForm.value.customer_id),
      batch_no: backwardForm.value.batch_no,
    })) as { data?: { traces?: TraceChainResponse[] } }
    backwardResult.value = res.data?.traces || []
  } catch (error) {
    ElMessage.error(t('businessTrace.message.backwardTraceFailed'))
  } finally {
    loading.value = false
  }
}

const handleCreateSnapshot = async () => {
  if (!traceResult.value) {
    ElMessage.warning(t('businessTrace.message.queryTraceFirst'))
    return
  }
  loading.value = true
  try {
    // v11 批次 182 P2-1 修复：const res: any 改为 as 具体类型
    const res = (await createTraceSnapshot(traceResult.value.trace_chain_id)) as { data?: string }
    // 安全检查：防止后端返回 data 为 null 时崩溃
    if (res.data) snapshotMessage.value = res.data
    ElMessage.success(t('businessTrace.message.snapshotCreated'))
  } catch (error) {
    ElMessage.error(t('businessTrace.message.createSnapshotFailed'))
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="app-container">
    <ElTabs v-model="traceMode" type="card">
      <ElTabPane :label="$t('businessTrace.tab.fiveDimension')" name="five_dimension">
        <ElCard :title="$t('businessTrace.fiveDimensionCardTitle')" class="trace-card">
          <ElRow :gutter="20">
            <ElCol :span="16">
              <ElInput
                v-model="fiveDimensionId"
                :placeholder="$t('businessTrace.placeholder.fiveDimensionId')"
                class="filter-item"
                @keyup.enter="handleFiveDimensionTrace"
              />
            </ElCol>
            <ElCol :span="4">
              <ElButton
                type="primary"
                :loading="loading"
                class="w-full"
                @click="handleFiveDimensionTrace"
              >
                <Search /> {{ $t('businessTrace.button.trace') }}
              </ElButton>
            </ElCol>
            <ElCol :span="4">
              <ElButton
                type="success"
                :disabled="!traceResult"
                :loading="loading"
                class="w-full"
                @click="handleCreateSnapshot"
              >
                <Camera /> {{ $t('businessTrace.button.createSnapshot') }}
              </ElButton>
            </ElCol>
          </ElRow>
          <div v-if="snapshotMessage" class="snapshot-message">
            <ElDivider />
            <span class="success-text">{{ snapshotMessage }}</span>
          </div>
        </ElCard>

        <div v-if="traceResult" class="trace-result">
          <ElCard :title="$t('businessTrace.card.traceChainDetail')" class="detail-card">
            <ElDescriptions :column="3" border>
              <ElDescriptionsItem :label="$t('businessTrace.field.traceChainId')">{{
                traceResult.trace_chain_id
              }}</ElDescriptionsItem>
              <ElDescriptionsItem :label="$t('businessTrace.field.fiveDimensionId')">{{
                traceResult.five_dimension_id
              }}</ElDescriptionsItem>
              <ElDescriptionsItem :label="$t('businessTrace.field.productId')">{{ traceResult.product_id }}</ElDescriptionsItem>
              <ElDescriptionsItem :label="$t('businessTrace.field.batchNo')">{{ traceResult.batch_no }}</ElDescriptionsItem>
              <ElDescriptionsItem :label="$t('businessTrace.field.colorNo')">{{ traceResult.color_no }}</ElDescriptionsItem>
              <ElDescriptionsItem :label="$t('businessTrace.field.grade')">{{ traceResult.grade }}</ElDescriptionsItem>
              <ElDescriptionsItem :label="$t('businessTrace.field.totalStages')">{{
                traceResult.total_stages
              }}</ElDescriptionsItem>
              <ElDescriptionsItem :label="$t('businessTrace.field.startTime')">{{ traceResult.start_time }}</ElDescriptionsItem>
              <ElDescriptionsItem :label="$t('businessTrace.field.endTime')">{{
                traceResult.end_time || '-'
              }}</ElDescriptionsItem>
            </ElDescriptions>
          </ElCard>

          <ElCard :title="$t('businessTrace.card.traceFlow')" class="flow-card">
            <ElSteps :space="80" align-center>
              <ElStep
                v-for="(stage, index) in traceResult.stages"
                :key="stage.stage_id"
                :title="stage.stage_name"
                :description="stage.bill_no"
                :status="(stageStatusMap[stage.stage_type] || 'wait') as 'wait' | 'process' | 'finish' | 'error' | 'success'"
              >
                <template #icon>
                  <div class="stage-icon">
                    <span class="stage-number">{{ index + 1 }}</span>
                  </div>
                </template>
              </ElStep>
            </ElSteps>

            <ElDivider />

            <h4>{{ $t('businessTrace.stageDetailTitle') }}</h4>
            <ElTable :data="traceResult.stages" border style="width: 100%" :aria-label="$t('businessTrace.stageTableAriaLabel')">
              <ElTableColumn prop="stage_id" :label="$t('businessTrace.table.stageId')" width="100" />
              <ElTableColumn prop="stage_name" :label="$t('businessTrace.table.stageName')" width="120" />
              <ElTableColumn prop="bill_type" :label="$t('businessTrace.table.billType')" width="120" />
              <ElTableColumn prop="bill_no" :label="$t('businessTrace.table.billNo')" width="150" />
              <ElTableColumn prop="warehouse_name" :label="$t('businessTrace.table.warehouse')" width="120" />
              <ElTableColumn prop="supplier_name" :label="$t('businessTrace.table.supplier')" width="120" />
              <ElTableColumn prop="customer_name" :label="$t('businessTrace.table.customer')" width="120" />
              <ElTableColumn prop="quantity_meters" :label="$t('businessTrace.table.quantityMeters')" width="100" align="right" />
              <ElTableColumn prop="quantity_kg" :label="$t('businessTrace.table.quantityKg')" width="100" align="right" />
              <ElTableColumn prop="created_at" :label="$t('businessTrace.table.time')" width="150" />
            </ElTable>
          </ElCard>
        </div>

        <ElResult
          v-else
          icon="primary"
          :title="$t('businessTrace.empty.fiveDimensionTitle')"
          :sub-title="$t('businessTrace.empty.fiveDimensionSubTitle')"
        />
      </ElTabPane>

      <ElTabPane :label="$t('businessTrace.tab.forward')" name="forward">
        <ElCard :title="$t('businessTrace.card.supplierToCustomer')" class="trace-card">
          <ElForm :model="forwardForm" label-width="100px" :aria-label="$t('businessTrace.form.forwardAriaLabel')">
            <ElRow :gutter="20">
              <ElCol :span="10">
                <ElFormItem :label="$t('businessTrace.field.supplierId')">
                  <ElInputNumber v-model="forwardForm.supplier_id" :placeholder="$t('businessTrace.placeholder.supplierId')" />
                </ElFormItem>
              </ElCol>
              <ElCol :span="10">
                <ElFormItem :label="$t('businessTrace.field.batchNo')">
                  <ElInput v-model="forwardForm.batch_no" :placeholder="$t('businessTrace.placeholder.batchNo')" />
                </ElFormItem>
              </ElCol>
              <ElCol :span="4">
                <ElButton
                  type="primary"
                  :loading="loading"
                  class="w-full"
                  style="margin-top: 24px"
                  @click="handleForwardTrace"
                >
                  <ArrowRightBold /> {{ $t('businessTrace.button.forwardTrace') }}
                </ElButton>
              </ElCol>
            </ElRow>
          </ElForm>
        </ElCard>

        <ElTable
          v-if="forwardResult.length > 0"
          :data="forwardResult"
          border
          fit
          highlight-current-row
          style="width: 100%"
          :aria-label="$t('businessTrace.table.forwardResultAriaLabel')"
        >
          <ElTableColumn prop="id" label="ID" width="80" />
          <ElTableColumn prop="trace_chain_id" :label="$t('businessTrace.table.traceChainId')" width="150" />
          <ElTableColumn prop="five_dimension_id" :label="$t('businessTrace.field.fiveDimensionId')" />
          <ElTableColumn prop="batch_no" :label="$t('businessTrace.field.batchNo')" width="120" />
          <ElTableColumn prop="color_no" :label="$t('businessTrace.field.colorNo')" width="100" />
          <ElTableColumn prop="grade" :label="$t('businessTrace.field.grade')" width="100" />
          <ElTableColumn prop="current_stage" :label="$t('businessTrace.table.currentStage')" width="120" />
          <ElTableColumn prop="current_bill_no" :label="$t('businessTrace.table.currentBillNo')" width="150" />
          <ElTableColumn prop="created_at" :label="$t('businessTrace.table.createdAt')" width="150" />
        </ElTable>

        <ElResult
          v-else
          icon="primary"
          :title="$t('businessTrace.empty.queryTitle')"
          :sub-title="$t('businessTrace.empty.forwardSubTitle')"
        />
      </ElTabPane>

      <ElTabPane :label="$t('businessTrace.tab.backward')" name="backward">
        <ElCard :title="$t('businessTrace.card.customerToSupplier')" class="trace-card">
          <ElForm :model="backwardForm" label-width="100px" :aria-label="$t('businessTrace.form.backwardAriaLabel')">
            <ElRow :gutter="20">
              <ElCol :span="10">
                <ElFormItem :label="$t('businessTrace.field.customerId')">
                  <ElInputNumber v-model="backwardForm.customer_id" :placeholder="$t('businessTrace.placeholder.customerId')" />
                </ElFormItem>
              </ElCol>
              <ElCol :span="10">
                <ElFormItem :label="$t('businessTrace.field.batchNo')">
                  <ElInput v-model="backwardForm.batch_no" :placeholder="$t('businessTrace.placeholder.batchNo')" />
                </ElFormItem>
              </ElCol>
              <ElCol :span="4">
                <ElButton
                  type="primary"
                  :loading="loading"
                  class="w-full"
                  style="margin-top: 24px"
                  @click="handleBackwardTrace"
                >
                  <ArrowLeftBold /> {{ $t('businessTrace.button.backwardTrace') }}
                </ElButton>
              </ElCol>
            </ElRow>
          </ElForm>
        </ElCard>

        <ElTable
          v-if="backwardResult.length > 0"
          :data="backwardResult"
          border
          fit
          highlight-current-row
          style="width: 100%"
          :aria-label="$t('businessTrace.table.backwardResultAriaLabel')"
        >
          <ElTableColumn prop="id" label="ID" width="80" />
          <ElTableColumn prop="trace_chain_id" :label="$t('businessTrace.table.traceChainId')" width="150" />
          <ElTableColumn prop="five_dimension_id" :label="$t('businessTrace.field.fiveDimensionId')" />
          <ElTableColumn prop="batch_no" :label="$t('businessTrace.field.batchNo')" width="120" />
          <ElTableColumn prop="color_no" :label="$t('businessTrace.field.colorNo')" width="100" />
          <ElTableColumn prop="grade" :label="$t('businessTrace.field.grade')" width="100" />
          <ElTableColumn prop="current_stage" :label="$t('businessTrace.table.currentStage')" width="120" />
          <ElTableColumn prop="current_bill_no" :label="$t('businessTrace.table.currentBillNo')" width="150" />
          <ElTableColumn prop="created_at" :label="$t('businessTrace.table.createdAt')" width="150" />
        </ElTable>

        <ElResult
          v-else
          icon="primary"
          :title="$t('businessTrace.empty.queryTitle')"
          :sub-title="$t('businessTrace.empty.backwardSubTitle')"
        />
      </ElTabPane>
    </ElTabs>
  </div>
</template>

<style scoped>
.app-container {
  padding: 20px;
}

.trace-card {
  margin-bottom: 20px;
}

.filter-item {
  width: 100%;
}

.snapshot-message {
  margin-top: 15px;
}

.success-text {
  color: #67c23a;
}

.trace-result {
  margin-top: 20px;
}

.detail-card {
  margin-bottom: 20px;
}

.flow-card {
  margin-bottom: 20px;
}

.stage-icon {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  background: #ecf5ff;
  display: flex;
  align-items: center;
  justify-content: center;
}

.stage-number {
  font-weight: bold;
  color: #409eff;
}

.w-full {
  width: 100%;
}
</style>
