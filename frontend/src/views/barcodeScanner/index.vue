<script setup lang="ts">
import { ref, computed } from 'vue'
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
  ElDescriptionsItem,
  ElCard,
  ElTabs,
  ElTabPane,
  ElResult,
  ElDivider,
  ElPagination,
} from 'element-plus'
import { Search, Box, Refresh } from '@element-plus/icons-vue'
import {
  scanToShip,
  scanInventory,
  type ScanData,
  type ScanHistory,
} from '@/api/barcode-scanner'
import type { ApiResponse } from '@/types/api'
import { useTableApi } from '@/composables/useTableApi'
import { logger } from '@/utils/logger'

const { t } = useI18n({ useScope: 'global' })

const activeTab = ref('scan')
const barcodeInput = ref('')
const orderId = ref(0)
const scanResult = ref<ScanData | null>(null)
const scanMessage = ref('')
const scanSuccess = ref(false)
// scan/ship tab 独立 loading（不接入 useTableApi）
const scanLoading = ref(false)

const shipForm = ref({
  orderId: 0,
  barcode: '',
})

// 批次 390：history tab 接入 useTableApi，修复 0-based 分页 bug
// 原代码第 118 行 getScanHistory(pagination.value.page - 1, ...) 为 0-based 分页，
// 与后端 page.unwrap_or(1).clamp(1,1000) + page.saturating_sub(1)*page_size 约定不一致。
// useTableApi 使用 1-based 分页，直接传 page 给后端，无需 -1 转换。
// 后端返回 { data: { items: [], total: 0 } }，items 在自动探测列表中，无需配置 listKey。
const {
  data: historyData,
  total,
  loading: historyLoading,
  page,
  pageSize,
  refresh: loadHistory,
} = useTableApi<ScanHistory>({
  url: '/scanner/history',
  defaultPageSize: 20,
  onError: (err: unknown) => {
    logger.error(t('barcodeScanner.message.fetchHistoryFailed'), err)
    ElMessage.error(t('barcodeScanner.message.fetchHistoryFailed'))
  },
})

const statusOptions = computed(() => [
  { label: t('barcodeScanner.status.inStock'), value: 'IN_STOCK' },
  { label: t('barcodeScanner.status.shipped'), value: 'SHIPPED' },
  { label: t('barcodeScanner.status.scrapped'), value: 'SCRAPPED' },
])

const getStatusLabel = (value: string) => {
  return statusOptions.value.find(s => s.value === value)?.label || value
}

const handleScan = async () => {
  if (!barcodeInput.value.trim()) {
    ElMessage.warning(t('barcodeScanner.message.barcodeRequired'))
    return
  }
  scanLoading.value = true
  try {
    // v11 批次 146 P1-3 修复：拦截器已返回 ApiResponse 完整对象，
    // res.data 即业务数据（ScanResult/ScanData），无需 res.data!.data 双层访问
    const res = await scanInventory(barcodeInput.value)
    const data = (res as ApiResponse<ScanData> | undefined)?.data
    scanResult.value = data ?? null
    scanSuccess.value = true
    scanMessage.value = t('barcodeScanner.message.scanSuccess')
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    scanSuccess.value = false
    scanMessage.value = (error as { response?: { data?: { message?: string } } }).response?.data?.message || t('barcodeScanner.message.scanFailed')
    scanResult.value = null
  } finally {
    scanLoading.value = false
  }
}

const handleScanToShip = async () => {
  if (!barcodeInput.value.trim()) {
    ElMessage.warning(t('barcodeScanner.message.barcodeRequired'))
    return
  }
  if (!orderId.value) {
    ElMessage.warning(t('barcodeScanner.message.orderIdRequired'))
    return
  }
  scanLoading.value = true
  try {
    // v11 批次 146 P1-3 修复：拦截器已返回 ApiResponse 完整对象，
    // res.data 即业务数据（ScanToShipResponse），无需 res.data!.data 双层访问
    const res = await scanToShip({
      barcode: barcodeInput.value,
      order_id: Number(orderId.value),
    })
    const data = (res as ApiResponse<{ message?: string }> | undefined)?.data
    scanSuccess.value = true
    scanMessage.value = data?.message || t('barcodeScanner.message.shipSuccess')
    scanResult.value = null
    barcodeInput.value = ''
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    scanSuccess.value = false
    scanMessage.value = (error as { response?: { data?: { message?: string } } }).response?.data?.message || t('barcodeScanner.message.shipFailed')
  } finally {
    scanLoading.value = false
  }
}

// 批次 390：history tab 由 useTableApi setup 自动加载，无需手动调 loadHistory()
</script>

