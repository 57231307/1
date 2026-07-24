<!--
  CustomerListTab.vue - CRM 客户列表 Tab
  来源：原 crm/index.vue 中 客户列表 tab 内容
  拆分日期：2026-06-15 B3-3
-->
<template>
  <div class="customer-list-tab">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">客户管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>CRM</el-breadcrumb-item>
          <el-breadcrumb-item>客户列表</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          新建客户
        </el-button>
        <el-button @click="handlePrint">
          <el-icon><Printer /></el-icon>
          打印
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          导出
        </el-button>
        <el-button @click="router.push('/crm/pool')">
          <el-icon><Coin /></el-icon>
          公海池
        </el-button>
        <el-button @click="router.push('/crm/assignment')">
          <el-icon><Share /></el-icon>
          客户分配
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form" aria-label="客户列表筛选表单">
        <el-form-item label="关键词">
          <el-input v-model="queryParams.keyword" placeholder="客户编码/名称/联系人" clearable @clear="handleQuery" @keyup.enter="handleQuery" />
        </el-form-item>
        <el-form-item label="客户类型">
          <el-select v-model="queryParams.customer_type" placeholder="选择类型" clearable>
            <el-option label="普通客户" value="normal" />
            <el-option label="VIP客户" value="vip" />
            <el-option label="批发客户" value="wholesale" />
          </el-select>
        </el-form-item>
        <el-form-item label="标签">
          <el-select v-model="queryParams.tag_id" placeholder="选择标签" clearable>
            <el-option v-for="tag in tags" :key="tag.id" :label="tag.name" :value="tag.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="queryParams.status" placeholder="选择状态" clearable>
            <el-option label="启用" value="active" />
            <el-option label="禁用" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">查询</el-button>
          <el-button @click="handleReset">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table v-loading="loading" :data="customers" stripe aria-label="客户列表">
        <el-table-column prop="customer_code" label="客户编码" width="120" fixed />
        <el-table-column prop="customer_name" label="客户名称" min-width="180" fixed>
          <template #default="{ row }">
            <el-button type="primary" link @click="viewDetail(row.id)">{{
              row.customer_name
            }}</el-button>
          </template>
        </el-table-column>
        <el-table-column prop="contact_person" label="联系人" width="100" />
        <el-table-column prop="phone" label="电话" width="130" />
        <el-table-column prop="customer_type" label="类型" width="100">
          <template #default="{ row }">
            <el-tag :type="getCustomerTypeTag(row.customer_type)" size="small">
              {{ getCustomerTypeLabel(row.customer_type) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="tags" label="标签" min-width="150">
          <template #default="{ row }">
            <el-tag
              v-for="tag in row.tags"
              :key="tag.id"
              :color="tag.color"
              size="small"
              class="table-tag"
            >
              {{ tag.name }}
            </el-tag>
            <span v-if="!row.tags.length" class="no-tags">-</span>
          </template>
        </el-table-column>
        <el-table-column prop="owner_name" label="负责人" width="100" />
        <el-table-column prop="total_amount" label="累计金额" width="120" align="right">
          <template #default="{ row }">
            {{ row.total_amount ? formatCurrency(row.total_amount) : '-' }}
          </template>
        </el-table-column>
        <el-table-column prop="last_follow_up" label="最近跟进" width="120" />
        <el-table-column prop="status" label="状态" width="80">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="viewDetail(row.id)">详情</el-button>
            <!-- P2-17 修复（批次 86 v2 复审）：编辑/删除按钮补齐 v-permission -->
            <el-button v-permission="'crm_customer:update'" type="primary" link size="small" @click="handleEdit(row)">编辑</el-button>
            <el-button v-permission="'crm_customer:delete'" type="danger" link size="small" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          aria-label="客户列表分页"
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
        />
      </div>
    </el-card>

    <!-- 新增/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="dialogTitle"
      width="700px"
      :close-on-click-modal="false"
      aria-label="客户编辑对话框"
      @close="resetForm"
    >
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px" aria-label="客户信息表单">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="客户编码" prop="customer_code">
              <el-input v-model="formData.customer_code" placeholder="请输入客户编码" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="客户名称" prop="customer_name">
              <el-input v-model="formData.customer_name" placeholder="请输入客户名称" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="联系人" prop="contact_person">
              <el-input v-model="formData.contact_person" placeholder="请输入联系人" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="电话" prop="phone">
              <el-input v-model="formData.phone" placeholder="请输入电话" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="邮箱" prop="email">
              <el-input v-model="formData.email" placeholder="请输入邮箱" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="客户类型" prop="customer_type">
              <el-select
                v-model="formData.customer_type"
                placeholder="请选择类型"
                style="width: 100%"
              >
                <el-option label="普通客户" value="normal" />
                <el-option label="VIP客户" value="vip" />
                <el-option label="批发客户" value="wholesale" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="地址" prop="address">
          <el-input v-model="formData.address" placeholder="请输入地址" />
        </el-form-item>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="税号" prop="tax_number">
              <el-input v-model="formData.tax_number" placeholder="请输入税号" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="信用额度" prop="credit_limit">
              <el-input-number
                v-model="formData.credit_limit"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="开户银行" prop="bank_name">
              <el-input v-model="formData.bank_name" placeholder="请输入开户银行" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="银行账号" prop="bank_account">
              <el-input v-model="formData.bank_account" placeholder="请输入银行账号" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="状态" prop="status">
          <el-radio-group v-model="formData.status">
            <el-radio value="active">启用</el-radio>
            <el-radio value="inactive">禁用</el-radio>
          </el-radio-group>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { Plus, Coin, Share, Download, Printer } from '@element-plus/icons-vue'
// D14 Batch 5b：原 crmEnhancedApi 对象已转风格 B 函数
import { getCrmTagList, deleteCustomer, updateCustomer, createCustomer, type CustomerTag, type CustomerWithTags } from '@/api/crm-enhanced'
import { useTableApi } from '@/composables/useTableApi'
// V15 P0-S12 修复（Batch 475b）：导出改用后端带水印 xlsx 接口
// 后端 GET /crm/customers/export 已就绪（Batch 474 注入水印 + 行级数据权限 + 异步审计日志）
import { exportFromBackend } from '@/utils/export'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'
import { escapeHtml } from '@/utils/print'

const hasLoaded = createLazyLoader()

const router = useRouter()
const submitLoading = ref(false)
const dialogVisible = ref(false)
const isEdit = ref(false)
const formRef = ref<FormInstance>()
const tags = ref<CustomerTag[]>([])

// 筛选条件（仅筛选字段，分页由 useTableApi 管理）
const queryParams = reactive({
  keyword: '',
  customer_type: '',
  status: '',
  tag_id: undefined as number | undefined,
})

// 批次 277：接入 useTableApi，消除手写 customers/total/loading/fetchCustomerList 重复
// useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: customers,
  loading,
  page,
  pageSize,
  total,
  refresh: fetchCustomerList,
  setQueryParam,
} = useTableApi<CustomerWithTags>({
  url: '/crm/customers/enhanced',
  onError: (err: unknown) =>
    ElMessage.error((err instanceof Error ? err.message : String(err)) || '获取客户列表失败'),
})

// 批次 277：同步筛选条件到 useTableApi.queryParams 并刷新
const syncQueryParams = () => {
  setQueryParam('keyword', queryParams.keyword || undefined)
  setQueryParam('customer_type', queryParams.customer_type || undefined)
  setQueryParam('status', queryParams.status || undefined)
  setQueryParam('tag_id', queryParams.tag_id)
}

const formData = reactive({
  id: undefined as number | undefined,
  customer_code: '',
  customer_name: '',
  contact_person: '',
  phone: '',
  email: '',
  address: '',
  customer_type: 'normal',
  tax_number: '',
  credit_limit: 0,
  bank_name: '',
  bank_account: '',
  status: 'active',
})

const formRules: FormRules = {
  customer_code: [{ required: true, message: '请输入客户编码', trigger: 'blur' }],
  customer_name: [{ required: true, message: '请输入客户名称', trigger: 'blur' }],
  contact_person: [{ required: true, message: '请输入联系人', trigger: 'blur' }],
  phone: [
    { required: true, message: '请输入电话', trigger: 'blur' },
    { pattern: /^1[3-9]\d{9}$/, message: '请输入正确的手机号', trigger: 'blur' },
  ],
}

const dialogTitle = computed(() => (isEdit.value ? '编辑客户' : '新建客户'))

const formatCurrency = (amount: number) => `¥${(amount || 0).toFixed(2)}`

const getCustomerTypeLabel = (type: string) => {
  const labels: Record<string, string> = {
    normal: '普通客户',
    vip: 'VIP客户',
    wholesale: '批发客户',
  }
  return labels[type] || type
}

const getCustomerTypeTag = (type: string) => {
  const typeMap: Record<string, string> = {
    normal: '',
    vip: 'warning',
    wholesale: 'success',
  }
  return typeMap[type] || ''
}

const fetchTags = async () => {
  try {
    const res = await getCrmTagList()
    tags.value = res.data || []
  } catch (error) {
    tags.value = []
  }
}

// 批次 277：查询 - 同步筛选条件并回到首页重载
const handleQuery = () => {
  syncQueryParams()
  page.value = 1
  fetchCustomerList()
}

// 批次 277：重置 - 清空筛选并回到首页重载
const handleReset = () => {
  queryParams.keyword = ''
  queryParams.customer_type = ''
  queryParams.status = ''
  queryParams.tag_id = undefined
  syncQueryParams()
  page.value = 1
  fetchCustomerList()
}

// 批次 277：分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handlePageChange = (p: number) => {
  page.value = p
}

