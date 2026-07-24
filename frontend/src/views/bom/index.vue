<template>
  <div class="bom-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">{{ $t('bomModule.title') }}</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">{{ $t('bomModule.breadcrumb.home') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ $t('bomModule.breadcrumb.production') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ $t('bomModule.breadcrumb.bom') }}</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          {{ $t('bomModule.create') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form" :aria-label="$t('bomModule.filter.ariaLabel')">
        <el-form-item :label="$t('bomModule.filter.productName')">
          <el-input v-model="queryParams.product_name" :placeholder="$t('bomModule.filter.productNamePlaceholder')" clearable />
        </el-form-item>
        <el-form-item :label="$t('bomModule.filter.status')">
          <el-select v-model="queryParams.status" :placeholder="$t('bomModule.filter.statusPlaceholder')" clearable>
            <el-option :label="$t('bomModule.status.draft')" value="draft" />
            <el-option :label="$t('bomModule.status.active')" value="active" />
            <el-option :label="$t('bomModule.status.archived')" value="archived" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">{{ $t('bomModule.filter.query') }}</el-button>
          <el-button @click="handleReset">{{ $t('bomModule.filter.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table v-loading="loading" :data="boms" stripe :aria-label="$t('bomModule.table.ariaLabel')">
        <el-table-column prop="product_code" :label="$t('bomModule.table.productCode')" width="140" fixed />
        <el-table-column prop="product_name" :label="$t('bomModule.table.productName')" min-width="180" fixed />
        <el-table-column prop="version" :label="$t('bomModule.table.version')" width="100" />
        <el-table-column prop="is_default" :label="$t('bomModule.table.isDefault')" width="80">
          <template #default="{ row }">
            <el-tag v-if="row.is_default" type="success" size="small">{{ $t('bomModule.defaultTag') }}</el-tag>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column prop="status" :label="$t('bomModule.table.status')" width="100">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)" size="small">
              {{ getStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="remark" :label="$t('bomModule.table.remark')" min-width="150" show-overflow-tooltip />
        <el-table-column prop="updated_at" :label="$t('bomModule.table.updatedAt')" width="180" />
        <el-table-column :label="$t('bomModule.table.operation')" width="280" fixed="right">
          <template #default="{ row }">
            <!-- P3 维度 10 修复（批次 87）：编辑/复制/设默认/删除按钮补齐 v-permission -->
            <!-- v11 批次 169 P2-1 修复：row as any 改为 row as Bom -->
            <el-button
              v-permission="'bom:update'"
              type="primary"
              link
              size="small"
              @click="handleEdit(row as Bom)"
              >{{ $t('bomModule.table.edit') }}</el-button
            >
            <el-button
              v-permission="'bom:create'"
              type="primary"
              link
              size="small"
              @click="handleCopy(row as Bom)"
              >{{ $t('bomModule.table.copy') }}</el-button
            >
            <el-button
              v-if="!row.is_default"
              v-permission="'bom:update'"
              type="success"
              link
              size="small"
              @click="handleSetDefault(row as Bom)"
            >
              {{ $t('bomModule.table.setDefault') }}
            </el-button>
            <el-button
              v-permission="'bom:delete'"
              type="danger"
              link
              size="small"
              @click="handleDelete(row as Bom)"
              >{{ $t('bomModule.table.delete') }}</el-button
            >
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
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
          :aria-label="$t('bomModule.table.paginationAriaLabel')"
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
      :aria-label="formData.id ? $t('bomModule.dialog.editAriaLabel') : $t('bomModule.dialog.createAriaLabel')"
    >
      <BillOfMaterialsForm
        ref="billOfMaterialsFormRef"
        :form-data="formData"
        :mode="dialogMode"
        @submit="handleSubmit"
        @cancel="dialogVisible = false"
      />
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  copyBom,
  setDefaultBom,
  deleteBom,
  createBom,
  updateBom,
  type Bom,
} from '@/api/bom'
import BillOfMaterialsForm from './BillOfMaterialsForm.vue'
import { useTableApi } from '@/composables/useTableApi'

const { t } = useI18n({ useScope: 'global' })

const dialogVisible = ref(false)
const dialogMode = ref<'create' | 'edit'>('create')
const billOfMaterialsFormRef = ref<InstanceType<typeof BillOfMaterialsForm>>()

const queryParams = reactive({
  product_name: '',
  status: '',
})

// 批次 275：接入 useTableApi，消除手写 boms/total/loading/fetchData 重复
// useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: boms,
  loading,
  page,
  pageSize,
  total,
  refresh: fetchData,
  setQueryParam,
} = useTableApi<Bom>({
  url: '/boms',
  onError: (err: unknown) =>
    ElMessage.error((err instanceof Error ? err.message : String(err)) || t('bomModule.message.fetchFailed')),
})

// 批次 275：同步筛选条件到 useTableApi.queryParams 并刷新
const syncQueryParams = () => {
  setQueryParam('product_name', queryParams.product_name || undefined)
  setQueryParam('status', queryParams.status || undefined)
}

const handleQuery = () => {
  syncQueryParams()
  page.value = 1
  fetchData()
}

const handleReset = () => {
  queryParams.product_name = ''
  queryParams.status = ''
  syncQueryParams()
  page.value = 1
  fetchData()
}

// 分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handlePageChange = (p: number) => {
  page.value = p
}

const handleSizeChange = (s: number) => {
  pageSize.value = s
  page.value = 1
}

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
  return dialogMode.value === 'create' ? t('bomModule.dialog.createTitle') : t('bomModule.dialog.editTitle')
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
    draft: t('bomModule.status.draft'),
    active: t('bomModule.status.active'),
    archived: t('bomModule.status.archived'),
  }
  return labels[status] || status
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
      t('bomModule.message.copyConfirm', { name: `${row.product_name} - ${row.version}` }),
      t('bomModule.message.copyConfirmTitle')
    )
    await copyBom(row.id)
    ElMessage.success(t('bomModule.message.copySuccess'))
    fetchData()
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    if (error !== 'cancel') {
      ElMessage.error((error instanceof Error ? error.message : String(error)) || t('bomModule.message.copyFailed'))
    }
  }
}

