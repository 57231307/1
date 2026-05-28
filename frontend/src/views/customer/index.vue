<template>
  <div class="customer-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">客户管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>基础数据</el-breadcrumb-item>
          <el-breadcrumb-item>客户管理</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          新建客户
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
      <el-table v-loading="loading" :data="customers" stripe>
        <el-table-column prop="customer_code" label="客户编码" width="120" fixed />
        <el-table-column prop="customer_name" label="客户名称" min-width="180" fixed />
        <el-table-column prop="contact_person" label="联系人" width="100" />
        <el-table-column prop="contact_phone" label="电话" width="130" />
        <el-table-column prop="contact_email" label="邮箱" width="180" show-overflow-tooltip />
        <el-table-column prop="customer_type" label="类型" width="100">
          <template #default="{ row }">
            <el-tag :type="getCustomerTypeTag(row.customer_type)" size="small">
              {{ getCustomerTypeLabel(row.customer_type) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="province" label="省份" width="100" />
        <el-table-column prop="credit_limit" label="信用额度" width="120" align="right">
          <template #default="{ row }">
            {{ row.credit_limit ? formatCurrency(row.credit_limit) : '-' }}
          </template>
        </el-table-column>
        <el-table-column prop="payment_terms" label="账期(天)" width="90" align="center" />
        <el-table-column prop="status" label="状态" width="80">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleEdit(row)">编辑</el-button>
            <el-button type="danger" link size="small" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>

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
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="120px">
        <el-divider content-position="left">基本信息</el-divider>
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
            <el-form-item label="联系电话" prop="contact_phone">
              <el-input v-model="formData.contact_phone" placeholder="请输入联系电话" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="邮箱" prop="contact_email">
              <el-input v-model="formData.contact_email" placeholder="请输入邮箱" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="客户类型" prop="customer_type">
              <el-select
                v-model="formData.customer_type"
                placeholder="请选择类型"
                style="width: 100%"
              >
                <el-option label="零售" value="retail" />
                <el-option label="批发" value="wholesale" />
                <el-option label="VIP" value="vip" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="行业" prop="customer_industry">
              <el-input v-model="formData.customer_industry" placeholder="请输入行业" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="年采购额" prop="annual_purchase">
              <el-input-number
                v-model="formData.annual_purchase"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-divider content-position="left">地址信息</el-divider>
        <el-form-item label="地址" prop="address">
          <el-input v-model="formData.address" placeholder="请输入详细地址" />
        </el-form-item>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="省份" prop="province">
              <el-input v-model="formData.province" placeholder="请输入省份" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="城市" prop="city">
              <el-input v-model="formData.city" placeholder="请输入城市" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="邮编" prop="postal_code">
              <el-input v-model="formData.postal_code" placeholder="请输入邮编" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="国家" prop="country">
              <el-input v-model="formData.country" placeholder="请输入国家" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-divider content-position="left">财务信息</el-divider>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="税号" prop="tax_id">
              <el-input v-model="formData.tax_id" placeholder="请输入税号" />
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
            <el-form-item label="账期(天)" prop="payment_terms">
              <el-input-number v-model="formData.payment_terms" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="状态" prop="status">
              <el-radio-group v-model="formData.status">
                <el-radio value="active">启用</el-radio>
                <el-radio value="inactive">停用</el-radio>
              </el-radio-group>
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
        <el-divider content-position="left">业务信息</el-divider>
        <el-form-item label="主营产品" prop="main_products">
          <el-input v-model="formData.main_products" placeholder="请输入主营产品" />
        </el-form-item>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="质量要求" prop="quality_requirement">
              <el-input v-model="formData.quality_requirement" placeholder="请输入质量要求" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="验货标准" prop="inspection_standard">
              <el-input v-model="formData.inspection_standard" placeholder="请输入验货标准" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="备注" prop="notes">
          <el-input v-model="formData.notes" type="textarea" :rows="3" placeholder="请输入备注" />
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
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import { customerApi, type Customer } from '@/api/customer'

const loading = ref(false)
const submitLoading = ref(false)
const customers = ref<Customer[]>([])
const total = ref(0)
const dialogVisible = ref(false)
const isEdit = ref(false)
const formRef = ref<FormInstance>()

const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  customer_type: '',
  status: '',
})

