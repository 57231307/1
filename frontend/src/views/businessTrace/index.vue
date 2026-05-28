<script setup lang="ts">
import { ref } from 'vue'
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
    ElMessage.warning('请输入五维ID')
    return
  }
  loading.value = true
  try {
    const res: any = await getTraceByFiveDimension(fiveDimensionId.value)
    traceResult.value = res.data!
    snapshotMessage.value = ''
  } catch (error) {
    ElMessage.error('追溯失败')
  } finally {
    loading.value = false
  }
}

const handleForwardTrace = async () => {
  if (!forwardForm.value.supplier_id || !forwardForm.value.batch_no) {
    ElMessage.warning('请填写供应商ID和批次号')
    return
  }
  loading.value = true
  try {
    const res: any = await forwardTrace({
      supplier_id: Number(forwardForm.value.supplier_id),
      batch_no: forwardForm.value.batch_no,
    })
    forwardResult.value = res.data.traces
  } catch (error) {
    ElMessage.error('正向追溯失败')
  } finally {
    loading.value = false
  }
}

const handleBackwardTrace = async () => {
  if (!backwardForm.value.customer_id || !backwardForm.value.batch_no) {
    ElMessage.warning('请填写客户ID和批次号')
    return
  }
  loading.value = true
  try {
    const res: any = await backwardTrace({
      customer_id: Number(backwardForm.value.customer_id),
      batch_no: backwardForm.value.batch_no,
    })
    backwardResult.value = res.data.traces
  } catch (error) {
    ElMessage.error('反向追溯失败')
  } finally {
    loading.value = false
  }
}

