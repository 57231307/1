<!--
  AssetListTab.vue - 固定资产 Tab
  来源：原 fixed-assets/index.vue 主体内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="asset-list-tab">
    <div class="page-header">
      <h2 class="page-title">固定资产</h2>
      <div>
        <el-button type="primary" @click="openDialog()">
          <el-icon><Plus /></el-icon>新建资产
        </el-button>
        <el-button @click="handleDepreciateAll">
          <el-icon><Refresh /></el-icon>计提折旧
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>导出
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryForm">
        <el-form-item label="资产编码">
          <el-input v-model="queryForm.asset_code" placeholder="编码" clearable />
        </el-form-item>
        <el-form-item label="资产名称">
          <el-input v-model="queryForm.asset_name" placeholder="名称" clearable />
        </el-form-item>
        <el-form-item label="类别">
          <el-select v-model="queryForm.category" placeholder="选择类别" clearable>
            <el-option label="房屋建筑" value="building" />
            <el-option label="机器设备" value="equipment" />
            <el-option label="运输工具" value="vehicle" />
            <el-option label="电子设备" value="electronic" />
            <el-option label="办公家具" value="furniture" />
          </el-select>
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="queryForm.status" placeholder="选择状态" clearable>
            <el-option label="在用" value="in_use" />
            <el-option label="闲置" value="idle" />
            <el-option label="已处置" value="disposed" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSearch">查询</el-button>
          <el-button @click="handleReset">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover">
      <el-table v-loading="loading" :data="assetList" stripe>
        <el-table-column prop="asset_code" label="资产编码" width="120" />
        <el-table-column prop="asset_name" label="资产名称" min-width="150" />
        <el-table-column prop="category" label="类别" width="100">
          <template #default="{ row }">{{ getCategoryLabel(row.category) }}</template>
        </el-table-column>
        <el-table-column prop="department_name" label="使用部门" width="120" />
        <el-table-column prop="purchase_date" label="购置日期" width="120" />
        <el-table-column label="原值" width="120" align="right">
          <template #default="{ row }">¥{{ row.purchase_amount.toFixed(2) }}</template>
        </el-table-column>
        <el-table-column label="累计折旧" width="120" align="right">
          <template #default="{ row }">¥{{ row.accumulated_depreciation.toFixed(2) }}</template>
        </el-table-column>
        <el-table-column label="净值" width="120" align="right">
          <template #default="{ row }">¥{{ row.net_value.toFixed(2) }}</template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="openDialog(row)">编辑</el-button>
            <el-button type="success" link size="small" @click="handleDepreciate(row)"
              >折旧</el-button
            >
            <el-button type="danger" link size="small" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="queryForm.page"
          v-model:page-size="queryForm.page_size"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="handleSearch"
          @current-change="handleSearch"
        />
      </div>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="form.id ? '编辑资产' : '新建资产'" width="600px">
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="资产编码" prop="asset_code">
              <el-input v-model="form.asset_code" :disabled="!!form.id" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="资产名称" prop="asset_name">
              <el-input v-model="form.asset_name" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="类别" prop="category">
              <el-select v-model="form.category" placeholder="选择类别" style="width: 100%">
                <el-option label="房屋建筑" value="building" />
                <el-option label="机器设备" value="equipment" />
                <el-option label="运输工具" value="vehicle" />
                <el-option label="电子设备" value="electronic" />
                <el-option label="办公家具" value="furniture" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="购置日期" prop="purchase_date">
              <el-date-picker
                v-model="form.purchase_date"
                type="date"
                placeholder="选择日期"
                value-format="YYYY-MM-DD"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="原值" prop="purchase_amount">
              <el-input-number
                v-model="form.purchase_amount"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="残值" prop="salvage_value">
              <el-input-number
                v-model="form.salvage_value"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="使用年限(月)" prop="useful_life_months">
              <el-input-number v-model="form.useful_life_months" :min="1" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="折旧方法" prop="depreciation_method">
              <el-select v-model="form.depreciation_method" style="width: 100%">
                <el-option label="直线法" value="straight_line" />
                <el-option label="工作量法" value="workload" />
                <el-option label="双倍余额递减" value="double_declining" />
                <el-option label="年数总和法" value="sum_of_years" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="位置">
              <el-input v-model="form.location" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="保管人">
              <el-input v-model="form.custodian" />
            </el-form-item>
          </el-col>
        </el-row>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Refresh, Download } from '@element-plus/icons-vue'
import {
  listAssets,
  createAsset,
  updateAsset,
  deleteAsset as deleteAssetApi,
  depreciateAsset,
  type FixedAsset,
  type FixedAssetCreateRequest,
  type FixedAssetUpdateRequest,
} from '@/api/asset'
import { logger } from '@/utils/logger'

const loading = ref(false)
const submitLoading = ref(false)
const dialogVisible = ref(false)
const assetList = ref<FixedAsset[]>([])
const total = ref(0)
const formRef = ref<FormInstance>()

const queryForm = reactive({
  asset_code: '',
  asset_name: '',
  category: '',
  status: '',
  page: 1,
  page_size: 20,
})

const form = reactive<FixedAssetCreateRequest & { id?: number }>({
  id: undefined,
  asset_code: '',
  asset_name: '',
  category: 'equipment',
  purchase_date: new Date().toISOString().split('T')[0],
  purchase_amount: 0,
  salvage_value: 0,
  useful_life_months: 60,
  depreciation_method: 'straight_line',
  location: '',
  custodian: '',
})

