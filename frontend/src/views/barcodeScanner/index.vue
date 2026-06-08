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
  ElTabs,
  ElTabPane,
  ElResult,
} from 'element-plus'
import { Search, Box, Refresh } from '@element-plus/icons-vue'
import {
  scanToShip,
  scanInventory,
  getScanHistory,
  type ScanData,
  type ScanHistory,
} from '@/api/barcode-scanner'

const activeTab = ref('scan')
const barcodeInput = ref('')
const orderId = ref(0)
const scanResult = ref<ScanData | null>(null)
const scanMessage = ref('')
const scanSuccess = ref(false)
const loading = ref(false)

const shipForm = ref({
  orderId: 0,
  barcode: '',
})

const historyData = ref<ScanHistory[]>([])
const total = ref(0)
const pagination = ref({
  page: 1,
  pageSize: 20,
})

const statusOptions = [
  { label: '在库', value: 'IN_STOCK' },
  { label: '已发货', value: 'SHIPPED' },
  { label: '已报废', value: 'SCRAPPED' },
]

const getStatusLabel = (value: string) => {
  return statusOptions.find((s) => s.value === value)?.label || value
}

const handleScan = async () => {
  if (!barcodeInput.value.trim()) {
    ElMessage.warning('请输入条码')
    return
  }
  loading.value = true
  try {
    const res: any = await scanInventory(barcodeInput.value)
    scanResult.value = res.data!.data
    scanSuccess.value = true
    scanMessage.value = '扫码成功'
  } catch (error: any) {
    scanSuccess.value = false
    scanMessage.value = error.response?.data?.message || '扫码失败'
    scanResult.value = null
  } finally {
    loading.value = false
  }
}

const handleScanToShip = async () => {
  if (!barcodeInput.value.trim()) {
    ElMessage.warning('请输入条码')
    return
  }
  if (!orderId.value) {
    ElMessage.warning('请输入订单ID')
    return
  }
  loading.value = true
  try {
    const res: any = await scanToShip({
      barcode: barcodeInput.value,
      order_id: Number(orderId.value),
    })
    scanSuccess.value = true
    scanMessage.value = res.data!.data.message
    scanResult.value = null
    barcodeInput.value = ''
  } catch (error: any) {
    scanSuccess.value = false
    scanMessage.value = error.response?.data?.message || '发货失败'
  } finally {
    loading.value = false
  }
}

const loadHistory = async () => {
  loading.value = true
  try {
    const res: any = await getScanHistory(pagination.value.page - 1, pagination.value.pageSize)
    historyData.value = res.data!.items
    total.value = res.data!.total
  } catch (error) {
    ElMessage.error('加载历史失败')
  } finally {
    loading.value = false
  }
}

const handlePageChange = (page: number) => {
  pagination.value.page = page
  loadHistory()
}

const handlePageSizeChange = (pageSize: number) => {
  pagination.value.pageSize = pageSize
  loadHistory()
}

loadHistory()
</script>

