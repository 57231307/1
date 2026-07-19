<!--
  crm/assignment.vue - 客户分配规则主入口
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-3）：
  原 400+ 行"上帝组件"已拆分为：
  - tabs/RuleDialogTab.vue - 新建/编辑规则对话框
  - tabs/ManualAssignDialogTab.vue - 手动分配对话框

  本主入口承担：列表 + 工具栏 + 公共样式。
-->
<template>
  <div class="assignment-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">客户分配规则</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>CRM</el-breadcrumb-item>
          <el-breadcrumb-item>分配规则</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="openCreateRuleDialog">
          <el-icon><Plus /></el-icon>
          新建规则
        </el-button>
      </div>
    </div>

    <el-tabs v-model="activeTab" class="assignment-tabs">
      <el-tab-pane label="分配规则" name="rules">
        <el-card shadow="hover">
          <el-table v-loading="ruleLoading" :data="ruleList" border stripe aria-label="分配规则列表">
            <el-table-column type="index" label="序号" width="60" align="center" />
            <el-table-column prop="name" label="规则名称" min-width="150" show-overflow-tooltip />
            <el-table-column prop="strategy" label="分配策略" width="120" align="center">
              <template #default="{ row }">
                <el-tag>{{ getStrategyLabel(row.strategy) }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column
              prop="user_names"
              label="分配对象"
              min-width="200"
              show-overflow-tooltip
            />
            <el-table-column prop="priority" label="优先级" width="100" align="center" />
            <el-table-column prop="enabled" label="状态" width="100" align="center">
              <template #default="{ row }">
                <el-tag v-if="row.enabled" type="success">启用</el-tag>
                <el-tag v-else type="info">禁用</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="updated_at" label="更新时间" width="160" align="center" />
            <el-table-column label="操作" width="200" align="center" fixed="right">
              <template #default="{ row }">
                <!-- P2-17 修复（批次 86 v2 复审）：编辑/删除按钮补齐 v-permission -->
                <el-button v-permission="'crm_assignment:update'" type="primary" link size="small" @click="openEditRuleDialog(row)"
                  >编辑</el-button
                >
                <el-button v-permission="'crm_assignment:delete'" type="danger" link size="small" @click="handleDeleteRule(row)"
                  >删除</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="手动分配" name="manual">
        <el-card shadow="hover">
          <div class="toolbar">
            <el-form :inline="true" :model="assignQuery" class="filter-form" aria-label="待分配客户筛选表单">
              <el-form-item label="关键词">
                <el-input
                  v-model="assignQuery.keyword"
                  placeholder="客户名称/联系人"
                  clearable
                  @clear="fetchAssignableCustomers"
                  @keyup.enter="fetchAssignableCustomers"
                />
              </el-form-item>
              <el-form-item>
                <el-button type="primary" @click="fetchAssignableCustomers">查询</el-button>
              </el-form-item>
            </el-form>
          </div>

          <el-table v-loading="assignLoading" :data="assignableCustomers" border stripe aria-label="待分配客户列表">
            <el-table-column type="index" label="序号" width="60" align="center" />
            <el-table-column
              prop="customer_name"
              label="客户名称"
              min-width="150"
              show-overflow-tooltip
            />
            <el-table-column
              prop="contact_person"
              label="联系人"
              width="100"
              show-overflow-tooltip
            />
            <el-table-column prop="phone" label="电话" width="120" show-overflow-tooltip />
            <el-table-column
              prop="owner_name"
              label="当前负责人"
              width="100"
              show-overflow-tooltip
            />
            <el-table-column label="操作" width="120" align="center" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="openAssignDialog(row)"
                  >分配</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>
    </el-tabs>

    <RuleDialogTab
      v-model="ruleDialogVisible"
      :title="ruleDialogTitle"
      :row-data="currentRuleRow"
      :users="users"
      @submitted="fetchRules"
    />

    <ManualAssignDialogTab
      v-model="assignDialogVisible"
      :customer-name="currentCustomerName"
      :customer-id="currentCustomerId"
      :users="users"
      @submitted="fetchAssignableCustomers"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import { listUsers, type User } from '@/api/user'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'
import { crmEnhancedApi, type AssignableCustomer } from '@/api/crm-enhanced'
import RuleDialogTab from './tabs/RuleDialogTab.vue'
import ManualAssignDialogTab from './tabs/ManualAssignDialogTab.vue'

const hasLoaded = createLazyLoader()

const activeTab = ref('rules')
const ruleLoading = ref(false)
const ruleList = ref<unknown[]>([])

const assignLoading = ref(false)
const assignableCustomers = ref<unknown[]>([])
const assignQuery = reactive({ keyword: '' })

const users = ref<User[]>([])

interface RuleRow {
  id?: number
  name?: string
  strategy?: string
  userIds?: number[]
  priority?: number
  enabled?: boolean
  remark?: string
}

const ruleDialogVisible = ref(false)
const ruleDialogTitle = ref('新建规则')
const currentRuleRow = ref<RuleRow | null>(null)
const assignDialogVisible = ref(false)
const currentCustomerId = ref<number | null>(null)
const currentCustomerName = ref('')

const fetchRules = async () => {
  ruleLoading.value = true
  try {
    // P1-5：调用真实 API 获取分配规则（后端使用 recycle-rules 接口承载规则）
    const res = await crmEnhancedApi.getRecycleRules()
    // crm API 不嵌套 .data（直接返回 data），保留 ?? 容错
    ruleList.value = (res.data ?? res) as unknown as RuleRow[]
  } catch (error) {
    const err = error as Error
    logger.warn('获取分配规则失败', err.message)
  } finally {
    ruleLoading.value = false
  }
}

const fetchAssignableCustomers = async () => {
  assignLoading.value = true
  try {
    // P1-5：调用真实 API 获取可分配客户（公海池）
    const res = await crmEnhancedApi.getPoolList({ page: 1, page_size: 50 })
    assignableCustomers.value = (res.data?.list ?? res.data) as AssignableCustomer[]
  } catch (error) {
    const err = error as Error
    logger.warn('获取可分配客户失败', err.message)
  } finally {
    assignLoading.value = false
  }
}

const fetchUsers = async () => {
  try {
    const res = await listUsers()
    users.value = res.data?.list || []
  } catch (error) {
    users.value = []
  }
}

const openCreateRuleDialog = () => {
  currentRuleRow.value = null
  ruleDialogTitle.value = '新建规则'
  ruleDialogVisible.value = true
}

const openEditRuleDialog = (row: RuleRow) => {
  currentRuleRow.value = row
  ruleDialogTitle.value = '编辑规则'
  ruleDialogVisible.value = true
}

const openAssignDialog = (row: { id: number; customer_name: string }) => {
  currentCustomerId.value = row.id
  currentCustomerName.value = row.customer_name
  assignDialogVisible.value = true
}

const handleDeleteRule = async (row: { id: number; name: string }) => {
  try {
    await ElMessageBox.confirm(`确定删除规则 "${row.name}" 吗？`, '删除确认', {
      type: 'warning',
    })
    ElMessage.success('删除成功')
    fetchRules()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || '删除失败')
    }
  }
}

const getStrategyLabel = (strategy: string) => {
  const labelMap: Record<string, string> = {
    average: '平均分配',
    region: '按地域分配',
    industry: '按行业分配',
    scale: '按客户规模',
  }
  return labelMap[strategy] || strategy
}

onMounted(() => {
  fetchRules()
  loadIfNot('users', fetchUsers, hasLoaded)
})
</script>

<style scoped>
.assignment-page {
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

.assignment-tabs {
  background: #fff;
  border-radius: 4px;
}

.toolbar {
  margin-bottom: 16px;
}

.filter-form {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}
</style>