<template>
  <div class="app-container">
    <ElTabs v-model="activeTab">
      <ElTabPane :label="$t('barcodeScanner.tabs.scan')" name="scan">
        <ElCard :title="$t('barcodeScanner.scan.cardTitle')" class="scan-card">
          <div class="scan-area">
            <div class="scan-input-area">
              <ElInput
                v-model="barcodeInput"
                :placeholder="$t('barcodeScanner.scan.placeholder')"
                class="barcode-input"
                @keyup.enter="handleScan"
              />
            </div>
            <div class="scan-actions">
              <ElButton type="primary" :loading="scanLoading" class="scan-btn" @click="handleScan">
                <Search /> {{ $t('barcodeScanner.scan.button') }}
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
            <h4>{{ $t('barcodeScanner.scan.pieceInfo') }}</h4>
            <ElDescriptions :column="3" border>
              <ElDescriptionsItem :label="$t('barcodeScanner.detail.barcode')">{{ scanResult.barcode }}</ElDescriptionsItem>
              <ElDescriptionsItem :label="$t('barcodeScanner.detail.pieceNo')">{{ scanResult.piece_no }}</ElDescriptionsItem>
              <ElDescriptionsItem :label="$t('barcodeScanner.detail.productId')">{{ scanResult.product_id }}</ElDescriptionsItem>
              <ElDescriptionsItem :label="$t('barcodeScanner.detail.productName')">{{
                scanResult.product_name
              }}</ElDescriptionsItem>
              <ElDescriptionsItem :label="$t('barcodeScanner.detail.batchNo')">{{ scanResult.batch_no }}</ElDescriptionsItem>
              <ElDescriptionsItem :label="$t('barcodeScanner.detail.colorNo')">{{ scanResult.color_no }}</ElDescriptionsItem>
              <ElDescriptionsItem :label="$t('barcodeScanner.detail.grade')">{{ scanResult.grade }}</ElDescriptionsItem>
              <ElDescriptionsItem :label="$t('barcodeScanner.detail.quantityMeters')">{{ scanResult.quantity_meters }}</ElDescriptionsItem>
              <ElDescriptionsItem :label="$t('barcodeScanner.detail.quantityKg')">{{ scanResult.quantity_kg }}</ElDescriptionsItem>
              <ElDescriptionsItem :label="$t('barcodeScanner.detail.warehouseId')">{{ scanResult.warehouse_id }}</ElDescriptionsItem>
              <ElDescriptionsItem :label="$t('barcodeScanner.detail.warehouseName')">{{
                scanResult.warehouse_name
              }}</ElDescriptionsItem>
              <ElDescriptionsItem :label="$t('barcodeScanner.detail.status')">{{
                getStatusLabel(scanResult.status)
              }}</ElDescriptionsItem>
            </ElDescriptions>
          </div>

          <ElResult
            v-if="!scanMessage && !scanResult"
            icon="info"
            :title="$t('barcodeScanner.scan.resultTitle')"
            :sub-title="$t('barcodeScanner.scan.resultSubTitle')"
          />
        </ElCard>
      </ElTabPane>

      <ElTabPane :label="$t('barcodeScanner.tabs.ship')" name="ship">
        <ElCard :title="$t('barcodeScanner.ship.cardTitle')" class="scan-card">
          <ElForm :model="shipForm" label-width="100px" :aria-label="$t('barcodeScanner.ship.formAriaLabel')">
            <ElRow :gutter="20">
              <ElCol :span="8">
                <ElFormItem :label="$t('barcodeScanner.ship.orderId')">
                  <ElInputNumber
                    v-model="shipForm.orderId"
                    :placeholder="$t('barcodeScanner.ship.orderIdPlaceholder')"
                    class="w-full"
                  />
                </ElFormItem>
              </ElCol>
              <ElCol :span="12">
                <ElFormItem :label="$t('barcodeScanner.ship.barcode')">
                  <ElInput
                    v-model="shipForm.barcode"
                    :placeholder="$t('barcodeScanner.scan.placeholder')"
                    class="w-full"
                    @keyup.enter="handleScanToShip"
                  />
                </ElFormItem>
              </ElCol>
              <ElCol :span="4">
                <ElButton
                  type="primary"
                  :loading="scanLoading"
                  class="w-full"
                  style="margin-top: 24px"
                  @click="handleScanToShip"
                >
                  <Box /> {{ $t('barcodeScanner.ship.button') }}
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
            :title="$t('barcodeScanner.ship.resultTitle')"
            :sub-title="$t('barcodeScanner.ship.resultSubTitle')"
          />
        </ElCard>
      </ElTabPane>

      <ElTabPane :label="$t('barcodeScanner.tabs.history')" name="history">
        <div class="filter-actions" style="margin-bottom: 20px">
          <ElButton @click="loadHistory"> <Refresh /> {{ $t('barcodeScanner.history.refresh') }} </ElButton>
        </div>

        <ElTable
          :data="historyData"
          :loading="historyLoading"
          border
          fit
          highlight-current-row
          style="width: 100%"
          :aria-label="$t('barcodeScanner.history.tableAriaLabel')"
        >
          <ElTableColumn prop="id" :label="$t('barcodeScanner.history.id')" width="80" />
          <ElTableColumn prop="barcode" :label="$t('barcodeScanner.history.barcode')" width="180" />
          <ElTableColumn prop="piece_no" :label="$t('barcodeScanner.history.pieceNo')" width="150" />
          <ElTableColumn prop="scan_type" :label="$t('barcodeScanner.history.scanType')" width="120" />
          <ElTableColumn prop="result" :label="$t('barcodeScanner.history.result')" width="150" />
          <ElTableColumn prop="created_at" :label="$t('barcodeScanner.history.time')" width="180" />
        </ElTable>

        <!-- 批次 390：分页由 useTableApi watch 自动加载，v-model 双向绑定 page/pageSize -->
        <div class="pagination-wrapper" style="margin-top: 16px; text-align: right">
          <ElPagination
            v-model:current-page="page"
            v-model:page-size="pageSize"
            :page-sizes="[10, 20, 50, 100]"
            :total="total"
            layout="total, sizes, prev, pager, next, jumper"
            :aria-label="$t('barcodeScanner.history.paginationAriaLabel')"
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