const handleSetDefault = async (row: Bom) => {
  try {
    await ElMessageBox.confirm(
      t('bomModule.message.setDefaultConfirm', { name: `${row.product_name} - ${row.version}` }),
      t('bomModule.message.setDefaultConfirmTitle')
    )
    await setDefaultBom(row.id)
    ElMessage.success(t('bomModule.message.setDefaultSuccess'))
    fetchData()
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    if (error !== 'cancel') {
      ElMessage.error((error instanceof Error ? error.message : String(error)) || t('bomModule.message.setDefaultFailed'))
    }
  }
}

const handleDelete = async (row: Bom) => {
  try {
    await ElMessageBox.confirm(
      t('bomModule.message.deleteConfirm', { name: `${row.product_name} - ${row.version}` }),
      t('bomModule.message.deleteConfirmTitle'),
      { type: 'warning' }
    )
    await deleteBom(row.id)
    ElMessage.success(t('bomModule.message.deleteSuccess'))
    fetchData()
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    if (error !== 'cancel') {
      ElMessage.error((error instanceof Error ? error.message : String(error)) || t('bomModule.message.deleteFailed'))
    }
  }
}

// v11 批次 169 P2-1 修复：data: any 改为 Partial<Bom>
const handleSubmit = async (data: Partial<Bom>) => {
  try {
    if (dialogMode.value === 'create') {
      await createBom(data)
      ElMessage.success(t('bomModule.message.createSuccess'))
    } else {
      await updateBom(formData.id!, data)
      ElMessage.success(t('bomModule.message.updateSuccess'))
    }
    dialogVisible.value = false
    fetchData()
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error((error instanceof Error ? error.message : String(error)) || t('bomModule.message.operateFailed'))
  }
}

// 批次 275：useTableApi 构造时自动初始加载，无需 onMounted 调用 fetchData
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
