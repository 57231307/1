<template>
  <div class="fixed-assets-page">
    <div class="header">
      <h2>固定资产管理</h2>
      <el-button type="primary" @click="handleCreate">新增资产</el-button>
    </div>

    <el-table v-loading="loading" :data="assetList" border>
      <el-table-column prop="assetCode" label="资产编码" />
      <el-table-column prop="assetName" label="资产名称" />
      <el-table-column prop="category" label="资产类别" />
      <el-table-column prop="originalValue" label="原值" />
      <el-table-column prop="netValue" label="净值" />
      <el-table-column prop="purchaseDate" label="购置日期" />
      <el-table-column prop="status" label="状态">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">
            {{ getStatusLabel(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="200">
        <template #default="{ row }">
          <el-button size="small" @click="handleEdit(row)">编辑</el-button>
          <el-button size="small" type="danger" @click="handleDelete(row)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- 新建/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="dialogMode === 'create' ? '新增固定资产' : '编辑固定资产'"
      width="700px"
      @close="handleDialogClose"
    >
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="120px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="资产编码" prop="assetCode">
              <el-input v-model="formData.assetCode" placeholder="请输入资产编码" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="资产名称" prop="assetName">
              <el-input v-model="formData.assetName" placeholder="请输入资产名称" />
            </el-form-item>
          </el-col>
        </el-row>

        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="资产类别" prop="category">
              <el-select
                v-model="formData.category"
                placeholder="请选择资产类别"
                style="width: 100%"
              >
                <el-option label="房屋建筑物" value="building" />
                <el-option label="机器设备" value="machine" />
                <el-option label="运输工具" value="vehicle" />
                <el-option label="电子设备" value="electronic" />
                <el-option label="办公家具" value="furniture" />
                <el-option label="其他" value="other" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="规格型号" prop="specification">
              <el-input v-model="formData.specification" placeholder="请输入规格型号" />
            </el-form-item>
          </el-col>
        </el-row>

        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="原值 (元)" prop="originalValue">
              <el-input-number
                v-model="formData.originalValue"
                :min="0"
                :precision="2"
                :step="100"
                style="width: 100%"
                placeholder="请输入原值"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="预计残值 (元)" prop="salvageValue">
              <el-input-number
                v-model="formData.salvageValue"
                :min="0"
                :precision="2"
                style="width: 100%"
                placeholder="请输入预计残值"
              />
            </el-form-item>
          </el-col>
        </el-row>

        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="购置日期" prop="purchaseDate">
              <el-date-picker
                v-model="formData.purchaseDate"
                type="date"
                placeholder="请选择购置日期"
                style="width: 100%"
                format="YYYY-MM-DD"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="使用部门" prop="departmentId">
              <el-select
                v-model="formData.departmentId"
                placeholder="请选择使用部门"
                style="width: 100%"
                clearable
              >
                <el-option
                  v-for="dept in departmentList"
                  :key="dept.id"
                  :label="dept.name"
                  :value="dept.id"
                />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>

        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="存放地点" prop="location">
              <el-input v-model="formData.location" placeholder="请输入存放地点" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="责任人" prop="responsiblePerson">
              <el-input v-model="formData.responsiblePerson" placeholder="请输入责任人" />
            </el-form-item>
          </el-col>
        </el-row>

        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="折旧年限 (年)" prop="depreciationYears">
              <el-input-number
                v-model="formData.depreciationYears"
                :min="1"
                :max="50"
                style="width: 100%"
                placeholder="请输入折旧年限"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="状态" prop="status">
              <el-select v-model="formData.status" placeholder="请选择状态" style="width: 100%">
                <el-option label="使用中" value="in_use" />
                <el-option label="闲置" value="idle" />
                <el-option label="已报废" value="scrapped" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>

        <el-row :gutter="20">
          <el-col :span="24">
            <el-form-item label="备注" prop="remarks">
              <el-input
                v-model="formData.remarks"
                type="textarea"
                :rows="3"
                placeholder="请输入备注"
              />
            </el-form-item>
          </el-col>
        </el-row>
      </el-form>

      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit"> 确定 </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { listAssets, createAsset, updateAsset, deleteAsset } from '@/api/asset'
import { listDepartments } from '@/api/department'

const loading = ref(false)
const submitLoading = ref(false)
const dialogVisible = ref(false)
const dialogMode = ref<'create' | 'edit'>('create')
const formRef = ref<FormInstance>()
const assetList = ref<any[]>([])
const departmentList = ref<any[]>([])

const formData = reactive<any>({
  assetCode: '',
  assetName: '',
  category: '',
  specification: '',
  originalValue: 0,
  salvageValue: 0,
  purchaseDate: '',
  departmentId: null,
  location: '',
  responsiblePerson: '',
  depreciationYears: 5,
  status: 'in_use',
  remarks: '',
})

const formRules: FormRules = {
  assetCode: [{ required: true, message: '请输入资产编码', trigger: 'blur' }],
  assetName: [{ required: true, message: '请输入资产名称', trigger: 'blur' }],
  category: [{ required: true, message: '请选择资产类别', trigger: 'change' }],
  originalValue: [{ required: true, message: '请输入原值', trigger: 'blur' }],
  purchaseDate: [{ required: true, message: '请选择购置日期', trigger: 'change' }],
  status: [{ required: true, message: '请选择状态', trigger: 'change' }],
}

const getStatusType = (status: string) => {
  const types: Record<string, any> = {
    in_use: 'success',
    idle: 'warning',
    scrapped: 'danger',
  }
  return types[status] || 'info'
}

const getStatusLabel = (status: string) => {
  const labels: Record<string, string> = {
    in_use: '使用中',
    idle: '闲置',
    scrapped: '已报废',
  }
  return labels[status] || status
}

const loadAssets = async () => {
  loading.value = true
  try {
    const res = await listAssets()
    assetList.value = res.data! || []
  } catch (error) {
    ElMessage.error('加载资产列表失败')
  } finally {
    loading.value = false
  }
}

const loadDepartments = async () => {
  try {
    const res = await listDepartments()
    departmentList.value = res.data! || []
  } catch (error) {
    // 部门加载失败不影响主功能
  }
}

const handleCreate = () => {
  dialogMode.value = 'create'
  Object.assign(formData, {
    id: null,
    assetCode: '',
    assetName: '',
    category: '',
    specification: '',
    originalValue: 0,
    salvageValue: 0,
    purchaseDate: '',
    departmentId: null,
    location: '',
    responsiblePerson: '',
    depreciationYears: 5,
    status: 'in_use',
    remarks: '',
  })
  dialogVisible.value = true
}

const handleEdit = (row: any) => {
  dialogMode.value = 'edit'
  Object.assign(formData, {
    id: row.id,
    assetCode: row.assetCode,
    assetName: row.assetName,
    category: row.category,
    specification: row.specification,
    originalValue: row.originalValue,
    salvageValue: row.salvageValue,
    purchaseDate: row.purchaseDate,
    departmentId: row.departmentId,
    location: row.location,
    responsiblePerson: row.responsiblePerson,
    depreciationYears: row.depreciationYears,
    status: row.status,
    remarks: row.remarks,
  })
  dialogVisible.value = true
}

const handleDelete = async (row: any) => {
  if (!row.id) return

  try {
    await deleteAsset(row.id)
    ElMessage.success('删除成功')
    await loadAssets()
  } catch (error) {
    ElMessage.error('删除失败')
  }
}

const handleSubmit = async () => {
  if (!formRef.value) return

  await formRef.value.validate(async (valid: boolean) => {
    if (!valid) return

    submitLoading.value = true
    try {
      if (dialogMode.value === 'create') {
        await createAsset(formData)
        ElMessage.success('创建成功')
      } else {
        await updateAsset(formData.id, formData)
        ElMessage.success('更新成功')
      }
      dialogVisible.value = false
      await loadAssets()
    } catch (error) {
      ElMessage.error(dialogMode.value === 'create' ? '创建失败' : '更新失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const handleDialogClose = () => {
  formRef.value?.resetFields()
}

onMounted(() => {
  loadAssets()
  loadDepartments()
})
</script>

<style scoped>
.fixed-assets-page {
  padding: 20px;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
</style>
