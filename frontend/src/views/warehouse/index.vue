<template>
  <div class="warehouse-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">仓库管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>基础数据</el-breadcrumb-item>
          <el-breadcrumb-item>仓库管理</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          新建仓库
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

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form">
        <el-form-item label="关键词">
          <el-input v-model="queryParams.keyword" placeholder="仓库编码/名称" clearable />
        </el-form-item>
        <el-form-item label="仓库类型">
          <el-select v-model="queryParams.warehouse_type" placeholder="选择类型" clearable>
            <el-option label="原料仓" value="raw" />
            <el-option label="成品仓" value="finished" />
            <el-option label="半成品仓" value="semi" />
            <el-option label="退货仓" value="return" />
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
      <el-table v-loading="loading" :data="warehouses" stripe>
        <el-table-column prop="warehouse_code" label="仓库编码" width="120" fixed />
        <el-table-column prop="warehouse_name" label="仓库名称" min-width="180" fixed />
        <el-table-column prop="warehouse_type" label="类型" width="100">
          <template #default="{ row }">
            <el-tag :type="getWarehouseTypeTag(row.warehouse_type)" size="small">
              {{ getWarehouseTypeLabel(row.warehouse_type) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="address" label="地址" min-width="200" show-overflow-tooltip />
        <el-table-column prop="contact_person" label="负责人" width="100" />
        <el-table-column prop="phone" label="电话" width="130" />
        <el-table-column prop="capacity" label="容量" width="100" align="right">
          <template #default="{ row }">
            {{ row.capacity ? `${row.capacity} m³` : '-' }}
          </template>
        </el-table-column>
        <el-table-column prop="is_default" label="默认" width="80">
          <template #default="{ row }">
            <el-tag v-if="row.is_default" type="success" size="small">是</el-tag>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="80">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleEdit(row as any)"
              >编辑</el-button
            >
            <el-button type="danger" link size="small" @click="handleDelete(row as any)"
              >删除</el-button
            >
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
      width="600px"
      :close-on-click-modal="false"
      @close="resetForm"
    >
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="仓库编码" prop="warehouse_code">
              <el-input v-model="formData.warehouse_code" placeholder="请输入仓库编码" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="仓库名称" prop="warehouse_name">
              <el-input v-model="formData.warehouse_name" placeholder="请输入仓库名称" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="仓库类型" prop="warehouse_type">
              <el-select
                v-model="formData.warehouse_type"
                placeholder="请选择类型"
                style="width: 100%"
              >
                <el-option label="原料仓" value="raw" />
                <el-option label="成品仓" value="finished" />
                <el-option label="半成品仓" value="semi" />
                <el-option label="退货仓" value="return" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="容量(m³)" prop="capacity">
              <el-input-number v-model="formData.capacity" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="地址" prop="address">
          <el-input v-model="formData.address" placeholder="请输入地址" />
        </el-form-item>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="负责人" prop="contact_person">
              <el-input v-model="formData.contact_person" placeholder="请输入负责人" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="电话" prop="phone">
              <el-input v-model="formData.phone" placeholder="请输入电话" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="描述" prop="description">
          <el-input
            v-model="formData.description"
            type="textarea"
            :rows="3"
            placeholder="请输入描述"
          />
        </el-form-item>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="设为默认" prop="is_default">
              <el-switch v-model="formData.is_default" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="状态" prop="status">
              <el-radio-group v-model="formData.status">
                <el-radio value="active">启用</el-radio>
                <el-radio value="inactive">禁用</el-radio>
              </el-radio-group>
            </el-form-item>
          </el-col>
        </el-row>
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
import { Plus, Download, Printer } from '@element-plus/icons-vue'
import { warehouseApi, type Warehouse } from '@/api/warehouse'
import { exportData } from '@/utils/export'
import { printData } from '@/utils/print'

const loading = ref(false)
const submitLoading = ref(false)
const warehouses = ref<Warehouse[]>([])
const total = ref(0)
const dialogVisible = ref(false)
const isEdit = ref(false)
const formRef = ref<FormInstance>()

const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  warehouse_type: '',
  status: '',
})

