<!--
  报价单列表页
  - 筛选（客户/状态）
  - 表格 + 分页
  - 行操作：查看 / 编辑（draft, rejected） / 转订单（approved） / 取消（draft）
-->
<template>
  <div class="quotation-list">
    <el-card>
      <template #header>
        <div class="card-header">
          <span class="title">报价单管理</span>
          <el-button type="primary" @click="$router.push('/quotations/new')">
            <el-icon><Plus /></el-icon>
            新建报价单
          </el-button>
        </div>
      </template>

      <!-- 筛选区 -->
      <el-form :inline="true" :model="filters" class="filter-form">
        <el-form-item label="客户">
          <el-select
            v-model="filters.customer_id"
            clearable
            filterable
            placeholder="全部客户"
            style="width: 200px"
          >
            <el-option
              v-for="c in customers"
              :key="c.id"
              :label="c.customer_name || c.name"
              :value="c.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="filters.status" clearable placeholder="全部状态" style="width: 160px">
            <el-option
              v-for="(label, value) in QUOTATION_STATUS_LABELS"
              :key="value"
              :label="label"
              :value="value"
            />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSearch">查询</el-button>
          <el-button @click="handleReset">重置</el-button>
        </el-form-item>
      </el-form>

      <!-- 列表 -->
      <el-table
        v-loading="loading"
        :data="quotations"
        stripe
        border
        style="width: 100%"
        empty-text="暂无报价单"
      >
        <el-table-column prop="quotation_no" label="报价单号" width="170" />
        <el-table-column label="客户" min-width="160">
          <template #default="{ row }">
            {{ row.customer_name || row.customer_id }}
          </template>
        </el-table-column>
        <el-table-column prop="quotation_date" label="报价日期" width="120" />
        <el-table-column prop="valid_until" label="有效期" width="120" />
        <el-table-column label="价格条款" width="80">
          <template #default="{ row }">{{ row.price_terms }}</template>
        </el-table-column>
        <el-table-column label="金额" width="160" align="right">
          <template #default="{ row }">
            {{ row.currency }} {{ formatAmount(row.total_amount) }}
          </template>
        </el-table-column>
        <el-table-column label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="QUOTATION_STATUS_TAG_TYPES[row.status as QuotationStatus] as any">
              {{ QUOTATION_STATUS_LABELS[row.status as QuotationStatus] || row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="280" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" @click="goDetail(row)">查看</el-button>
            <el-button
              v-permission="'quotation:update'"
              v-if="row.status === 'draft' || row.status === 'rejected'"
              link
              type="primary"
              @click="goEdit(row)"
            >
              编辑
            </el-button>
            <el-button
              v-if="row.status === 'approved'"
              link
              type="success"
              @click="handleConvert(row)"
            >
              转订单
            </el-button>
            <el-button v-permission="'quotation:cancel'" v-if="row.status === 'draft'" link type="danger" @click="handleCancel(row)">
              取消
            </el-button>
          </template>
        </el-table-column>
      </el-table>

      <el-pagination
        v-model:current-page="pagination.page"
        v-model:page-size="pagination.pageSize"
        :page-sizes="[10, 20, 50, 100]"
        :total="pagination.total"
        layout="total, sizes, prev, pager, next, jumper"
        @current-change="loadData"
        @size-change="loadData"
      />
    </el-card>
  </div>
</template>

<script setup lang="ts">
// 报价单列表页脚本
// - 列表加载
// - 行操作：查看/编辑/转订单/取消
import { ref, reactive, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  listQuotations,
  cancelQuotation,
  convertQuotation,
  QUOTATION_STATUS_LABELS,
  QUOTATION_STATUS_TAG_TYPES,
  type QuotationResponseDto,
  type QuotationStatus,
} from '@/api/quotation'
import { listCustomers } from '@/api/customer'

const router = useRouter()
const loading = ref(false)
const quotations = ref<QuotationResponseDto[]>([])
const customers = ref<Array<{ id: number; customer_name?: string; name?: string }>>([])

const filters = reactive({
  customer_id: undefined as number | undefined,
  status: undefined as QuotationStatus | undefined,
})

const pagination = reactive({ page: 1, pageSize: 20, total: 0 })

/** 加载报价单列表 */
async function loadData() {
  loading.value = true
  try {
    const res = await listQuotations({
      page: pagination.page,
      page_size: pagination.pageSize,
      customer_id: filters.customer_id,
      status: filters.status,
    })
    const data = res.data
    // 后端可能返回 list/total 结构或纯数组
    if (Array.isArray(data)) {
      quotations.value = data
      pagination.total = data.length
    } else if (data && typeof data === 'object') {
      const obj = data as any
      quotations.value = obj.list || obj.items || []
      pagination.total = obj.total ?? quotations.value.length
    } else {
      quotations.value = []
      pagination.total = 0
    }
  } catch (e: any) {
    ElMessage.error(e?.message || '加载报价单列表失败')
    quotations.value = []
  } finally {
    loading.value = false
  }
}

/** 加载客户下拉 */
async function loadCustomers() {
  try {
    const res = await listCustomers({ page: 1, page_size: 1000 })
    const data = res.data as any
    customers.value = data?.list || data?.items || []
  } catch {
    customers.value = []
  }
}

function handleSearch() {
  pagination.page = 1
  loadData()
}

function handleReset() {
  filters.customer_id = undefined
  filters.status = undefined
  handleSearch()
}

function goDetail(row: QuotationResponseDto) {
  router.push(`/quotations/${row.id}`)
}

function goEdit(row: QuotationResponseDto) {
  router.push(`/quotations/${row.id}/edit`)
}

async function handleCancel(row: QuotationResponseDto) {
  try {
    await ElMessageBox.confirm(`确认取消报价单 ${row.quotation_no}？取消后无法恢复。`, '取消确认', {
      type: 'warning',
    })
  } catch {
    return
  }
  await cancelQuotation(row.id)
  ElMessage.success('已取消')
  loadData()
}

async function handleConvert(row: QuotationResponseDto) {
  try {
    await ElMessageBox.confirm(
      `确认将报价单 ${row.quotation_no} 转为销售订单？转订单后报价单状态将变为"已转订单"。`,
      '转订单确认',
      { type: 'warning' }
    )
  } catch {
    return
  }
  const res = await convertQuotation(row.id)
  const order = res.data
  ElMessage.success(`转订单成功，销售订单 ID：${order?.id}`)
  if (order?.id) {
    router.push(`/sales/orders/${order.id}`)
  } else {
    loadData()
  }
}

/** 金额格式化（保留 2 位 + 千分位） */
function formatAmount(value?: number): string {
  if (value === undefined || value === null) return '0.00'
  return Number(value).toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })
}

onMounted(() => {
  loadCustomers()
  loadData()
})
</script>

<style scoped>
.quotation-list {
  padding: 16px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.title {
  font-size: 18px;
  font-weight: 600;
}
.filter-form {
  margin-bottom: 16px;
}
</style>
