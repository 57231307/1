<template>
  <div class="crm-page">
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
        <el-button @click="$router.push('/crm/pool')">
          <el-icon><Coin /></el-icon>
          公海池
        </el-button>
        <el-button @click="$router.push('/crm/assignment')">
          <el-icon><Share /></el-icon>
          客户分配
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form">
        <el-form-item label="关键词">
          <el-input v-model="queryParams.keyword" placeholder="客户编码/名称/联系人" clearable />
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
      <el-tabs v-model="activeTab" @tab-change="handleTabChange">
        <el-tab-pane label="客户列表" name="list">
          <el-table v-loading="loading" :data="customers" stripe>
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
                <el-button type="primary" link size="small" @click="viewDetail(row.id)"
                  >详情</el-button
                >
                <el-button type="primary" link size="small" @click="handleEdit(row)"
                  >编辑</el-button
                >
                <el-button type="danger" link size="small" @click="handleDelete(row)"
                  >删除</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>

        <el-tab-pane label="客户分级 (RFM)" name="rfm">
          <div class="rfm-section">
            <el-row :gutter="20" class="mb-20">
              <el-col v-for="(count, level) in rfmDistribution" :key="level" :span="4">
                <el-card shadow="hover" class="rfm-card">
                  <div class="rfm-card-content">
                    <span class="rfm-card-level">{{ level }}</span>
                    <span class="rfm-card-count">{{ count }} 人</span>
                  </div>
                </el-card>
              </el-col>
            </el-row>

            <el-table v-loading="rfmLoading" :data="rfmCustomers" stripe>
              <el-table-column prop="customer_code" label="客户编码" width="120" />
              <el-table-column prop="customer_name" label="客户名称" min-width="180">
                <template #default="{ row }">
                  <el-button type="primary" link @click="viewDetail(row.id)">{{
                    row.customer_name
                  }}</el-button>
                </template>
              </el-table-column>
              <el-table-column prop="owner_name" label="负责人" width="100" />
              <el-table-column prop="rfm_score.level" label="等级" width="80" align="center">
                <template #default="{ row }">
                  <el-tag :type="getRfmLevelTag(row.rfm_score?.level)" size="small">
                    {{ row.rfm_score?.level || '-' }}
                  </el-tag>
                </template>
              </el-table-column>
              <el-table-column prop="rfm_score.label" label="分级" width="100" />
              <el-table-column prop="rfm_score.recency" label="R" width="80" align="center" />
              <el-table-column prop="rfm_score.frequency" label="F" width="80" align="center" />
              <el-table-column prop="rfm_score.monetary" label="M" width="80" align="center" />
              <el-table-column prop="total_amount" label="累计金额" width="120" align="right">
                <template #default="{ row }">
                  {{ row.total_amount ? formatCurrency(row.total_amount) : '-' }}
                </template>
              </el-table-column>
              <el-table-column prop="total_orders" label="订单数" width="80" align="center" />
              <el-table-column label="操作" width="100" fixed="right">
                <template #default="{ row }">
                  <el-button type="primary" link size="small" @click="viewDetail(row.id)"
                    >详情</el-button
                  >
                </template>
              </el-table-column>
            </el-table>
          </div>
        </el-tab-pane>
      </el-tabs>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="queryParams.page"
          v-model:page-size="queryParams.page_size"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="handleQuery"
          @current-change="handleQuery"
        />
      </div>
    </el-card>

    <!-- 新增/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="dialogTitle"
      width="700px"
      :close-on-click-modal="false"
      @close="resetForm"
    >
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px">
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
import { ref, reactive, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { Plus, Coin, Share } from '@element-plus/icons-vue'
import crmEnhancedApi, { type CustomerTag, type CustomerWithTags } from '@/api/crm-enhanced'

const router = useRouter()
const loading = ref(false)
const rfmLoading = ref(false)
const submitLoading = ref(false)
const customers = ref<CustomerWithTags[]>([])
const rfmCustomers = ref<CustomerWithTags[]>([])
const total = ref(0)
const dialogVisible = ref(false)
const isEdit = ref(false)
const formRef = ref<FormInstance>()
const activeTab = ref('list')
const tags = ref<CustomerTag[]>([])
const rfmDistribution = ref<Record<string, number>>({})

const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  customer_type: '',
  status: '',
  tag_id: undefined as number | undefined,
})

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
  const tags: Record<string, string> = {
    normal: '',
    vip: 'warning',
    wholesale: 'success',
  }
  return tags[type] || ''
}