const handleCreateSnapshot = async () => {
  if (!traceResult.value) {
    ElMessage.warning('请先查询追溯链')
    return
  }
  loading.value = true
  try {
    const res: any = await createTraceSnapshot(traceResult.value.trace_chain_id)
    snapshotMessage.value = res.data!
    ElMessage.success('快照创建成功')
  } catch (error) {
    ElMessage.error('创建快照失败')
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="app-container">
    <ElTabs v-model="traceMode" type="card">
      <ElTabPane label="五维追溯" name="five_dimension">
        <ElCard title="五维ID追溯" class="trace-card">
          <ElRow :gutter="20">
            <ElCol :span="16">
              <ElInput
                v-model="fiveDimensionId"
                placeholder="输入五维ID进行追溯（如：P1|B20240101|C001|D20240101001|G 一等品）"
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
                <Search /> 追溯
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
                <Camera /> 创建快照
              </ElButton>
            </ElCol>
          </ElRow>
          <div v-if="snapshotMessage" class="snapshot-message">
            <ElDivider />
            <span class="success-text">{{ snapshotMessage }}</span>
          </div>
        </ElCard>

        <div v-if="traceResult" class="trace-result">
          <ElCard title="追溯链详情" class="detail-card">
            <ElDescriptions :column="3" border>
              <ElDescriptionsItem label="追溯链ID">{{
                traceResult.trace_chain_id
              }}</ElDescriptionsItem>
              <ElDescriptionsItem label="五维ID">{{
                traceResult.five_dimension_id
              }}</ElDescriptionsItem>
              <ElDescriptionsItem label="产品ID">{{ traceResult.product_id }}</ElDescriptionsItem>
              <ElDescriptionsItem label="批次号">{{ traceResult.batch_no }}</ElDescriptionsItem>
              <ElDescriptionsItem label="色号">{{ traceResult.color_no }}</ElDescriptionsItem>
              <ElDescriptionsItem label="等级">{{ traceResult.grade }}</ElDescriptionsItem>
              <ElDescriptionsItem label="环节总数">{{
                traceResult.total_stages
              }}</ElDescriptionsItem>
              <ElDescriptionsItem label="开始时间">{{ traceResult.start_time }}</ElDescriptionsItem>
              <ElDescriptionsItem label="结束时间">{{
                traceResult.end_time || '-'
              }}</ElDescriptionsItem>
            </ElDescriptions>
          </ElCard>

          <ElCard title="追溯流程" class="flow-card">
            <ElSteps :space="80" align-center>
              <ElStep
                v-for="(stage, index) in traceResult.stages"
                :key="stage.stage_id"
                :title="stage.stage_name"
                :description="stage.bill_no"
                :status="(stageStatusMap[stage.stage_type] || 'wait') as any"
              >
                <template #icon>
                  <div class="stage-icon">
                    <span class="stage-number">{{ index + 1 }}</span>
                  </div>
                </template>
              </ElStep>
            </ElSteps>

            <ElDivider />

            <h4>环节明细</h4>
            <ElTable :data="traceResult.stages" border style="width: 100%">
              <ElTableColumn prop="stage_id" label="环节ID" width="100" />
              <ElTableColumn prop="stage_name" label="环节名称" width="120" />
              <ElTableColumn prop="bill_type" label="单据类型" width="120" />
              <ElTableColumn prop="bill_no" label="单据编号" width="150" />
              <ElTableColumn prop="warehouse_name" label="仓库" width="120" />
              <ElTableColumn prop="supplier_name" label="供应商" width="120" />
              <ElTableColumn prop="customer_name" label="客户" width="120" />
              <ElTableColumn prop="quantity_meters" label="米数" width="100" align="right" />
              <ElTableColumn prop="quantity_kg" label="公斤数" width="100" align="right" />
              <ElTableColumn prop="created_at" label="时间" width="150" />
            </ElTable>
          </ElCard>
        </div>

        <ElResult
          v-else
          icon="primary"
          title="请输入五维ID进行追溯"
          sub-title="输入五维ID后点击追溯按钮，查看完整的业务追溯链"
        />
      </ElTabPane>

      <ElTabPane label="正向追溯" name="forward">
        <ElCard title="从供应商追溯到客户" class="trace-card">
          <ElForm :model="forwardForm" label-width="100px">
            <ElRow :gutter="20">
              <ElCol :span="10">
                <ElFormItem label="供应商ID">
                  <ElInputNumber v-model="forwardForm.supplier_id" placeholder="请输入供应商ID" />
                </ElFormItem>
              </ElCol>
              <ElCol :span="10">
                <ElFormItem label="批次号">
                  <ElInput v-model="forwardForm.batch_no" placeholder="请输入批次号" />
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
                  <ArrowRightBold /> 正向追溯
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
        >
          <ElTableColumn prop="id" label="ID" width="80" />
          <ElTableColumn prop="trace_chain_id" label="追溯链ID" width="150" />
          <ElTableColumn prop="five_dimension_id" label="五维ID" />
          <ElTableColumn prop="batch_no" label="批次号" width="120" />
          <ElTableColumn prop="color_no" label="色号" width="100" />
          <ElTableColumn prop="grade" label="等级" width="100" />
          <ElTableColumn prop="current_stage" label="当前环节" width="120" />
          <ElTableColumn prop="current_bill_no" label="当前单据" width="150" />
          <ElTableColumn prop="created_at" label="创建时间" width="150" />
        </ElTable>

        <ElResult
          v-else
          icon="primary"
          title="请输入查询条件"
          sub-title="输入供应商ID和批次号，正向追溯物料流向"
        />
      </ElTabPane>

      <ElTabPane label="反向追溯" name="backward">
        <ElCard title="从客户追溯到供应商" class="trace-card">
          <ElForm :model="backwardForm" label-width="100px">
            <ElRow :gutter="20">
              <ElCol :span="10">
                <ElFormItem label="客户ID">
                  <ElInputNumber v-model="backwardForm.customer_id" placeholder="请输入客户ID" />
                </ElFormItem>
              </ElCol>
              <ElCol :span="10">
                <ElFormItem label="批次号">
                  <ElInput v-model="backwardForm.batch_no" placeholder="请输入批次号" />
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
                  <ArrowLeftBold /> 反向追溯
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
        >
          <ElTableColumn prop="id" label="ID" width="80" />
          <ElTableColumn prop="trace_chain_id" label="追溯链ID" width="150" />
          <ElTableColumn prop="five_dimension_id" label="五维ID" />
          <ElTableColumn prop="batch_no" label="批次号" width="120" />
          <ElTableColumn prop="color_no" label="色号" width="100" />
          <ElTableColumn prop="grade" label="等级" width="100" />
          <ElTableColumn prop="current_stage" label="当前环节" width="120" />
          <ElTableColumn prop="current_bill_no" label="当前单据" width="150" />
          <ElTableColumn prop="created_at" label="创建时间" width="150" />
        </ElTable>

        <ElResult
          v-else
          icon="primary"
          title="请输入查询条件"
          sub-title="输入客户ID和批次号，反向追溯物料来源"
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