const rules: FormRules = {
  asset_code: [{ required: true, message: '请输入资产编码', trigger: 'blur' }],
  asset_name: [{ required: true, message: '请输入资产名称', trigger: 'blur' }],
  category: [{ required: true, message: '请选择类别', trigger: 'change' }],
  purchase_date: [{ required: true, message: '请选择购置日期', trigger: 'change' }],
  purchase_amount: [{ required: true, message: '请输入原值', trigger: 'blur' }],
  useful_life_months: [{ required: true, message: '请输入使用年限', trigger: 'blur' }],
  depreciation_method: [{ required: true, message: '请选择折旧方法', trigger: 'change' }],
}

const getCategoryLabel = (category: string) => {
  const map: Record<string, string> = {
    building: '房屋建筑',
    equipment: '机器设备',
    vehicle: '运输工具',
    electronic: '电子设备',
    furniture: '办公家具',
  }
  return map[category] || category
}

const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    in_use: '在用',
    idle: '闲置',
    disposed: '已处置',
  }
  return map[status] || status
}

const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    in_use: 'success',
    idle: 'warning',
    disposed: 'info',
  }
  return map[status] || 'info'
}

const fetchAssets = async () => {
  loading.value = true
  try {
    const res = await listAssets(queryForm)
    const d = res.data as
      | { list?: FixedAsset[]; items?: FixedAsset[]; data?: FixedAsset[]; total?: number }
      | FixedAsset[]
    if (Array.isArray(d)) {
      assetList.value = d
      total.value = d.length
    } else {
      assetList.value = d?.list || d?.items || []
      total.value = d?.total || assetList.value.length
    }
  } catch (e) {
    const err = e as Error
    ElMessage.error(err.message || '获取资产列表失败')
  } finally {
    loading.value = false
  }
}

const handleSearch = () => {
  queryForm.page = 1
  fetchAssets()
}

const handleReset = () => {
  queryForm.asset_code = ''
  queryForm.asset_name = ''
  queryForm.category = ''
  queryForm.status = ''
  handleSearch()
}

const openDialog = (row?: FixedAsset) => {
  formRef.value?.resetFields()
  if (row) {
    form.id = row.id
    form.asset_code = row.asset_code
    form.asset_name = row.asset_name
    form.category = row.category
    form.purchase_date = row.purchase_date
    form.purchase_amount = row.purchase_amount
    form.salvage_value = row.salvage_value
    form.useful_life_months = row.useful_life_months
    form.depreciation_method = row.depreciation_method
    form.location = row.location
    form.custodian = row.custodian
  } else {
    form.id = undefined
    form.asset_code = ''
    form.asset_name = ''
    form.category = 'equipment'
    form.purchase_date = new Date().toISOString().split('T')[0]
    form.purchase_amount = 0
    form.salvage_value = 0
    form.useful_life_months = 60
    form.depreciation_method = 'straight_line'
    form.location = ''
    form.custodian = ''
  }
  dialogVisible.value = true
}

const handleSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async valid => {
    if (!valid) return
    submitLoading.value = true
    try {
      if (form.id) {
        const updateData: FixedAssetUpdateRequest = {
          asset_name: form.asset_name,
          location: form.location,
          custodian: form.custodian,
        }
        await updateAsset(form.id, updateData)
        ElMessage.success('更新成功')
      } else {
        await createAsset(form)
        ElMessage.success('创建成功')
      }
      dialogVisible.value = false
      fetchAssets()
    } catch (e) {
      const err = e as Error
      ElMessage.error(err.message || '操作失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const handleDelete = async (row: FixedAsset) => {
  try {
    await ElMessageBox.confirm(`确定删除资产 "${row.asset_name}" 吗？`, '删除确认', {
      type: 'warning',
    })
    await deleteAssetApi(row.id)
    ElMessage.success('删除成功')
    fetchAssets()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      ElMessage.error(err.message || '删除失败')
    }
  }
}

const handleDepreciate = async (row: FixedAsset) => {
  try {
    await ElMessageBox.confirm('确定对该资产计提折旧吗？', '计提折旧', { type: 'info' })
    await depreciateAsset(row.id)
    ElMessage.success('折旧成功')
    fetchAssets()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      ElMessage.error(err.message || '折旧失败')
    }
  }
}

const handleDepreciateAll = () => {
  ElMessage.info('请逐个对资产计提折旧')
}

const handleExport = () => {
  const csvContent = [
    ['资产编码', '资产名称', '类别', '原值', '累计折旧', '净值', '状态'],
    ...assetList.value.map(a => [
      a.asset_code,
      a.asset_name,
      getCategoryLabel(a.category),
      a.purchase_amount.toFixed(2),
      a.accumulated_depreciation.toFixed(2),
      a.net_value.toFixed(2),
      getStatusLabel(a.status),
    ]),
  ]
    .map(row => row.map(cell => `"${cell}"`).join(','))
    .join('\n')
  const blob = new Blob(['\uFEFF' + csvContent], { type: 'text/csv;charset=utf-8;' })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(blob)
  link.download = `固定资产_${new Date().toISOString().split('T')[0]}.csv`
  link.click()
  ElMessage.success('导出成功')
  logger.info('固定资产列表已导出')
}

onMounted(() => {
  fetchAssets()
})
</script>
