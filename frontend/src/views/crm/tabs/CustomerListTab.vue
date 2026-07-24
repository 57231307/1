<!--
  CustomerListTab.vue - CRM 客户列表 Tab
  来源：原 crm/index.vue 中 客户列表 tab 内容
  拆分日期：2026-06-15 B3-3
  D05 Batch 4：接入 useI18n，所有硬编码中文迁移到 locales/zh-CN.ts + en-US.ts
-->
<template>
  <div class="customer-list-tab">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">{{ $t('crmCustomer.title') }}</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">{{ $t('crmCustomer.breadcrumb.home') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ $t('crmCustomer.breadcrumb.crm') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ $t('crmCustomer.breadcrumb.customerList') }}</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          {{ $t('crmCustomer.create') }}
        </el-button>
        <el-button @click="handlePrint">
          <el-icon><Printer /></el-icon>
          {{ $t('crmCustomer.print') }}
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          {{ $t('crmCustomer.export') }}
        </el-button>
        <el-button @click="router.push('/crm/pool')">
          <el-icon><Coin /></el-icon>
          {{ $t('crmCustomer.pool') }}
        </el-button>
        <el-button @click="router.push('/crm/assignment')">
          <el-icon><Share /></el-icon>
          {{ $t('crmCustomer.assignment') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form" :aria-label="$t('crmCustomer.filter.ariaLabel')">
        <el-form-item :label="$t('crmCustomer.filter.keyword')">
          <el-input v-model="queryParams.keyword" :placeholder="$t('crmCustomer.filter.keywordPlaceholder')" clearable @clear="handleQuery" @keyup.enter="handleQuery" />
        </el-form-item>
        <el-form-item :label="$t('crmCustomer.filter.customerType')">
          <el-select v-model="queryParams.customer_type" :placeholder="$t('crmCustomer.filter.customerTypePlaceholder')" clearable>
            <el-option :label="$t('crmCustomer.customerType.normal')" value="normal" />
            <el-option :label="$t('crmCustomer.customerType.vip')" value="vip" />
            <el-option :label="$t('crmCustomer.customerType.wholesale')" value="wholesale" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('crmCustomer.filter.tag')">
          <el-select v-model="queryParams.tag_id" :placeholder="$t('crmCustomer.filter.tagPlaceholder')" clearable>
            <el-option v-for="tag in tags" :key="tag.id" :label="tag.name" :value="tag.id" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('crmCustomer.filter.status')">
          <el-select v-model="queryParams.status" :placeholder="$t('crmCustomer.filter.statusPlaceholder')" clearable>
            <el-option :label="$t('crmCustomer.status.active')" value="active" />
            <el-option :label="$t('crmCustomer.status.inactive')" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">{{ $t('crmCustomer.filter.query') }}</el-button>
          <el-button @click="handleReset">{{ $t('crmCustomer.filter.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table v-loading="loading" :data="customers" stripe :aria-label="$t('crmCustomer.table.ariaLabel')">
        <el-table-column prop="customer_code" :label="$t('crmCustomer.table.customerCode')" width="120" fixed />
        <el-table-column prop="customer_name" :label="$t('crmCustomer.table.customerName')" min-width="180" fixed>
          <template #default="{ row }">
            <el-button type="primary" link @click="viewDetail(row.id)">{{
              row.customer_name
            }}</el-button>
          </template>
        </el-table-column>
        <el-table-column prop="contact_person" :label="$t('crmCustomer.table.contactPerson')" width="100" />
        <el-table-column prop="phone" :label="$t('crmCustomer.table.phone')" width="130" />
        <el-table-column prop="customer_type" :label="$t('crmCustomer.table.type')" width="100">
          <template #default="{ row }">
            <el-tag :type="getCustomerTypeTag(row.customer_type)" size="small">
              {{ getCustomerTypeLabel(row.customer_type) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="tags" :label="$t('crmCustomer.table.tag')" min-width="150">
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
        <el-table-column prop="owner_name" :label="$t('crmCustomer.table.owner')" width="100" />
        <el-table-column prop="total_amount" :label="$t('crmCustomer.table.totalAmount')" width="120" align="right">
          <template #default="{ row }">
            {{ row.total_amount ? formatCurrency(row.total_amount) : '-' }}
          </template>
        </el-table-column>
        <el-table-column prop="last_follow_up" :label="$t('crmCustomer.table.lastFollowUp')" width="120" />
        <el-table-column prop="status" :label="$t('crmCustomer.table.status')" width="80">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? $t('crmCustomer.status.active') : $t('crmCustomer.status.inactive') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="$t('crmCustomer.table.operation')" width="180" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="viewDetail(row.id)">{{ $t('crmCustomer.table.detail') }}</el-button>
            <!-- P2-17 修复（批次 86 v2 复审）：编辑/删除按钮补齐 v-permission -->
            <el-button v-permission="'crm_customer:update'" type="primary" link size="small" @click="handleEdit(row)">{{ $t('crmCustomer.table.edit') }}</el-button>
            <el-button v-permission="'crm_customer:delete'" type="danger" link size="small" @click="handleDelete(row)">{{ $t('crmCustomer.table.delete') }}</el-button>
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
          :aria-label="$t('crmCustomer.table.paginationAriaLabel')"
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
      :aria-label="$t('crmCustomer.dialog.ariaLabel')"
      @close="resetForm"
    >
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px" :aria-label="$t('crmCustomer.dialog.formAriaLabel')">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('crmCustomer.dialog.customerCode')" prop="customer_code">
              <el-input v-model="formData.customer_code" :placeholder="$t('crmCustomer.dialog.customerCodePlaceholder')" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('crmCustomer.dialog.customerName')" prop="customer_name">
              <el-input v-model="formData.customer_name" :placeholder="$t('crmCustomer.dialog.customerNamePlaceholder')" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('crmCustomer.dialog.contactPerson')" prop="contact_person">
              <el-input v-model="formData.contact_person" :placeholder="$t('crmCustomer.dialog.contactPersonPlaceholder')" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('crmCustomer.dialog.phone')" prop="phone">
              <el-input v-model="formData.phone" :placeholder="$t('crmCustomer.dialog.phonePlaceholder')" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('crmCustomer.dialog.email')" prop="email">
              <el-input v-model="formData.email" :placeholder="$t('crmCustomer.dialog.emailPlaceholder')" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('crmCustomer.dialog.customerType')" prop="customer_type">
              <el-select
                v-model="formData.customer_type"
                :placeholder="$t('crmCustomer.dialog.customerTypePlaceholder')"
                style="width: 100%"
              >
                <el-option :label="$t('crmCustomer.customerType.normal')" value="normal" />
                <el-option :label="$t('crmCustomer.customerType.vip')" value="vip" />
                <el-option :label="$t('crmCustomer.customerType.wholesale')" value="wholesale" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item :label="$t('crmCustomer.dialog.address')" prop="address">
          <el-input v-model="formData.address" :placeholder="$t('crmCustomer.dialog.addressPlaceholder')" />
        </el-form-item>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('crmCustomer.dialog.taxNumber')" prop="tax_number">
              <el-input v-model="formData.tax_number" :placeholder="$t('crmCustomer.dialog.taxNumberPlaceholder')" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('crmCustomer.dialog.creditLimit')" prop="credit_limit">
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
            <el-form-item :label="$t('crmCustomer.dialog.bankName')" prop="bank_name">
              <el-input v-model="formData.bank_name" :placeholder="$t('crmCustomer.dialog.bankNamePlaceholder')" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('crmCustomer.dialog.bankAccount')" prop="bank_account">
              <el-input v-model="formData.bank_account" :placeholder="$t('crmCustomer.dialog.bankAccountPlaceholder')" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item :label="$t('crmCustomer.dialog.status')" prop="status">
          <el-radio-group v-model="formData.status">
            <el-radio value="active">{{ $t('crmCustomer.status.active') }}</el-radio>
            <el-radio value="inactive">{{ $t('crmCustomer.status.inactive') }}</el-radio>
          </el-radio-group>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ $t('crmCustomer.dialog.cancel') }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{ $t('crmCustomer.dialog.save') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
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

const { t } = useI18n({ useScope: 'global' })

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
    ElMessage.error((err instanceof Error ? err.message : String(err)) || t('crmCustomer.message.loadFailed')),
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

const formRules = computed<FormRules>(() => ({
  customer_code: [{ required: true, message: t('crmCustomer.validation.customerCodeRequired'), trigger: 'blur' }],
  customer_name: [{ required: true, message: t('crmCustomer.validation.customerNameRequired'), trigger: 'blur' }],
  contact_person: [{ required: true, message: t('crmCustomer.validation.contactPersonRequired'), trigger: 'blur' }],
  phone: [
    { required: true, message: t('crmCustomer.validation.phoneRequired'), trigger: 'blur' },
    { pattern: /^1[3-9]\d{9}$/, message: t('crmCustomer.validation.phonePattern'), trigger: 'blur' },
  ],
}))

const dialogTitle = computed(() => (isEdit.value ? t('crmCustomer.dialog.editTitle') : t('crmCustomer.dialog.createTitle')))

const formatCurrency = (amount: number) => `¥${(amount || 0).toFixed(2)}`

// D05 Batch 4：getCustomerTypeLabel 改为函数返回，使 t() 在每次渲染时响应式求值
const getCustomerTypeLabel = (type: string) => {
  const labels: Record<string, string> = {
    normal: t('crmCustomer.customerType.normal'),
    vip: t('crmCustomer.customerType.vip'),
    wholesale: t('crmCustomer.customerType.wholesale'),
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
    await ElMessageBox.confirm(t('crmCustomer.message.deleteConfirm', { name: row.customer_name }), t('crmCustomer.message.deleteConfirmTitle'), {
      type: 'warning',
    })
    await deleteCustomer(row.id)
    ElMessage.success(t('crmCustomer.message.deleteSuccess'))
    fetchCustomerList()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || t('crmCustomer.message.deleteFailed'))
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
        ElMessage.success(t('crmCustomer.message.updateSuccess'))
      } else {
        await createCustomer(formData)
        ElMessage.success(t('crmCustomer.message.createSuccess'))
      }
      dialogVisible.value = false
      fetchCustomerList()
    } catch (error) {
      const err = error as Error
      ElMessage.error(err.message || t('crmCustomer.message.operationFailed'))
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
    ElMessage.error(t('crmCustomer.message.printWindowFailed'))
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
      <td>${escapeHtml(item.status === 'active' ? t('crmCustomer.status.active') : t('crmCustomer.status.inactive'))}</td>
    </tr>
  `
    )
    .join('')
  const now = new Date().toISOString().split('T')[0]
  printWindow.document.write(`
    <html><head><meta charset="utf-8"><title>${t('crmCustomer.print.title')}</title>
    <style>
      @media print { @page { size: landscape; } }
      body { font-family: "Microsoft YaHei", sans-serif; font-size: 12px; }
      h1 { text-align: center; }
      table { width: 100%; border-collapse: collapse; margin-top: 12px; }
      th, td { border: 1px solid #333; padding: 6px 8px; }
      th { background: #f5f5f5; }
      .meta { text-align: center; color: #666; font-size: 11px; }
    </style></head><body>
    <h1>${t('crmCustomer.print.title')}</h1>
    <div class="meta">${t('crmCustomer.print.date')}: ${now} | ${t('crmCustomer.print.total', { count: customers.value.length })}</div>
    <table>
      <thead><tr><th>${t('crmCustomer.table.customerCode')}</th><th>${t('crmCustomer.table.customerName')}</th><th>${t('crmCustomer.table.contactPerson')}</th><th>${t('crmCustomer.table.phone')}</th><th>${t('crmCustomer.table.type')}</th><th>${t('crmCustomer.table.owner')}</th><th>${t('crmCustomer.table.totalAmount')}</th><th>${t('crmCustomer.table.status')}</th></tr></thead>
      <tbody>${rows}</tbody>
    </table>
    </body></html>
  `)
  printWindow.document.close()
  printWindow.onload = () => printWindow.print()
  logger.info(t('crmCustomer.print.logMessage'))
}

onMounted(() => {
  loadIfNot('fetchTags', fetchTags, hasLoaded)
  // 批次 277：客户列表由 useTableApi 在 setup 阶段自动加载，无需在此手动调用
})
</script>
