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
          <span>色号价格列表</span>
          <div>
            <el-button type="primary" :icon="Plus" @click="$router.push('/color-prices/create')">
              新建价格
            </el-button>
            <el-button type="success" :icon="Edit" @click="$router.push('/color-prices/batch-adjust')">
              批量调价
            </el-button>
          </div>
        </div>
      </template>

      <el-form :inline="true" :model="filterForm" class="filter-form" aria-label="色卡价格筛选表单">
        <el-form-item label="产品 ID">
          <el-input v-model.number="filterForm.product_id" placeholder="产品 ID" clearable style="width: 140px" />
        </el-form-item>
        <el-form-item label="色号 ID">
          <el-input v-model.number="filterForm.color_id" placeholder="色号 ID" clearable style="width: 140px" />
        </el-form-item>
        <el-form-item label="客户等级">
          <el-select v-model="filterForm.customer_level" placeholder="全部" clearable style="width: 120px">
            <el-option label="VIP" value="VIP" />
            <el-option label="NORMAL" value="NORMAL" />
            <el-option label="GOLD" value="GOLD" />
            <el-option label="SILVER" value="SILVER" />
          </el-select>
        </el-form-item>
        <el-form-item label="季节">
          <el-select v-model="filterForm.season" placeholder="全部" clearable style="width: 120px">
            <el-option label="春夏 SS" value="SS" />
            <el-option label="秋冬 AW" value="AW" />
            <el-option label="节日 HOLIDAY" value="HOLIDAY" />
          </el-select>
        </el-form-item>
        <el-form-item label="币种">
          <el-select v-model="filterForm.currency" placeholder="全部" clearable style="width: 100px">
            <el-option label="CNY" value="CNY" />
            <el-option label="USD" value="USD" />
            <el-option label="EUR" value="EUR" />
          </el-select>
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="filterForm.is_active" placeholder="全部" clearable style="width: 100px">
            <el-option label="启用" :value="true" />
            <el-option label="禁用" :value="false" />
          </el-select>
        </el-form-item>
        <el-form-item label="审批状态">
          <el-select v-model="filterForm.approval_status" placeholder="全部" clearable style="width: 120px">
            <el-option label="待审批" value="PENDING" />
            <el-option label="已通过" value="APPROVED" />
            <el-option label="已拒绝" value="REJECTED" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :icon="Search" @click="handleSearch">查询</el-button>
          <el-button :icon="Refresh" @click="handleReset">重置</el-button>
        </el-form-item>
      </el-form>

      <el-table :data="tableData" v-loading="loading" border stripe aria-label="色卡价格列表" @selection-change="handleSelectionChange">
        <el-table-column type="selection" width="55" />
        <el-table-column prop="id" label="ID" width="80" />
        <el-table-column prop="product_id" label="产品" width="100" />
        <el-table-column prop="color_id" label="色号" width="100" />
        <el-table-column label="客户等级" width="100">
          <template #default="{ row }">
            <el-tag v-if="row.customer_level" :type="getLevelColor(row.customer_level)">
              {{ getLevelLabel(row.customer_level) }}
            </el-tag>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column label="季节" width="100">
          <template #default="{ row }">
            <el-tag v-if="row.season" :type="getSeasonColor(row.season)">
              {{ getSeasonLabel(row.season) }}
            </el-tag>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column label="基础价" width="140">
          <template #default="{ row }">{{ formatPrice(row.base_price, row.currency) }}</template>
        </el-table-column>
        <el-table-column label="币种" width="80" prop="currency" />
        <el-table-column label="审批状态" width="100">
          <template #default="{ row }">
            <el-tag :type="getApprovalColor(row.approval_status)">
              {{ getApprovalLabel(row.approval_status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="生效日期" width="120" prop="effective_from" />
        <el-table-column label="状态" width="80">
          <template #default="{ row }">
            <el-tag :type="row.is_active ? 'success' : 'info'">
              {{ row.is_active ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" @click="handleView(row)">详情</el-button>
            <el-button link type="warning" @click="handleAdjust(row)">调价</el-button>
            <el-button link type="danger" @click="handleDelete(row)" v-if="row.is_active">删除</el-button>
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
          aria-label="色卡价格列表分页"
        />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Search, Refresh, Edit } from '@element-plus/icons-vue'
import {
  deleteColorPrice,
  formatPrice,
  getLevelLabel,
  getLevelColor,
  getSeasonLabel,
  getSeasonColor,
  getApprovalLabel,
  getApprovalColor,
  type ColorPriceListItem,
} from '@/api/color-price'
// 批次 280：接入 useTableApi，消除手写 tableData/loading/total/loadData 重复
import { useTableApi } from '@/composables/useTableApi'

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
// listColorPrices 返回 PagedResponse<T>（{ items, total }），useTableApi detectList 会取 obj.items
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
    ElMessage.error('加载失败：' + (errMsg || '未知错误'))
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
    await ElMessageBox.confirm(`确定删除色号价格 #${row.id}？`, '确认', { type: 'warning' })
    await deleteColorPrice(row.id)
    ElMessage.success('删除成功')
    loadData()
  } catch (e: unknown) {
    if (e === 'cancel') return
    // v11 批次 180 P2-1 修复：catch (e: any) 改为 catch (e: unknown) + 类型守卫
    const errMsg = e instanceof Error ? e.message : String(e)
    ElMessage.error('删除失败：' + (errMsg || '未知错误'))
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
