<template>
  <div class="dye-batch-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">缸号管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>面料行业</el-breadcrumb-item>
          <el-breadcrumb-item>缸号管理</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          新建缸号
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          导出
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form" aria-label="缸号筛选表单">
        <el-form-item label="关键词">
          <el-input
            v-model="queryParams.keyword"
            placeholder="缸号/批次号"
            clearable
            @clear="handleQuery"
          />
        </el-form-item>
        <el-form-item label="色号">
          <el-input
            v-model="queryParams.color_no"
            placeholder="请输入色号"
            clearable
            @clear="handleQuery"
          />
        </el-form-item>
        <el-form-item label="产品">
          <el-select
            v-model="queryParams.product_id"
            placeholder="选择产品"
            clearable
            filterable
            @change="handleQuery"
          >
            <el-option v-for="p in products" :key="p.id" :label="p.product_name" :value="p.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="状态">
          <el-select
            v-model="queryParams.status"
            placeholder="选择状态"
            clearable
            @change="handleQuery"
          >
            <el-option label="进行中" value="ACTIVE" />
            <el-option label="已完成" value="COMPLETED" />
          </el-select>
        </el-form-item>
        <el-form-item label="染色日期">
          <el-date-picker
            v-model="queryParams.date_range"
            type="daterange"
            range-separator="至"
            start-placeholder="开始日期"
            end-placeholder="结束日期"
            @change="handleQuery"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">
            <el-icon><Search /></el-icon>
            查询
          </el-button>
          <el-button @click="handleReset">
            <el-icon><Refresh /></el-icon>
            重置
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table v-loading="loading" :data="dyeBatchList" border stripe aria-label="缸号列表">
        <el-table-column type="index" label="序号" width="60" align="center" />
        <el-table-column prop="batch_no" label="缸号" width="120" show-overflow-tooltip />
        <el-table-column prop="product_name" label="产品" width="150" show-overflow-tooltip />
        <el-table-column prop="color_no" label="色号" width="100" show-overflow-tooltip />
        <el-table-column prop="color_code" label="颜色代码" width="100" show-overflow-tooltip />
        <el-table-column prop="dye_date" label="染色日期" width="120" align="center" />
        <el-table-column prop="quantity" label="染色数量" width="100" align="right" />
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="remarks" label="备注" min-width="150" show-overflow-tooltip />
        <el-table-column label="操作" width="200" align="center" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleView(row as DyeBatch)"
              >查看</el-button
            >
            <el-button
              v-if="row.status === 'ACTIVE'"
              type="primary"
              link
              size="small"
              @click="handleEdit(row as DyeBatch)"
              >编辑</el-button
            >
            <el-button
              v-if="row.status === 'ACTIVE'"
              type="success"
              link
              size="small"
              @click="handleComplete(row as DyeBatch)"
              >完成</el-button
            >
            <el-button
              v-if="row.status === 'ACTIVE'"
              type="danger"
              link
              size="small"
              @click="handleDelete(row as DyeBatch)"
              >删除</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-container">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="handleSizeChange"
          @current-change="handleCurrentChange"
          aria-label="缸号列表分页"
        />
      </div>
    </el-card>

    <!-- 新建/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="dialogTitle"
      width="700px"
      :close-on-click-modal="false"
      aria-label="缸号编辑对话框"
    >
      <el-form ref="formRef" :model="formData" :rules="formRules" :disabled="isView" label-width="100px" aria-label="缸号表单">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="缸号" prop="batch_no">
              <el-input v-model="formData.batch_no" placeholder="请输入缸号" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="产品" prop="product_id">
              <el-select v-model="formData.product_id" placeholder="请选择产品" filterable>
                <el-option
                  v-for="p in products"
                  :key="p.id"
                  :label="p.product_name"
                  :value="p.id"
                />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="色号" prop="color_no">
              <el-input v-model="formData.color_no" placeholder="请输入色号" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="颜色代码" prop="color_code">
              <el-input v-model="formData.color_code" placeholder="请输入颜色代码" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="染色日期" prop="dye_date">
              <el-date-picker
                v-model="formData.dye_date"
                type="date"
                placeholder="请选择染色日期"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="染色数量" prop="quantity">
              <el-input-number
                v-model="formData.quantity"
                :precision="2"
                :min="0"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="备注" prop="remarks">
          <el-input v-model="formData.remarks" type="textarea" :rows="3" placeholder="请输入备注" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ isView ? '关闭' : '取消' }}</el-button>
        <el-button v-if="!isView" type="primary" @click="handleSubmitForm">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Download, Search, Refresh } from '@element-plus/icons-vue'
import {
  createDyeBatch,
  updateDyeBatch,
  deleteDyeBatch,
  completeDyeBatch,
  exportDyeBatches,
} from '@/api/dye-batch'
import type { DyeBatch } from '@/api/dye-batch'
import { getProductList } from '@/api/product'
import type { Product } from '@/api/product'
import { logger } from '@/utils/logger'
import { useTableApi } from '@/composables/useTableApi'

// 查询参数（筛选条件，分页由 useTableApi 管理）
const queryParams = reactive({
  keyword: '',
  color_no: '',
  product_id: '',
  status: '',
  date_range: [] as string[],
})

// 批次 271：接入 useTableApi，消除手写 page/pageSize/total/loading + getList 重复
// useTableApi 自动管理分页状态、loading、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: dyeBatchList,
  loading,
  page,
  pageSize,
  total,
  refresh,
  setQueryParam,
} = useTableApi<DyeBatch>({
  url: '/production/dye-batches',
  onError: (e: unknown) => logger.error('获取缸号列表失败:', String(e)),
})

