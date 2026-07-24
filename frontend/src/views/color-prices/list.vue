<!--
  面料多色号定价扩展 - 列表页
  色号价格列表（分页 + 多维过滤）
  创建时间: 2026-06-18
-->
<template>
  <div class="color-price-list">
    <el-card>
      <template #header>
        <div class="card-header">
          <span>{{ $t('colorPrices.list.title') }}</span>
          <div>
            <el-button type="primary" :icon="Plus" @click="$router.push('/color-prices/create')">
              {{ $t('colorPrices.list.createPrice') }}
            </el-button>
            <el-button type="success" :icon="Edit" @click="$router.push('/color-prices/batch-adjust')">
              {{ $t('colorPrices.list.batchAdjust') }}
            </el-button>
          </div>
        </div>
      </template>

      <el-form :inline="true" :model="filterForm" class="filter-form" :aria-label="$t('colorPrices.list.filter.ariaLabel')">
        <el-form-item :label="$t('colorPrices.list.filter.productId')">
          <el-input v-model.number="filterForm.product_id" :placeholder="$t('colorPrices.list.filter.productId')" clearable style="width: 140px" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.list.filter.colorId')">
          <el-input v-model.number="filterForm.color_id" :placeholder="$t('colorPrices.list.filter.colorId')" clearable style="width: 140px" />
        </el-form-item>
        <el-form-item :label="$t('colorPrices.list.filter.customerLevel')">
          <el-select v-model="filterForm.customer_level" :placeholder="$t('colorPrices.common.all')" clearable style="width: 120px">
            <el-option :label="$t('colorPrices.customerLevel.VIP')" value="VIP" />
            <el-option :label="$t('colorPrices.customerLevel.NORMAL')" value="NORMAL" />
            <el-option :label="$t('colorPrices.customerLevel.GOLD')" value="GOLD" />
            <el-option :label="$t('colorPrices.customerLevel.SILVER')" value="SILVER" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('colorPrices.list.filter.season')">
          <el-select v-model="filterForm.season" :placeholder="$t('colorPrices.common.all')" clearable style="width: 120px">
            <el-option :label="$t('colorPrices.season.SS')" value="SS" />
            <el-option :label="$t('colorPrices.season.AW')" value="AW" />
            <el-option :label="$t('colorPrices.season.HOLIDAY')" value="HOLIDAY" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('colorPrices.list.filter.currency')">
          <el-select v-model="filterForm.currency" :placeholder="$t('colorPrices.common.all')" clearable style="width: 100px">
            <el-option :label="$t('colorPrices.currency.CNY')" value="CNY" />
            <el-option :label="$t('colorPrices.currency.USD')" value="USD" />
            <el-option :label="$t('colorPrices.currency.EUR')" value="EUR" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('colorPrices.list.filter.status')">
          <el-select v-model="filterForm.is_active" :placeholder="$t('colorPrices.common.all')" clearable style="width: 100px">
            <el-option :label="$t('colorPrices.common.enable')" :value="true" />
            <el-option :label="$t('colorPrices.common.disable')" :value="false" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('colorPrices.list.filter.approvalStatus')">
          <el-select v-model="filterForm.approval_status" :placeholder="$t('colorPrices.common.all')" clearable style="width: 120px">
            <el-option :label="$t('colorPrices.approvalStatus.PENDING')" value="PENDING" />
            <el-option :label="$t('colorPrices.approvalStatus.APPROVED')" value="APPROVED" />
            <el-option :label="$t('colorPrices.approvalStatus.REJECTED')" value="REJECTED" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :icon="Search" @click="handleSearch">{{ $t('colorPrices.common.search') }}</el-button>
          <el-button :icon="Refresh" @click="handleReset">{{ $t('colorPrices.common.reset') }}</el-button>
        </el-form-item>
      </el-form>

      <el-table :data="tableData" v-loading="loading" border stripe :aria-label="$t('colorPrices.list.table.ariaLabel')" @selection-change="handleSelectionChange">
        <el-table-column type="selection" width="55" />
        <el-table-column prop="id" :label="$t('colorPrices.list.table.id')" width="80" />
        <el-table-column prop="product_id" :label="$t('colorPrices.list.table.product')" width="100" />
        <el-table-column prop="color_id" :label="$t('colorPrices.list.table.color')" width="100" />
        <el-table-column :label="$t('colorPrices.list.table.customerLevel')" width="100">
          <template #default="{ row }">
            <el-tag v-if="row.customer_level" :type="getLevelColor(row.customer_level)">
              {{ getLevelLabel(row.customer_level) }}
            </el-tag>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column :label="$t('colorPrices.list.table.season')" width="100">
          <template #default="{ row }">
            <el-tag v-if="row.season" :type="getSeasonColor(row.season)">
              {{ getSeasonLabel(row.season) }}
            </el-tag>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column :label="$t('colorPrices.list.table.basePrice')" width="140">
          <template #default="{ row }">{{ formatPrice(row.base_price, row.currency) }}</template>
        </el-table-column>
        <el-table-column :label="$t('colorPrices.list.table.currency')" width="80" prop="currency" />
        <el-table-column :label="$t('colorPrices.list.table.approvalStatus')" width="100">
          <template #default="{ row }">
            <el-tag :type="getApprovalColor(row.approval_status)">
              {{ getApprovalLabel(row.approval_status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="$t('colorPrices.list.table.effectiveFrom')" width="120" prop="effective_from" />
        <el-table-column :label="$t('colorPrices.list.table.status')" width="80">
          <template #default="{ row }">
            <el-tag :type="row.is_active ? 'success' : 'info'">
              {{ row.is_active ? $t('colorPrices.common.enable') : $t('colorPrices.common.disable') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="$t('colorPrices.list.table.operation')" width="180" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" @click="handleView(row)">{{ $t('colorPrices.common.detail') }}</el-button>
            <el-button link type="warning" @click="handleAdjust(row)">{{ $t('colorPrices.list.table.adjust') }}</el-button>
            <el-button link type="danger" @click="handleDelete(row)" v-if="row.is_active">{{ $t('colorPrices.common.delete') }}</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :total="total"
          :page-sizes="[10, 20, 50, 100]"
          layout="total, sizes, prev, pager, next, jumper"
          :aria-label="$t('colorPrices.list.table.paginationAriaLabel')"
        />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Search, Refresh, Edit } from '@element-plus/icons-vue'
import {
  deleteColorPrice,
  formatPrice,
  getLevelColor,
  getSeasonColor,
  getApprovalColor,
  type ColorPriceListItem,
} from '@/api/color-price'
// 批次 280：接入 useTableApi，消除手写 tableData/loading/total/loadData 重复
import { useTableApi } from '@/composables/useTableApi'

const { t } = useI18n({ useScope: 'global' })

// 状态码 → 本地化标签（响应式：随语言切换自动更新）
const getLevelLabel = (level: string | null | undefined) => t(`colorPrices.customerLevel.${level || 'default'}`)
const getSeasonLabel = (season: string | null | undefined) => t(`colorPrices.season.${season || 'default'}`)
const getApprovalLabel = (status: string) => t(`colorPrices.approvalStatus.${status}`)

const router = useRouter()
const selectedRows = ref<ColorPriceListItem[]>([])

// 批次 280：filterForm 仅保留筛选字段，分页字段由 useTableApi 管理
const filterForm = reactive({
  product_id: undefined as number | undefined,
  color_id: undefined as number | undefined,
  customer_level: undefined as string | undefined,
  season: undefined as string | undefined,
  currency: undefined as string | undefined,
  is_active: undefined as boolean | undefined,
  approval_status: undefined as string | undefined,
})

// 批次 280：useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
// getColorPriceList 返回 PagedResponse<T>（{ items, total }），useTableApi detectList 会取 obj.items
const {
  data: tableData,
  loading,
  page,
  pageSize,
  total,
  refresh: loadData,
  setQueryParam,
} = useTableApi<ColorPriceListItem>({
  url: '/color-prices',
  onError: (err: unknown) => {
    // v11 批次 180 P2-1 修复：unknown + 类型守卫
    const errMsg = err instanceof Error ? err.message : String(err)
    ElMessage.error(t('colorPrices.message.loadFailed', { msg: errMsg || t('colorPrices.message.unknownError') }))
  },
})

// 批次 280：同步筛选条件到 useTableApi.queryParams 并刷新
const syncQueryParams = () => {
  setQueryParam('product_id', filterForm.product_id || undefined)
  setQueryParam('color_id', filterForm.color_id || undefined)
  setQueryParam('customer_level', filterForm.customer_level || undefined)
  setQueryParam('season', filterForm.season || undefined)
  setQueryParam('currency', filterForm.currency || undefined)
  setQueryParam('is_active', filterForm.is_active)
  setQueryParam('approval_status', filterForm.approval_status || undefined)
}

const handleSearch = () => {
  syncQueryParams()
  page.value = 1
  loadData()
}

const handleReset = () => {
  filterForm.product_id = undefined
  filterForm.color_id = undefined
  filterForm.customer_level = undefined
  filterForm.season = undefined
  filterForm.currency = undefined
  filterForm.is_active = undefined
  filterForm.approval_status = undefined
  handleSearch()
}

const handleView = (row: ColorPriceListItem) => {
  router.push(`/color-prices/detail/${row.id}`)
}

const handleAdjust = (row: ColorPriceListItem) => {
  router.push(`/color-prices/batch-adjust?ids=${row.id}`)
}

const handleDelete = async (row: ColorPriceListItem) => {
  try {
    await ElMessageBox.confirm(t('colorPrices.message.deleteConfirm', { id: row.id }), t('colorPrices.common.confirm'), { type: 'warning' })
    await deleteColorPrice(row.id)
    ElMessage.success(t('colorPrices.message.deleteSuccess'))
    loadData()
  } catch (e: unknown) {
    if (e === 'cancel') return
    // v11 批次 180 P2-1 修复：catch (e: any) 改为 catch (e: unknown) + 类型守卫
    const errMsg = e instanceof Error ? e.message : String(e)
    ElMessage.error(t('colorPrices.message.deleteFailed', { msg: errMsg || t('colorPrices.message.unknownError') }))
  }
}

const handleSelectionChange = (rows: ColorPriceListItem[]) => {
  selectedRows.value = rows
}
</script>

<style scoped>
.color-price-list { padding: 20px; }
.card-header { display: flex; justify-content: space-between; align-items: center; }
.filter-form { margin-bottom: 16px; }
.pagination-wrapper { margin-top: 16px; text-align: right; }
</style>
