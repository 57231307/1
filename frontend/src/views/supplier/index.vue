<template>
  <div class="supplier-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">供应商管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>基础数据</el-breadcrumb-item>
          <el-breadcrumb-item>供应商管理</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          新建供应商
        </el-button>
        <el-button @click="handlePrint">
          <el-icon><Printer /></el-icon>
          打印
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          导出
        </el-button>
      </div>
    </div>

    <SupplierList
      :suppliers="suppliers"
      :total="total"
      :loading="loading"
      :query-params="queryParams"
      :dialog-mode="dialogMode"
      @search="fetchData"
      @reset="handleReset"
      @update:query-params="(v: any) => Object.assign(queryParams, v)"
      @add="handleAdd"
      @view="handleView"
      @edit="handleEdit"
      @delete="handleDelete"
    />

    <!-- 新增/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="dialogTitle"
      width="800px"
      :close-on-click-modal="false"
      @close="resetForm"
    >
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="120px">
        <el-divider content-position="left">基本信息</el-divider>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="供应商编码" prop="supplier_code">
              <el-input v-model="formData.supplier_code" placeholder="请输入供应商编码" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="供应商名称" prop="supplier_name">
              <el-input v-model="formData.supplier_name" placeholder="请输入供应商名称" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="供应商简称" prop="supplier_short_name">
              <el-input v-model="formData.supplier_short_name" placeholder="请输入简称" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="供应商类型" prop="supplier_type">
              <el-select
                v-model="formData.supplier_type"
                placeholder="请选择类型"
                style="width: 100%"
              >
                <el-option label="生产商" value="manufacturer" />
                <el-option label="经销商" value="distributor" />
                <el-option label="服务商" value="service" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="信用代码" prop="credit_code">
              <el-input v-model="formData.credit_code" placeholder="请输入统一社会信用代码" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="法人代表" prop="legal_representative">
              <el-input v-model="formData.legal_representative" placeholder="请输入法人代表" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-divider content-position="left">联系信息</el-divider>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="联系电话" prop="contact_phone">
              <el-input v-model="formData.contact_phone" placeholder="请输入联系电话" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="邮箱" prop="email">
              <el-input v-model="formData.email" placeholder="请输入邮箱" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="网址" prop="website">
              <el-input v-model="formData.website" placeholder="请输入网址" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="传真" prop="fax">
              <el-input v-model="formData.fax" placeholder="请输入传真" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="注册地址" prop="registered_address">
          <el-input v-model="formData.registered_address" placeholder="请输入注册地址" />
        </el-form-item>
        <el-form-item label="经营地址" prop="business_address">
          <el-input v-model="formData.business_address" placeholder="请输入经营地址" />
        </el-form-item>
        <el-divider content-position="left">财务信息</el-divider>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="纳税人类型" prop="taxpayer_type">
              <el-select
                v-model="formData.taxpayer_type"
                placeholder="请选择类型"
                style="width: 100%"
              >
                <el-option label="一般纳税人" value="general" />
                <el-option label="小规模纳税人" value="small" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="注册资本(万)" prop="registered_capital">
              <el-input-number
                v-model="formData.registered_capital"
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
        <el-divider content-position="left">业务信息</el-divider>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="等级" prop="grade">
              <el-select v-model="formData.grade" placeholder="请选择等级" style="width: 100%">
                <el-option label="A级" value="A" />
                <el-option label="B级" value="B" />
                <el-option label="C级" value="C" />
                <el-option label="D级" value="D" />
              </el-select>
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
        <el-form-item label="主营业务" prop="main_business">
          <el-input v-model="formData.main_business" placeholder="请输入主营业务" />
        </el-form-item>
        <el-form-item label="备注" prop="remarks">
          <el-input v-model="formData.remarks" type="textarea" :rows="3" placeholder="请输入备注" />
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
import SupplierList from './SupplierList.vue'
import { ref, reactive, onMounted, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { Plus, Download, Printer } from '@element-plus/icons-vue'
import { supplierApi, type Supplier } from '@/api/supplier'
import { exportData } from '@/utils/export'
import { printData } from '@/utils/print'

const loading = ref(false)
const submitLoading = ref(false)
const suppliers = ref<Supplier[]>([])
const total = ref(0)
const dialogVisible = ref(false)
const isEdit = ref(false)
const formRef = ref<FormInstance>()
// SupplierList 通过 dialog-mode prop 接收的当前模式（add/edit/view）
const dialogMode = ref<'add' | 'edit' | 'view'>('add')

const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  grade: '',
  status: '',
})

