<!--
  定制订单列表页
  - 筛选（客户/状态/关键词）
  - V2Table 表格 + 分页
  - 行操作：查看 / 跟踪 / 推进 / 取消
-->
<template>
  <div class="custom-order-list">
    <el-card>
      <template #header>
        <div class="card-header">
          <span class="title">定制订单管理</span>
          <el-button type="primary" @click="$router.push('/custom-orders/new')">
            <el-icon><Plus /></el-icon>
            新建定制订单
          </el-button>
        </div>
      </template>

      <!-- 筛选区 -->
      <el-form :inline="true" :model="filters" class="filter-form" aria-label="定制订单筛选表单">
        <el-form-item label="状态">
          <el-select v-model="filters.status" clearable placeholder="全部状态" style="width: 180px">
            <el-option
              v-for="(label, value) in STATUS_LABELS"
              :key="value"
              :label="label"
              :value="value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="关键词">
          <el-input v-model="filters.keyword" placeholder="订单号" clearable style="width: 200px" />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSearch">查询</el-button>
          <el-button @click="handleReset">重置</el-button>
        </el-form-item>
      </el-form>

      <!-- 列表 -->
      <el-table
        v-loading="loading"
        :data="orders"
        stripe
        border
        style="width: 100%"
        empty-text="暂无定制订单"
        aria-label="定制订单列表"
      >
        <el-table-column prop="order_no" label="订单号" width="180" />
        <el-table-column prop="spec" label="规格" min-width="150" show-overflow-tooltip />
        <el-table-column label="数量" width="100" align="right">
          <template #default="{ row }">
            {{ row.quantity }} {{ row.unit }}
          </template>
        </el-table-column>
        <el-table-column label="金额" width="140" align="right">
          <template #default="{ row }">
            <span v-if="row.total_amount">{{ row.currency }} {{ formatAmount(row.total_amount) }}</span>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column label="状态" width="120" align="center">
          <template #default="{ row }">
            <el-tag :type="STATUS_COLORS[row.status] || 'info'">
              {{ STATUS_LABELS[row.status] || row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="期望交付" width="120">
          <template #default="{ row }">
            {{ row.expected_delivery_date || '-' }}
          </template>
        </el-table-column>
        <!-- v3 复审 P2-4：新增备注列，使用 show-overflow-tooltip 处理长文本 -->
        <el-table-column
          prop="notes"
          label="备注"
          min-width="160"
          show-overflow-tooltip
        >
          <template #default="{ row }">
            {{ row.notes || '-' }}
          </template>
        </el-table-column>
        <el-table-column prop="created_at" label="创建时间" width="170" />
        <el-table-column label="操作" width="240" fixed="right">
          <template #default="{ row }">
            <el-button size="small" link @click="goDetail(row.id)">详情</el-button>
            <el-button size="small" link type="primary" @click="goTracking(row.id)">跟踪</el-button>
            <el-button
              v-if="row.status !== 'completed' && row.status !== 'cancelled'"
              size="small"
              link
              type="success"
              @click="handleAdvance(row)"
            >
              推进
            </el-button>
            <el-button
              v-if="row.status === 'draft'"
              size="small"
              link
              type="danger"
              @click="handleCancel(row)"
            >
              取消
            </el-button>
          </template>
        </el-table-column>
      </el-table>

      <!-- 分页 -->
      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :total="total"
        :page-sizes="[10, 20, 50, 100]"
        layout="total, sizes, prev, pager, next, jumper"
        @current-change="handleCurrentChange"
        @size-change="handleSizeChange"
        style="margin-top: 16px; text-align: right"
        aria-label="定制订单列表分页"
      />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  advanceCustomOrder,
  cancelCustomOrder,
  CUSTOM_ORDER_STATUS as STATUS_LABELS,
  CUSTOM_ORDER_STATUS_COLORS as STATUS_COLORS,
} from '@/api/custom-order'
import type { CustomOrderListItem } from '@/api/custom-order'
// 批次 94 P2-12 修复：导入 useUserStore 用于获取真实操作人 ID（原硬编码为 1）
import { useUserStore } from '@/store/user'
import logger from '@/utils/logger'
import { useTableApi } from '@/composables/useTableApi'

const router = useRouter()
// 批次 94 P2-12 修复：获取用户 store 以读取当前登录用户 ID
const userStore = useUserStore()
const filters = ref({ status: '', keyword: '' })

// 批次 274：接入 useTableApi，消除手写 orders/loading/pagination.total + loadData 重复
// useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: orders,
  loading,
  page,
  pageSize,
  total,
  refresh: loadData,
  setQueryParam,
} = useTableApi<CustomOrderListItem>({
  url: '/custom-orders',
  listKey: 'items',
  onError: () => {
    logger.error('加载定制订单失败')
    ElMessage.error('加载定制订单失败')
  },
})

function formatAmount(val: number | string | null | undefined) {
  if (val === null || val === undefined) return '0.00'
  return Number(val).toFixed(2)
}

// 批次 274：同步筛选条件到 useTableApi.queryParams 并刷新
// useTableApi 自动 watch page/pageSize 变化触发重载，无需手动 loadData
function syncQueryParams() {
  setQueryParam('status', filters.value.status || undefined)
  setQueryParam('keyword', filters.value.keyword || undefined)
}

function handleSearch() {
  syncQueryParams()
  page.value = 1
  loadData()
}

function handleReset() {
  filters.value = { status: '', keyword: '' }
  syncQueryParams()
  page.value = 1
  loadData()
}

// 分页（useTableApi 自动 watch page/pageSize 变化触发重载）
function handleSizeChange(s: number) {
  pageSize.value = s
  page.value = 1
}

function handleCurrentChange(p: number) {
  page.value = p
}

function goDetail(id: number) {
  router.push(`/custom-orders/${id}`)
}

function goTracking(id: number) {
  router.push(`/custom-orders/${id}/track`)
}

async function handleAdvance(row: CustomOrderListItem) {
  try {
    await ElMessageBox.confirm(`确定推进订单 ${row.order_no} 到下一阶段？`, '确认推进', {
      type: 'warning',
    })
    // 批次 94 P2-12 修复：原硬编码 operator_id: 1，改为从 userStore 获取真实当前用户 ID
    const operatorId = userStore.userInfo?.id
    if (!operatorId) {
      ElMessage.error('无法获取当前用户信息，请重新登录后重试')
      return
    }
    await advanceCustomOrder(row.id, { operator_id: operatorId, notes: '状态推进' })
    ElMessage.success('推进成功')
    loadData()
  } catch (e: unknown) {
    if (e !== 'cancel') {
      const msg = e instanceof Error ? e.message : String(e)
      ElMessage.error(msg || '推进失败')
    }
  }
}

async function handleCancel(row: CustomOrderListItem) {
  try {
    const { value: reason } = await ElMessageBox.prompt('请输入取消原因', '取消定制订单', {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      inputPattern: /\S+/,
      inputErrorMessage: '原因不能为空',
    })
    await cancelCustomOrder(row.id, reason)
    ElMessage.success('取消成功')
    loadData()
  } catch (e: unknown) {
    if (e !== 'cancel') {
      const msg = e instanceof Error ? e.message : String(e)
      ElMessage.error(msg || '取消失败')
    }
  }
}

// 批次 274：useTableApi 构造时自动初始加载，无需 onMounted 调用 loadData
</script>

<style scoped>
.custom-order-list {
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
