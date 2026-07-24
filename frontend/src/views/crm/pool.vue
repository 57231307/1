<!--
  crm/pool.vue - 客户公海池主入口
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-3）：
  原 485 行"上帝组件"已拆分为：
  - tabs/ClaimDialogTab.vue - 领取对话框
  - tabs/TransferDialogTab.vue - 分配对话框
  - tabs/ReleaseDialogTab.vue - 释放对话框

  本主入口承担：列表 + 工具栏 + 公共样式。
-->
<template>
  <div class="pool-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">客户公海池</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>CRM</el-breadcrumb-item>
          <el-breadcrumb-item>公海池</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleClaimSelected">
          <el-icon><Plus /></el-icon>
          批量领取
        </el-button>
        <el-button @click="router.push('/crm')">
          <el-icon><Back /></el-icon>
          返回客户列表
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form" aria-label="公海客户筛选表单">
        <el-form-item label="关键词">
          <el-input
            v-model="queryParams.keyword"
            placeholder="客户名称/联系人/电话"
            clearable
            @clear="handleQuery"
            @keyup.enter="handleQuery"
          />
        </el-form-item>
        <el-form-item label="客户类型">
          <el-select
            v-model="queryParams.customer_type"
            placeholder="选择类型"
            clearable
            @change="handleQuery"
          >
            <el-option label="普通客户" value="normal" />
            <el-option label="VIP客户" value="vip" />
            <el-option label="批发客户" value="wholesale" />
          </el-select>
        </el-form-item>
        <el-form-item label="在池天数">
          <el-select
            v-model="queryParams.daysInPool"
            placeholder="选择天数"
            clearable
            @change="handleQuery"
          >
            <el-option label="1周内" value="7" />
            <el-option label="1月内" value="30" />
            <el-option label="3月内" value="90" />
            <el-option label="3月以上" value="91" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">
            <el-icon><Search /></el-icon>
            查询
          </el-button>
          <el-button @click="handleReset">
            <el-icon><Refresh /></el-icon>
            重置
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table
        v-loading="loading"
        :data="poolList"
        border
        stripe
        aria-label="公海客户列表"
        @selection-change="handleSelectionChange"
      >
        <el-table-column type="selection" width="55" align="center" />
        <el-table-column type="index" label="序号" width="60" align="center" />
        <el-table-column
          prop="customer_name"
          label="客户名称"
          min-width="150"
          show-overflow-tooltip
        />
        <el-table-column prop="contact_person" label="联系人" width="100" show-overflow-tooltip />
        <el-table-column prop="phone" label="电话" width="120" show-overflow-tooltip />
        <el-table-column prop="customer_type" label="类型" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getCustomerTypeTag(row.customer_type)" size="small">
              {{ getCustomerTypeLabel(row.customer_type) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="released_at" label="入池时间" width="160" align="center" />
        <el-table-column prop="released_by_name" label="释放人" width="100" show-overflow-tooltip />
        <el-table-column prop="days_in_pool" label="在池天数" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getDaysTag(row.days_in_pool)" size="small">
              {{ row.days_in_pool }} 天
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="release_reason"
          label="释放原因"
          min-width="150"
          show-overflow-tooltip
        />
        <el-table-column label="操作" width="240" align="center" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="openClaimDialog(row)"
              >领取</el-button
            >
            <el-button type="primary" link size="small" @click="openTransferDialog(row)"
              >分配</el-button
            >
            <el-button
              v-if="row.previous_owner_id"
              type="warning"
              link
              size="small"
              @click="openReleaseDialog(row)"
              >重新释放</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-container">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          aria-label="公海客户列表分页"
          @size-change="handleSizeChange"
          @current-change="handleCurrentChange"
        />
      </div>
    </el-card>

    <ClaimDialogTab
      v-model="claimDialogVisible"
      :customer-name="currentCustomerName"
      :customer-id="currentCustomerId"
      @submitted="getList"
    />

    <TransferDialogTab
      v-model="transferDialogVisible"
      :customer-name="currentCustomerName"
      :customer-id="currentCustomerId"
      :users="users"
      @submitted="getList"
    />

    <ReleaseDialogTab
      v-model="releaseDialogVisible"
      :customer-name="currentCustomerName"
      :customer-id="currentCustomerId"
      @submitted="getList"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { Plus, Back, Search, Refresh } from '@element-plus/icons-vue'
import { getUserList, type User } from '@/api/user'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'
import { type PoolCustomer } from '@/api/crm-enhanced'
import { useTableApi } from '@/composables/useTableApi'
import ClaimDialogTab from './tabs/ClaimDialogTab.vue'
import TransferDialogTab from './tabs/TransferDialogTab.vue'
import ReleaseDialogTab from './tabs/ReleaseDialogTab.vue'

const hasLoaded = createLazyLoader()

const router = useRouter()
const queryParams = reactive({
  keyword: '',
  customer_type: '',
  daysInPool: '',
})

// 批次 269：接入 useTableApi，消除手写分页重复 + 修复原硬编码参数 bug
const {
  data: poolList,
  loading,
  page,
  pageSize,
  total,
  refresh: getList,
  setQueryParam,
} = useTableApi<PoolCustomer>({
  url: '/crm/pool',
  onError: (e: unknown) => logger.warn('加载公海池列表失败', String(e)),
})

const users = ref<User[]>([])

const claimDialogVisible = ref(false)
const transferDialogVisible = ref(false)
const releaseDialogVisible = ref(false)
const currentCustomerId = ref<number | null>(null)
const currentCustomerName = ref('')

const fetchUsers = async () => {
  try {
    const res = await getUserList()
    users.value = res.data?.list || []
  } catch (error) {
    users.value = []
  }
}

const handleQuery = () => {
  setQueryParam('keyword', queryParams.keyword || undefined)
  setQueryParam('customer_type', queryParams.customer_type || undefined)
  page.value = 1
  getList()
}

const handleReset = () => {
  queryParams.keyword = ''
  queryParams.customer_type = ''
  queryParams.daysInPool = ''
  handleQuery()
}

const openClaimDialog = (row: { id: number; customer_name: string }) => {
  currentCustomerId.value = row.id
  currentCustomerName.value = row.customer_name
  claimDialogVisible.value = true
}

const openTransferDialog = (row: { id: number; customer_name: string }) => {
  currentCustomerId.value = row.id
  currentCustomerName.value = row.customer_name
  transferDialogVisible.value = true
}

const openReleaseDialog = (row: { id: number; customer_name: string }) => {
  currentCustomerId.value = row.id
  currentCustomerName.value = row.customer_name
  releaseDialogVisible.value = true
}

const handleClaimSelected = () => {
  ElMessage.info('请勾选需要领取的客户')
}

const handleSelectionChange = () => {
  // 选区变化
}

const handleSizeChange = (val: number) => {
  pageSize.value = val
  page.value = 1
}

const handleCurrentChange = (val: number) => {
  page.value = val
}

const getCustomerTypeLabel = (type: string) => {
  const labelMap: Record<string, string> = {
    normal: '普通客户',
    vip: 'VIP客户',
    wholesale: '批发客户',
  }
  return labelMap[type] || type
}

const getCustomerTypeTag = (type: string) => {
  const typeMap: Record<string, string> = { normal: '', vip: 'warning', wholesale: 'success' }
  return typeMap[type] || ''
}

const getDaysTag = (days: number) => {
  if (days > 90) return 'danger'
  if (days > 30) return 'warning'
  return 'success'
}

onMounted(() => {
  loadIfNot('users', fetchUsers, hasLoaded)
})
</script>

<style scoped>
.pool-page {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.header-left {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.page-title {
  margin: 0;
  font-size: 24px;
  font-weight: 600;
}

.header-actions {
  display: flex;
  gap: 10px;
}

.filter-card {
  margin-bottom: 20px;
}

.filter-form {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.table-card {
  margin-bottom: 20px;
}

.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 20px;
}
</style>