const formData = reactive({
  id: undefined as number | undefined,
  warehouse_code: '',
  warehouse_name: '',
  warehouse_type: 'finished',
  address: '',
  contact_person: '',
  phone: '',
  capacity: undefined as number | undefined,
  description: '',
  is_default: false,
  status: 'active',
})

const formRules: FormRules = {
  warehouse_code: [{ required: true, message: '请输入仓库编码', trigger: 'blur' }],
  warehouse_name: [{ required: true, message: '请输入仓库名称', trigger: 'blur' }],
  warehouse_type: [{ required: true, message: '请选择仓库类型', trigger: 'change' }],
}

const dialogTitle = computed(() => (isEdit.value ? '编辑仓库' : '新建仓库'))

const getWarehouseTypeLabel = (type: string) => {
  const labels: Record<string, string> = {
    raw: '原料仓',
    finished: '成品仓',
    semi: '半成品仓',
    return: '退货仓',
  }
  return labels[type] || type
}

const getWarehouseTypeTag = (type: string) => {
  const tags: Record<string, string> = {
    raw: 'warning',
    finished: 'success',
    semi: 'info',
    return: 'danger',
  }
  return tags[type] || ''
}

const fetchData = async () => {
  loading.value = true
  try {
    const res = await warehouseApi.list(queryParams)
    warehouses.value = res.data!.list || []
    total.value = res.data?.total || 0
  } catch (error: any) {
    ElMessage.error(error.message || '获取仓库列表失败')
    warehouses.value = []
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
  queryParams.warehouse_type = ''
  queryParams.status = ''
  handleQuery()
}

const resetForm = () => {
  formData.id = undefined
  formData.warehouse_code = ''
  formData.warehouse_name = ''
  formData.warehouse_type = 'finished'
  formData.address = ''
  formData.contact_person = ''
  formData.phone = ''
  formData.capacity = undefined
  formData.description = ''
  formData.is_default = false
  formData.status = 'active'
  formRef.value?.clearValidate()
}

const handleCreate = () => {
  resetForm()
  isEdit.value = false
  dialogVisible.value = true
}

const handleEdit = (row: Warehouse) => {
  resetForm()
  Object.assign(formData, row)
  isEdit.value = true
  dialogVisible.value = true
}

const handleDelete = async (row: Warehouse) => {
  try {
    await ElMessageBox.confirm(`确定删除仓库 "${row.warehouse_name}" 吗？`, '删除确认', {
      type: 'warning',
    })
    await warehouseApi.delete(row.id)
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
        await warehouseApi.update(formData.id!, formData)
        ElMessage.success('更新成功')
      } else {
        await warehouseApi.create(formData)
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
    filename: '仓库列表',
    columns: [
      { key: 'warehouse_code', title: '仓库编码' },
      { key: 'warehouse_name', title: '仓库名称' },
      { key: 'warehouse_type', title: '类型', formatter: v => getWarehouseTypeLabel(String(v)) },
      { key: 'address', title: '地址' },
      { key: 'contact_person', title: '负责人' },
      { key: 'phone', title: '电话' },
      { key: 'capacity', title: '容量(m³)' },
      { key: 'status', title: '状态', formatter: v => (v === 'active' ? '启用' : '禁用') },
    ],
    data: warehouses.value,
  })
}

const handlePrint = () => {
  printData({
    title: '仓库列表',
    columns: [
      { key: 'warehouse_code', title: '仓库编码', width: '100px' },
      { key: 'warehouse_name', title: '仓库名称' },
      {
        key: 'warehouse_type',
        title: '类型',
        width: '80px',
        formatter: v => getWarehouseTypeLabel(String(v)),
      },
      { key: 'contact_person', title: '负责人', width: '80px' },
      { key: 'phone', title: '电话', width: '120px' },
      {
        key: 'status',
        title: '状态',
        width: '60px',
        formatter: v => (v === 'active' ? '启用' : '禁用'),
      },
    ],
    data: warehouses.value,
  })
}

onMounted(() => {
  fetchData()
})
</script>

<style scoped>
.warehouse-page {
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
