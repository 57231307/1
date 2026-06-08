<template>
  <div class="bom-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">BOM 管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>生产管理</el-breadcrumb-item>
          <el-breadcrumb-item>BOM 管理</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          新建 BOM
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form">
        <el-form-item label="产品名称">
          <el-input v-model="queryParams.product_name" placeholder="请输入产品名称" clearable />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="queryParams.status" placeholder="选择状态" clearable>
            <el-option label="草稿" value="draft" />
            <el-option label="启用" value="active" />
            <el-option label="归档" value="archived" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">查询</el-button>
          <el-button @click="handleReset">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table v-loading="loading" :data="boms" stripe>
        <el-table-column prop="product_code" label="产品编码" width="140" fixed />
        <el-table-column prop="product_name" label="产品名称" min-width="180" fixed />
        <el-table-column prop="version" label="版本号" width="100" />
        <el-table-column prop="is_default" label="默认" width="80">
          <template #default="{ row }">
            <el-tag v-if="row.is_default" type="success" size="small">默认</el-tag>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)" size="small">
              {{ getStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="remark" label="备注" min-width="150" show-overflow-tooltip />
        <el-table-column prop="updated_at" label="更新时间" width="180" />
        <el-table-column label="操作" width="280" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleEdit(row as any)">编辑</el-button>
            <el-button type="primary" link size="small" @click="handleCopy(row as any)">复制</el-button>
            <el-button
              v-if="!row.is_default"
              type="success"
              link
              size="small"
              @click="handleSetDefault(row as any)"
            >
              设为默认
            </el-button>
            <el-button type="danger" link size="small" @click="handleDelete(row as any)">删除</el-button>
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
      width="900px"
      :close-on-click-modal="false"
      @close="resetForm"
    >
      <BomForm
        ref="bomFormRef"
        :form-data="formData"
        :mode="dialogMode"
        @submit="handleSubmit"
        @cancel="dialogVisible = false"
      />
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import { bomApi, type Bom } from '@/api/bom'
import BomForm from './BomForm.vue'

const loading = ref(false)
const boms = ref<Bom[]>([])
const total = ref(0)
const dialogVisible = ref(false)
const dialogMode = ref<'create' | 'edit'>('create')
const bomFormRef = ref<InstanceType<typeof BomForm>>()

const queryParams = reactive({
  page: 1,
  page_size: 20,
  product_name: '',
  status: '',
})

const formData = reactive({
  id: undefined as number | undefined,
  product_id: undefined as number | undefined,
  product_name: '',
  version: '',
  is_default: false,
  status: 'draft' as 'draft' | 'active' | 'archived',
  remark: '',
  items: [] as Array<{
    material_name: string
    quantity: number
    unit: string
    loss_rate: number
  }>,
})

const dialogTitle = computed(() => {
  return dialogMode.value === 'create' ? '新建 BOM' : '编辑 BOM'
})

const getStatusType = (status: string) => {
  const types: Record<string, string> = {
    draft: 'info',
    active: 'success',
    archived: 'warning',
  }
  return types[status] || 'info'
}

const getStatusLabel = (status: string) => {
  const labels: Record<string, string> = {
    draft: '草稿',
    active: '启用',
    archived: '归档',
  }
  return labels[status] || status
}

const fetchData = async () => {
  loading.value = true
  try {
    const res = await bomApi.list(queryParams)
    boms.value = res.data!.list || []
    total.value = res.data?.total || 0
  } catch (error: any) {
    ElMessage.error(error.message || '获取 BOM 列表失败')
    boms.value = []
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
  queryParams.product_name = ''
  queryParams.status = ''
  handleQuery()
}

const resetForm = () => {
  formData.id = undefined
  formData.product_id = undefined
  formData.product_name = ''
  formData.version = ''
  formData.is_default = false
  formData.status = 'draft'
  formData.remark = ''
  formData.items = []
}

const handleCreate = () => {
  resetForm()
  dialogMode.value = 'create'
  dialogVisible.value = true
}

const handleEdit = (row: Bom) => {
  resetForm()
  Object.assign(formData, {
    id: row.id,
    product_id: row.product_id,
    product_name: row.product_name,
    version: row.version,
    is_default: row.is_default,
    status: row.status,
    remark: row.remark,
    items: row.items || [],
  })
  dialogMode.value = 'edit'
  dialogVisible.value = true
}

const handleCopy = async (row: Bom) => {
  try {
    await ElMessageBox.confirm(
      `确定复制 BOM "${row.product_name} - ${row.version}" 吗？`,
      '复制确认'
    )
    await bomApi.copy(row.id)
    ElMessage.success('复制成功')
    fetchData()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '复制失败')
    }
  }
}

const handleSetDefault = async (row: Bom) => {
  try {
    await ElMessageBox.confirm(
      `确定将 BOM "${row.product_name} - ${row.version}" 设为默认吗？`,
      '设为默认确认'
    )
    await bomApi.setDefault(row.id)
    ElMessage.success('设置成功')
    fetchData()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '设置失败')
    }
  }
}

const handleDelete = async (row: Bom) => {
  try {
    await ElMessageBox.confirm(
      `确定删除 BOM "${row.product_name} - ${row.version}" 吗？`,
      '删除确认',
      { type: 'warning' }
    )
    await bomApi.delete(row.id)
    ElMessage.success('删除成功')
    fetchData()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '删除失败')
    }
  }
}

const handleSubmit = async (data: any) => {
  try {
    if (dialogMode.value === 'create') {
      await bomApi.create(data)
      ElMessage.success('创建成功')
    } else {
      await bomApi.update(formData.id!, data)
      ElMessage.success('更新成功')
    }
    dialogVisible.value = false
    fetchData()
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
  }
}

onMounted(() => {
  fetchData()
})
</script>

<style scoped>
.bom-page {
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
:deep(.el-card__header) {
  padding: 16px 20px;
  border-bottom: 1px solid #ebeef5;
}
:deep(.el-card__body) {
  padding: 20px;
}
</style>