const formData = reactive({
  id: undefined as number | undefined,
  supplier_code: '',
  supplier_name: '',
  supplier_short_name: '',
  supplier_type: '',
  credit_code: '',
  registered_address: '',
  business_address: '',
  legal_representative: '',
  registered_capital: 0,
  contact_phone: '',
  fax: '',
  website: '',
  email: '',
  main_business: '',
  taxpayer_type: '',
  bank_name: '',
  bank_account: '',
  grade: '',
  status: 'active',
  remarks: '',
})

const formRules: FormRules = {
  supplier_code: [{ required: true, message: '请输入供应商编码', trigger: 'blur' }],
  supplier_name: [{ required: true, message: '请输入供应商名称', trigger: 'blur' }],
  contact_phone: [
    { required: true, message: '请输入联系电话', trigger: 'blur' },
    { pattern: /^1[3-9]\d{9}$/, message: '请输入正确的手机号', trigger: 'blur' },
  ],
}

const dialogTitle = computed(() => (isEdit.value ? '编辑供应商' : '新建供应商'))

const fetchData = async () => {
  loading.value = true
  try {
    const res = await supplierApi.list(queryParams)
    suppliers.value = res.data!.list || []
    total.value = res.data?.total || 0
  } catch (error: any) {
    ElMessage.error(error.message || '获取供应商列表失败')
    suppliers.value = []
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
  queryParams.grade = ''
  queryParams.status = ''
  handleQuery()
}

const resetForm = () => {
  formData.id = undefined
  formData.supplier_code = ''
  formData.supplier_name = ''
  formData.supplier_short_name = ''
  formData.supplier_type = ''
  formData.credit_code = ''
  formData.registered_address = ''
  formData.business_address = ''
  formData.legal_representative = ''
  formData.registered_capital = 0
  formData.contact_phone = ''
  formData.fax = ''
  formData.website = ''
  formData.email = ''
  formData.main_business = ''
  formData.taxpayer_type = ''
  formData.bank_name = ''
  formData.bank_account = ''
  formData.grade = ''
  formData.status = 'active'
  formData.remarks = ''
  formRef.value?.clearValidate()
}

const handleCreate = () => {
  resetForm()
  isEdit.value = false
  dialogMode.value = 'add'
  dialogVisible.value = true
}

const handleAdd = () => {
  handleCreate()
}

const handleView = (row: Supplier) => {
  resetForm()
  Object.assign(formData, row)
  isEdit.value = false
  dialogMode.value = 'view'
  dialogVisible.value = true
}

const handleEdit = (row: Supplier) => {
  resetForm()
  Object.assign(formData, row)
  isEdit.value = true
  dialogMode.value = 'edit'
  dialogVisible.value = true
}

const handleDelete = async (row: Supplier) => {
  try {
    await ElMessageBox.confirm(`确定删除供应商 "${row.supplier_name}" 吗？`, '删除确认', {
      type: 'warning',
    })
    await supplierApi.delete(row.id)
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

  await formRef.value.validate(async valid => {
    if (!valid) return

    submitLoading.value = true
    try {
      if (isEdit.value) {
        await supplierApi.update(formData.id!, formData)
        ElMessage.success('更新成功')
      } else {
        await supplierApi.create(formData)
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

const handleExport = () => {
  exportData({
    filename: '供应商列表',
    columns: [
      { key: 'supplier_code', title: '供应商编码' },
      { key: 'supplier_name', title: '供应商名称' },
      { key: 'supplier_short_name', title: '简称' },
      { key: 'contact_phone', title: '联系电话' },
      { key: 'email', title: '邮箱' },
      { key: 'grade', title: '等级' },
      { key: 'supplier_type', title: '类型' },
      { key: 'status', title: '状态', formatter: v => (v === 'active' ? '启用' : '禁用') },
    ],
    data: suppliers.value,
  })
}

const handlePrint = () => {
  printData({
    title: '供应商列表',
    columns: [
      { key: 'supplier_code', title: '供应商编码', width: '100px' },
      { key: 'supplier_name', title: '供应商名称' },
      { key: 'contact_phone', title: '联系电话', width: '120px' },
      { key: 'grade', title: '等级', width: '60px' },
      { key: 'supplier_type', title: '类型', width: '80px' },
      {
        key: 'status',
        title: '状态',
        width: '60px',
        formatter: v => (v === 'active' ? '启用' : '禁用'),
      },
    ],
    data: suppliers.value,
  })
}

onMounted(() => {
  fetchData()
})
</script>

<style scoped>
.supplier-page {
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