<template>
  <div class="app-container">
    <ElTabs v-model="activeTab">
      <ElTabPane label="扫码查询" name="scan">
        <ElCard title="条码扫描" class="scan-card">
          <div class="scan-area">
            <div class="scan-input-area">
              <ElInput
                v-model="barcodeInput"
                placeholder="扫描或输入条码"
                class="barcode-input"
                @keyup.enter="handleScan"
              />
            </div>
            <div class="scan-actions">
              <ElButton type="primary" :loading="loading" class="scan-btn" @click="handleScan">
                <Search /> 扫码查询
              </ElButton>
            </div>
          </div>

          <div
            v-if="scanMessage"
            class="scan-result-message"
            :class="{ success: scanSuccess, error: !scanSuccess }"
          >
            {{ scanMessage }}
          </div>

          <div v-if="scanResult" class="scan-detail">
            <ElDivider />
            <h4>布卷信息</h4>
            <ElDescriptions :column="3" border>
              <ElDescriptionsItem label="条码">{{ scanResult.barcode }}</ElDescriptionsItem>
              <ElDescriptionsItem label="布卷号">{{ scanResult.piece_no }}</ElDescriptionsItem>
              <ElDescriptionsItem label="产品ID">{{ scanResult.product_id }}</ElDescriptionsItem>
              <ElDescriptionsItem label="产品名称">{{
                scanResult.product_name
              }}</ElDescriptionsItem>
              <ElDescriptionsItem label="批次号">{{ scanResult.batch_no }}</ElDescriptionsItem>
              <ElDescriptionsItem label="色号">{{ scanResult.color_no }}</ElDescriptionsItem>
              <ElDescriptionsItem label="等级">{{ scanResult.grade }}</ElDescriptionsItem>
              <ElDescriptionsItem label="米数">{{ scanResult.quantity_meters }}</ElDescriptionsItem>
              <ElDescriptionsItem label="公斤数">{{ scanResult.quantity_kg }}</ElDescriptionsItem>
              <ElDescriptionsItem label="仓库ID">{{ scanResult.warehouse_id }}</ElDescriptionsItem>
              <ElDescriptionsItem label="仓库名称">{{
                scanResult.warehouse_name
              }}</ElDescriptionsItem>
              <ElDescriptionsItem label="状态">{{
                getStatusLabel(scanResult.status)
              }}</ElDescriptionsItem>
            </ElDescriptions>
          </div>

          <ElResult
            v-if="!scanMessage && !scanResult"
            icon="info"
            title="扫码查询"
            sub-title="扫描或输入条码查询布卷信息"
          />
        </ElCard>
      </ElTabPane>

      <ElTabPane label="扫码发货" name="ship">
        <ElCard title="扫码出库" class="scan-card">
          <ElForm :model="shipForm" label-width="100px">
            <ElRow :gutter="20">
              <ElCol :span="8">
                <ElFormItem label="订单ID">
                  <ElInputNumber
                    v-model="shipForm.orderId"
                    placeholder="请输入订单ID"
                    class="w-full"
                  />
                </ElFormItem>
              </ElCol>
              <ElCol :span="12">
                <ElFormItem label="条码">
                  <ElInput
                    v-model="shipForm.barcode"
                    placeholder="扫描或输入条码"
                    class="w-full"
                    @keyup.enter="handleScanToShip"
                  />
                </ElFormItem>
              </ElCol>
              <ElCol :span="4">
                <ElButton
                  type="primary"
                  :loading="loading"
                  class="w-full"
                  style="margin-top: 24px"
                  @click="handleScanToShip"
                >
                  <Box /> 扫码发货
                </ElButton>
              </ElCol>
            </ElRow>
          </ElForm>

          <div
            v-if="scanMessage"
            class="scan-result-message"
            :class="{ success: scanSuccess, error: !scanSuccess }"
          >
            {{ scanMessage }}
          </div>

          <ElResult
            v-if="!scanMessage"
            icon="success"
            title="扫码发货"
            sub-title="输入订单ID后扫描条码完成出库"
          />
        </ElCard>
      </ElTabPane>

      <ElTabPane label="扫码历史" name="history">
        <div class="filter-actions" style="margin-bottom: 20px">
          <ElButton @click="loadHistory"> <Refresh /> 刷新 </ElButton>
        </div>

        <ElTable
    :data="historyData"
          :loading="loading"
          border
          fit
          highlight-current-row
          style="width: 100%"
        >
          <ElTableColumn prop="id" label="ID" width="80" />
          <ElTableColumn prop="barcode" label="条码" width="180" />
          <ElTableColumn prop="piece_no" label="布卷号" width="150" />
          <ElTableColumn prop="scan_type" label="扫码类型" width="120" />
          <ElTableColumn prop="result" label="结果" width="150" />
          <ElTableColumn prop="created_at" label="时间" width="180" />
        </ElTable>

    <div class="pagination-wrapper" style="margin-top: 16px; text-align: right;">
      <ElPagination
        v-model:current-page="pagination.page"
        v-model:page-size="pagination.pageSize"
        :page-sizes="[10, 20, 50, 100]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        @size-change="handlePageSizeChange"
        @current-change="handlePageChange"
      />
    </div>
      </ElTabPane>
    </ElTabs>
  </div>
</template>

<style scoped>
.app-container {
  padding: 20px;
}

.scan-card {
  margin-bottom: 20px;
}

.scan-area {
  display: flex;
  gap: 20px;
  align-items: center;
}

.scan-input-area {
  flex: 1;
}

.barcode-input {
  width: 100%;
  font-size: 18px;
}

.scan-actions {
  flex-shrink: 0;
}

.scan-btn {
  padding: 12px 32px;
  font-size: 16px;
}

.scan-result-message {
  margin-top: 20px;
  padding: 15px;
  border-radius: 4px;
  font-size: 16px;
}

.scan-result-message.success {
  background: #f0f9eb;
  color: #67c23a;
  border: 1px solid #b3e6ab;
}

.scan-result-message.error {
  background: #fef0f0;
  color: #f56c6c;
  border: 1px solid #fbc4c4;
}

.scan-detail {
  margin-top: 20px;
}

.filter-actions {
  margin-top: 10px;
}

.w-full {
  width: 100%;
}
</style>
