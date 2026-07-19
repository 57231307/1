<!--
  customerCredit/index.vue - 客户信用管理主入口
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-3）：
  原 400+ 行"上帝组件"已拆分为：
  - tabs/RatingDialogTab.vue - 设置信用评级对话框
  - tabs/AdjustDialogTab.vue - 调整额度对话框
  - tabs/AmountDialogTab.vue - 占用/释放额度对话框

  本主入口承担：列表 + 工具栏 + 公共样式。
-->
<template>
  <div class="customer-credit">
    <el-card>
      <template #header>
        <div class="card-header">
          <span>客户信用管理</span>
        </div>
      </template>

      <div class="toolbar">
        <el-button type="primary" @click="openRatingDialog">设置信用评级</el-button>
      </div>

      <el-table :data="creditList" border stripe aria-label="客户信用列表">
        <el-table-column prop="customer_name" label="客户名称" />
        <el-table-column prop="credit_rating" label="信用等级">
          <template #default="{ row }">
            <el-tag v-if="row.credit_rating === 'AAA'" type="success">AAA</el-tag>
            <el-tag v-else-if="row.credit_rating === 'AA'" type="success">AA</el-tag>
            <el-tag v-else-if="row.credit_rating === 'A'" type="success">A</el-tag>
            <el-tag v-else-if="row.credit_rating === 'BBB'" type="warning">BBB</el-tag>
            <el-tag v-else-if="row.credit_rating === 'BB'" type="warning">BB</el-tag>
            <el-tag v-else-if="row.credit_rating === 'B'" type="warning">B</el-tag>
            <el-tag v-else type="danger">{{ row.credit_rating || '-' }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="credit_limit" label="信用额度" />
        <el-table-column prop="used_credit" label="已用额度" />
        <el-table-column prop="available_credit" label="可用额度">
          <template #default="{ row }">
            <span
              :style="{
                color: row.available_credit && row.available_credit > 0 ? '#67c23a' : '#f56c6c',
              }"
            >
              {{ row.available_credit }}
            </span>
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态">
          <template #default="{ row }">
            <el-tag v-if="row.status === 'active'" type="success">正常</el-tag>
            <el-tag v-else type="danger">停用</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" fixed="right" width="300">
          <template #default="{ row }">
            <el-button link type="primary" @click="openAdjustDialog(row)">调整额度</el-button>
            <el-button link type="primary" @click="openOccupyDialog(row)">占用额度</el-button>
            <el-button link type="primary" @click="openReleaseDialog(row)">释放额度</el-button>
            <el-button
              v-if="row.status === 'active'"
              link
              type="danger"
              @click="handleDeactivate(row)"
              >停用</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :total="total"
        :page-sizes="[10, 20, 50, 100]"
        layout="total, sizes, prev, pager, next, jumper"
        aria-label="客户信用列表分页"
        @size-change="handleSizeChange"
      />
    </el-card>

    <RatingDialogTab
      v-model="ratingDialogVisible"
      :customers="customerOptions"
      @submitted="fetchCredits"
    />

    <AdjustDialogTab
      v-model="adjustDialogVisible"
      :customer-id="currentCustomerId"
      @submitted="fetchCredits"
    />

    <AmountDialogTab
      v-model="amountDialogVisible"
      :customer-id="currentCustomerId"
      :operation-type="amountOperationType"
      @submitted="fetchCredits"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { deactivateCredit, type CustomerCredit } from '@/api/customer-credit'
import { customerApi, type Customer } from '@/api/customer'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'
import { useTableApi } from '@/composables/useTableApi'
import RatingDialogTab from './tabs/RatingDialogTab.vue'
import AdjustDialogTab from './tabs/AdjustDialogTab.vue'
import AmountDialogTab from './tabs/AmountDialogTab.vue'

const hasLoaded = createLazyLoader()

const customerOptions = ref<Customer[]>([])

// 批次 272：接入 useTableApi，消除手写 pagination/creditList/total + fetchCredits 重复
// useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: creditList,
  page,
  pageSize,
  total,
  refresh: fetchCredits,
} = useTableApi<CustomerCredit>({
  url: '/crm/customer-credits',
  onError: (e: unknown) => {
    ElMessage.error('获取信用列表失败')
    logger.warn('获取信用列表失败', String(e))
  },
})

const ratingDialogVisible = ref(false)
const adjustDialogVisible = ref(false)
const amountDialogVisible = ref(false)
const amountOperationType = ref<'occupy' | 'release'>('occupy')
const currentCustomerId = ref<number | null>(null)

// fetchCredits 由 useTableApi 的 refresh 提供（批次 272）

const fetchCustomers = async () => {
  try {
    const res = await customerApi.list({ page: 1, page_size: 100 })
    customerOptions.value = res.data?.list || []
  } catch (error) {
    customerOptions.value = []
  }
}

// 分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handleSizeChange = () => {
  page.value = 1
}

const openRatingDialog = () => {
  ratingDialogVisible.value = true
}

const openAdjustDialog = (row: CustomerCredit) => {
  if (!row.id) return
  currentCustomerId.value = row.id
  adjustDialogVisible.value = true
}

const openOccupyDialog = (row: CustomerCredit) => {
  if (!row.id) return
  currentCustomerId.value = row.id
  amountOperationType.value = 'occupy'
  amountDialogVisible.value = true
}

const openReleaseDialog = (row: CustomerCredit) => {
  if (!row.id) return
  currentCustomerId.value = row.id
  amountOperationType.value = 'release'
  amountDialogVisible.value = true
}

const handleDeactivate = async (row: CustomerCredit) => {
  if (!row.id) return

  try {
    await ElMessageBox.confirm('确认停用该客户信用？', '提示', {
      confirmButtonText: '确认',
      cancelButtonText: '取消',
      type: 'warning',
    })

    await deactivateCredit(row.id)
    ElMessage.success('停用成功')
    fetchCredits()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      ElMessage.error(err.message || '停用失败')
    }
  }
}

// 批次 272：useTableApi 构造时自动初始加载，无需 onMounted 调用 fetchCredits
onMounted(() => {
  loadIfNot('fetchCustomers', fetchCustomers, hasLoaded)
})
</script>

<style scoped>
.customer-credit .card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.customer-credit .toolbar {
  margin-bottom: 16px;
}

.customer-credit .el-table {
  margin-bottom: 16px;
}
</style>