const getRfmLevelTag = (level: string) => {
  const tags: Record<string, string> = {
    A: 'success',
    B: 'primary',
    C: 'warning',
    D: 'info',
    E: 'danger',
  }
  return tags[level] || ''
}

const fetchCustomerList = async () => {
  loading.value = true
  try {
    const res = await crmEnhancedApi.getCustomerList(queryParams)
    customers.value = res.data?.list || []
    total.value = res.data?.total || 0
  } catch (error: any) {
    ElMessage.error(error.message || '获取客户列表失败')
    customers.value = []
    total.value = 0
  } finally {
    loading.value = false
  }
}

const fetchRfmDistribution = async () => {
  try {
    const res = await crmEnhancedApi.getRfmDistribution()
    rfmDistribution.value = res.data || {}
  } catch (error: any) {
    rfmDistribution.value = {}
  }
}

const fetchTags = async () => {
  try {
    const res = await crmEnhancedApi.getTags()
    tags.value = res.data || []
  } catch (error: any) {
    tags.value = []
  }
}

const handleQuery = () => {
  queryParams.page = 1
  if (activeTab.value === 'list') {
    fetchCustomerList()
  }
}

const handleReset = () => {
  queryParams.keyword = ''
  queryParams.customer_type = ''
  queryParams.status = ''
  queryParams.tag_id = undefined
  handleQuery()
}

const handleTabChange = (tab: string) => {
  if (tab === 'rfm' && !rfmCustomers.value.length) {
    fetchRfmCustomers()
  }
}

const fetchRfmCustomers = async () => {
  rfmLoading.value = true
  try {
    const res = await crmEnhancedApi.getCustomerList({ page: 1, page_size: 100 })
    rfmCustomers.value = res.data?.list || []
    fetchRfmDistribution()
  } catch (error: any) {
    rfmCustomers.value = []
  } finally {
    rfmLoading.value = false
  }
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
    await crmEnhancedApi.deleteCustomer(row.id)
    ElMessage.success('删除成功')
    fetchCustomerList()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '删除失败')
    }
  }
}

const handleSubmit = async () => {
  if (!formRef.value) return

  await formRef.value.validate(async (valid) => {
    if (!valid) return

    submitLoading.value = true
    try {
      if (isEdit.value) {
        await crmEnhancedApi.updateCustomer(formData.id!, formData)
        ElMessage.success('更新成功')
      } else {
        await crmEnhancedApi.createCustomer(formData)
        ElMessage.success('创建成功')
      }
      dialogVisible.value = false
      fetchCustomerList()
    } catch (error: any) {
      ElMessage.error(error.message || '操作失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const viewDetail = (id: number) => {
  router.push(`/crm/detail/${id}`)
}

onMounted(() => {
  fetchCustomerList()
  fetchTags()
})
</script>

<style scoped>
.crm-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 24px;
}
.header-left .page-title {
  font-size: 28px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 12px 0;
}
.header-actions {
  display: flex;
  gap: 12px;
}
.filter-card {
  margin-bottom: 20px;
}
.table-card {
  margin-bottom: 20px;
}
.pagination-wrapper {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}
.table-tag {
  border: none;
  margin-right: 4px;
}
.no-tags {
  color: #909399;
  font-size: 12px;
}
.mb-20 {
  margin-bottom: 20px;
}

.rfm-card {
  text-align: center;
}
.rfm-card-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}
.rfm-card-level {
  font-size: 32px;
  font-weight: 700;
  color: #303133;
}
.rfm-card-count {
  font-size: 14px;
  color: #909399;
}
</style>