// 产品列表
const products = ref<Product[]>([])

// 对话框
const dialogVisible = ref(false)
const dialogTitle = ref('')
const formRef = ref()
const isView = ref(false)

// 表单数据
const formData = reactive({
  id: undefined as number | undefined,
  batch_no: '',
  product_id: '',
  color_no: '',
  color_code: '',
  dye_date: '',
  quantity: 0,
  remarks: '',
})

// 表单验证规则
const formRules = {
  batch_no: [{ required: true, message: '请输入缸号', trigger: 'blur' }],
  product_id: [{ required: true, message: '请选择产品', trigger: 'change' }],
  color_no: [{ required: true, message: '请输入色号', trigger: 'blur' }],
  dye_date: [{ required: true, message: '请选择染色日期', trigger: 'change' }],
  quantity: [{ required: true, message: '请输入染色数量', trigger: 'blur' }],
}

// 批次 271：同步筛选条件到 useTableApi.queryParams 并刷新
// useTableApi 自动 watch page/pageSize 变化触发重载，无需手动 getList
const syncQueryParams = () => {
  setQueryParam('keyword', queryParams.keyword || undefined)
  setQueryParam('color_no', queryParams.color_no || undefined)
  setQueryParam('product_id', queryParams.product_id || undefined)
  setQueryParam('status', queryParams.status || undefined)
  setQueryParam('date_range', queryParams.date_range?.length ? queryParams.date_range : undefined)
}

// 获取产品列表
const getProducts = async () => {
  try {
    const res = await getProductList({ page: 1, page_size: 1000 })
    products.value = res.data?.list || []
  } catch (error) {
    logger.error('获取产品列表失败:', error)
  }
}

// 查询
const handleQuery = () => {
  syncQueryParams()
  page.value = 1
  refresh()
}

// 重置
const handleReset = () => {
  queryParams.keyword = ''
  queryParams.color_no = ''
  queryParams.product_id = ''
  queryParams.status = ''
  queryParams.date_range = []
  syncQueryParams()
  page.value = 1
  refresh()
}

// 新建
const handleCreate = () => {
  dialogTitle.value = '新建缸号'
  isView.value = false
  Object.assign(formData, {
    id: undefined,
    batch_no: '',
    product_id: '',
    color_no: '',
    color_code: '',
    dye_date: '',
    quantity: 0,
    remarks: '',
  })
  dialogVisible.value = true
}

// 查看（v14 P0-3 修复：实现只读查看功能，原 handler 为空导致业务失效）
const handleView = (row: DyeBatch) => {
  dialogTitle.value = '查看缸号'
  isView.value = true
  Object.assign(formData, row)
  dialogVisible.value = true
}

// 编辑
const handleEdit = (row: DyeBatch) => {
  dialogTitle.value = '编辑缸号'
  isView.value = false
  Object.assign(formData, row)
  dialogVisible.value = true
}

// 完成
const handleComplete = async (row: DyeBatch) => {
  try {
    await ElMessageBox.confirm('确认完成该缸号？', '提示', { type: 'warning' })
    await completeDyeBatch(row.id)
    ElMessage.success('操作成功')
    refresh()
  } catch (error) {
    logger.error('操作失败:', error)
  }
}

// 删除
const handleDelete = async (row: DyeBatch) => {
  try {
    await ElMessageBox.confirm('确认删除该缸号？', '提示', { type: 'warning' })
    await deleteDyeBatch(row.id)
    ElMessage.success('删除成功')
    refresh()
  } catch (error) {
    logger.error('删除失败:', error)
  }
}

// 导出
const handleExport = async () => {
  try {
    const res = await exportDyeBatches(queryParams)
    const url = window.URL.createObjectURL(new Blob([res]))
    const link = document.createElement('a')
    link.href = url
    link.setAttribute('download', '缸号管理.xlsx')
    document.body.appendChild(link)
    link.click()
    document.body.removeChild(link)
    window.URL.revokeObjectURL(url)
    ElMessage.success('导出成功')
  } catch (error) {
    logger.error('导出失败:', error)
  }
}

// 提交表单
const handleSubmitForm = async () => {
  try {
    await formRef.value?.validate()
    if (formData.id) {
      await updateDyeBatch(formData.id, formData)
    } else {
      await createDyeBatch(formData)
    }
    ElMessage.success('保存成功')
    dialogVisible.value = false
    refresh()
  } catch (error) {
    logger.error('表单验证失败:', error)
  }
}

// 分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handleSizeChange = (val: number) => {
  pageSize.value = val
  page.value = 1
}

const handleCurrentChange = (val: number) => {
  page.value = val
}

// 获取状态类型
const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    ACTIVE: 'warning',
    COMPLETED: 'success',
  }
  return map[status] || 'info'
}

// 获取状态标签
const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    ACTIVE: '进行中',
    COMPLETED: '已完成',
  }
  return map[status] || status
}

const hasLoaded = createLazyLoader()

// 批次 271：useTableApi 构造时自动初始加载，无需 onMounted 调用 getList
onMounted(() => {
  loadIfNot('products', getProducts, hasLoaded)
})
</script>

<style scoped>
.dye-batch-page {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.header-left {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.page-title {
  margin: 0;
  font-size: 24px;
  font-weight: 600;
}

.header-actions {
  display: flex;
  gap: 10px;
}

.filter-card {
  margin-bottom: 20px;
}

.filter-form {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.table-card {
  margin-bottom: 20px;
}

.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 20px;
}
</style>