const formData = reactive({
  id: undefined as number | undefined,
  customer_code: '',
  customer_name: '',
  contact_person: '',
  contact_phone: '',
  contact_email: '',
  address: '',
  city: '',
  province: '',
  country: '',
  postal_code: '',
  customer_type: 'retail',
  tax_id: '',
  credit_limit: 0,
  payment_terms: 30,
  bank_name: '',
  bank_account: '',
  status: 'active',
  notes: '',
  customer_industry: '',
  main_products: '',
  annual_purchase: 0,
  quality_requirement: '',
  inspection_standard: '',
})

const formRules: FormRules = {
  customer_code: [{ required: true, message: '请输入客户编码', trigger: 'blur' }],
  customer_name: [{ required: true, message: '请输入客户名称', trigger: 'blur' }],
  contact_person: [{ required: true, message: '请输入联系人', trigger: 'blur' }],
  contact_phone: [
    { required: true, message: '请输入电话', trigger: 'blur' },
    { pattern: /^1[3-9]\d{9}$/, message: '请输入正确的手机号', trigger: 'blur' },
  ],
}

const dialogTitle = computed(() => (isEdit.value ? '编辑客户' : '新建客户'))

const formatCurrency = (amount: number) => `¥${(amount || 0).toFixed(2)}`

const getCustomerTypeLabel = (type: string) => {
  const labels: Record<string, string> = {
    retail: '零售',
    vip: 'VIP',
    wholesale: '批发',
  }
  return labels[type] || type
}

const getCustomerTypeTag = (type: string) => {
  const tags: Record<string, string> = {
    retail: '',
    vip: 'warning',
    wholesale: 'success',
  }
  return tags[type] || ''
}

const fetchData = async () => {
  loading.value = true
  try {
    const res = await customerApi.list(queryParams)
    customers.value = res.data!.list || []
    total.value = res.data?.total || 0
  } catch (error: any) {
    ElMessage.error(error.message || '获取客户列表失败')
    customers.value = []
    total.value = 0
  } finally {
    loading.value = false
  }
}

const handleQuery = () => {
  queryParams.page = 1
  fetchData()
}

const handleReset = () => {
  queryParams.keyword = ''
  queryParams.customer_type = ''
  queryParams.status = ''
  handleQuery()
}

const resetForm = () => {
  formData.id = undefined
  formData.customer_code = ''
  formData.customer_name = ''
  formData.contact_person = ''
  formData.contact_phone = ''
  formData.contact_email = ''
  formData.address = ''
  formData.city = ''
  formData.province = ''
  formData.country = ''
  formData.postal_code = ''
  formData.customer_type = 'retail'
  formData.tax_id = ''
  formData.credit_limit = 0
  formData.payment_terms = 30
  formData.bank_name = ''
  formData.bank_account = ''
  formData.status = 'active'
  formData.notes = ''
  formData.customer_industry = ''
  formData.main_products = ''
  formData.annual_purchase = 0
  formData.quality_requirement = ''
  formData.inspection_standard = ''
  formRef.value?.clearValidate()
}

const handleCreate = () => {
  resetForm()
  isEdit.value = false
  dialogVisible.value = true
}

const handleEdit = (row: Customer) => {
  resetForm()
  Object.assign(formData, row)
  isEdit.value = true
  dialogVisible.value = true
}

const handleDelete = async (row: Customer) => {
  try {
    await ElMessageBox.confirm(`确定删除客户 "${row.customer_name}" 吗？`, '删除确认', {
      type: 'warning',
    })
    await customerApi.delete(row.id)
    ElMessage.success('删除成功')
    fetchData()
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
        await customerApi.update(formData.id!, formData)
        ElMessage.success('更新成功')
      } else {
        await customerApi.create(formData)
        ElMessage.success('创建成功')
      }
      dialogVisible.value = false
      fetchData()
    } catch (error: any) {
      ElMessage.error(error.message || '操作失败')
    } finally {
      submitLoading.value = false
    }
  })
}

onMounted(() => {
  fetchData()
})
</script>

<style scoped>
.customer-page {
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
</style>