const handleSizeChange = (s: number) => {
  pageSize.value = s
  page.value = 1
}

const resetForm = () => {
  formData.id = undefined
  formData.customer_code = ''
  formData.customer_name = ''
  formData.contact_person = ''
  formData.phone = ''
  formData.email = ''
  formData.address = ''
  formData.customer_type = 'normal'
  formData.tax_number = ''
  formData.credit_limit = 0
  formData.bank_name = ''
  formData.bank_account = ''
  formData.status = 'active'
  formRef.value?.clearValidate()
}

const handleCreate = () => {
  resetForm()
  isEdit.value = false
  dialogVisible.value = true
}

const handleEdit = (row: CustomerWithTags) => {
  resetForm()
  Object.assign(formData, row)
  isEdit.value = true
  dialogVisible.value = true
}

const handleDelete = async (row: CustomerWithTags) => {
  try {
    await ElMessageBox.confirm(`确定删除客户 "${row.customer_name}" 吗？`, '删除确认', {
      type: 'warning',
    })
    await deleteCustomer(row.id)
    ElMessage.success('删除成功')
    fetchCustomerList()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || '删除失败')
    }
  }
}

const handleSubmit = async () => {
  if (!formRef.value) return

  await formRef.value.validate(async valid => {
    if (!valid) return

    submitLoading.value = true
    try {
      if (isEdit.value) {
        await updateCustomer(formData.id as number, formData)
        ElMessage.success('更新成功')
      } else {
        await createCustomer(formData)
        ElMessage.success('创建成功')
      }
      dialogVisible.value = false
      fetchCustomerList()
    } catch (error) {
      const err = error as Error
      ElMessage.error(err.message || '操作失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const viewDetail = (id: number) => {
  router.push(`/crm/detail/${id}`)
}

/**
 * 导出客户列表为 xlsx（V15 P0-S12 修复 Batch 475b）
 *
 * 规则 3：导出统一使用 xlsx 格式
 * 改为调用后端 GET /crm/customers/export，后端注入水印 + 行级数据权限 + 异步审计日志
 * 传入当前列表筛选条件（status/customer_type/keyword），保证导出与列表一致
 */
const handleExport = async () => {
  const params: Record<string, unknown> = {
    status: queryParams.status || undefined,
    customer_type: queryParams.customer_type || undefined,
    keyword: queryParams.keyword.trim() || undefined,
  }
  await exportFromBackend('/crm/customers/export', params, 'crm_customers_export')
}

const handlePrint = () => {
  const printWindow = window.open('', '_blank')
  if (!printWindow) {
    ElMessage.error('无法打开打印窗口')
    return
  }
  const rows = customers.value
    .map(
      item => `
    <tr>
      <td>${escapeHtml(item.customer_code)}</td>
      <td>${escapeHtml(item.customer_name)}</td>
      <td>${escapeHtml(item.contact_person)}</td>
      <td>${escapeHtml(item.phone)}</td>
      <td>${escapeHtml(getCustomerTypeLabel(item.customer_type))}</td>
      <td>${escapeHtml(item.owner_name || '-')}</td>
      <td style="text-align:right">${item.total_amount ? '¥' + item.total_amount.toLocaleString() : '-'}</td>
      <td>${escapeHtml(item.status === 'active' ? '启用' : '禁用')}</td>
    </tr>
  `
    )
    .join('')
  const now = new Date().toISOString().split('T')[0]
  printWindow.document.write(`
    <html><head><meta charset="utf-8"><title>CRM客户列表</title>
    <style>
      @media print { @page { size: landscape; } }
      body { font-family: "Microsoft YaHei", sans-serif; font-size: 12px; }
      h1 { text-align: center; }
      table { width: 100%; border-collapse: collapse; margin-top: 12px; }
      th, td { border: 1px solid #333; padding: 6px 8px; }
      th { background: #f5f5f5; }
      .meta { text-align: center; color: #666; font-size: 11px; }
    </style></head><body>
    <h1>CRM客户列表</h1>
    <div class="meta">打印日期: ${now} | 共 ${customers.value.length} 条</div>
    <table>
      <thead><tr><th>客户编码</th><th>客户名称</th><th>联系人</th><th>电话</th><th>类型</th><th>负责人</th><th>累计金额</th><th>状态</th></tr></thead>
      <tbody>${rows}</tbody>
    </table>
    </body></html>
  `)
  printWindow.document.close()
  printWindow.onload = () => printWindow.print()
  logger.info('客户列表打印任务已生成')
}

onMounted(() => {
  loadIfNot('fetchTags', fetchTags, hasLoaded)
  // 批次 277：客户列表由 useTableApi 在 setup 阶段自动加载，无需在此手动调用
})
</script>
