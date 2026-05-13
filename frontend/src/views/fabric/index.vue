<template>
  <div class="fabric-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">面料管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>基础数据</el-breadcrumb-item>
          <el-breadcrumb-item>面料管理</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          新建面料
        </el-button>
        <el-button @click="handleImport">
          <el-icon><Upload /></el-icon>
          导入
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
          <el-input v-model="queryParams.keyword" placeholder="面料编码/名称" clearable @clear="handleQuery" />
        </el-form-item>
        <el-form-item label="分类">
          <el-select v-model="queryParams.category_id" placeholder="选择分类" clearable @change="handleQuery">
            <el-option v-for="cat in categories" :key="cat.id" :label="cat.name" :value="cat.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="queryParams.is_active" placeholder="选择状态" clearable @change="handleQuery">
            <el-option label="启用" :value="true" />
            <el-option label="禁用" :value="false" />
          </el-select>
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
      <el-table v-loading="loading" :data="fabrics" stripe @selection-change="handleSelectionChange">
        <el-table-column type="selection" width="50" />
        <el-table-column prop="fabric_code" label="面料编码" width="150" fixed />
        <el-table-column prop="fabric_name" label="面料名称" min-width="180" fixed />
        <el-table-column prop="category_name" label="分类" width="120">
          <template #default="{ row }">
            <el-tag v-if="row.category_name" type="info">{{ row.category_name }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="composition" label="成分" width="120" show-overflow-tooltip />
        <el-table-column prop="weight" label="克重" width="100" />
        <el-table-column prop="width" label="门幅" width="100" />
        <el-table-column prop="stock_quantity" label="库存" width="100">
          <template #default="{ row }">
            <span :class="{ 'low-stock': row.stock_quantity < (row.min_stock || 10) }">
              {{ row.stock_quantity || 0 }}
            </span>
          </template>
        </el-table-column>
        <el-table-column prop="price" label="单价" width="100">
          <template #default="{ row }">
            <span v-if="row.price">¥{{ row.price.toFixed(2) }}</span>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column prop="supplier_name" label="供应商" width="150" show-overflow-tooltip />
        <el-table-column prop="is_active" label="状态" width="80">
          <template #default="{ row }">
            <el-tag :type="row.is_active ? 'success' : 'info'" size="small">
              {{ row.is_active ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleView(row)">
              <el-icon><View /></el-icon>
              详情
            </el-button>
            <el-button type="primary" link size="small" @click="handleEdit(row)">
              <el-icon><Edit /></el-icon>
              编辑
            </el-button>
            <el-button type="danger" link size="small" @click="handleDelete(row)">
              <el-icon><Delete /></el-icon>
              删除
            </el-button>
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

    <el-dialog v-model="dialogVisible" :title="dialogTitle" width="700px" destroy-on-close>
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="面料编码" prop="fabric_code">
              <el-input v-model="formData.fabric_code" placeholder="自动生成或手动输入" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="面料名称" prop="fabric_name">
              <el-input v-model="formData.fabric_name" placeholder="请输入面料名称" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="面料分类" prop="category_id">
              <el-select v-model="formData.category_id" placeholder="选择分类" style="width: 100%">
                <el-option v-for="cat in categories" :key="cat.id" :label="cat.name" :value="cat.id" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="供应商" prop="supplier_id">
              <el-select v-model="formData.supplier_id" placeholder="选择供应商" style="width: 100%">
                <el-option label="供应商A" :value="1" />
                <el-option label="供应商B" :value="2" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="成分" prop="composition">
          <el-input v-model="formData.composition" placeholder="如: 100%棉, 棉65%/聚酯35%" />
        </el-form-item>
        <el-row :gutter="20">
          <el-col :span="8">
            <el-form-item label="克重" prop="weight">
              <el-input v-model="formData.weight" placeholder="如: 200g/m²">
                <template #append>g/m²</template>
              </el-input>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="门幅" prop="width">
              <el-input v-model="formData.width" placeholder="如: 150">
                <template #append>cm</template>
              </el-input>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="单价" prop="price">
              <el-input v-model.number="formData.price" placeholder="0.00">
                <template #append>元</template>
              </el-input>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="库存数量" prop="stock_quantity">
              <el-input-number v-model="formData.stock_quantity" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="最小库存" prop="min_stock">
              <el-input-number v-model="formData.min_stock" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="颜色" prop="color">
          <el-input v-model="formData.color" placeholder="可选" />
        </el-form-item>
        <el-form-item label="描述" prop="description">
          <el-input v-model="formData.description" type="textarea" :rows="3" placeholder="面料详细描述" />
        </el-form-item>
        <el-form-item label="状态">
          <el-switch v-model="formData.is_active" active-text="启用" inactive-text="禁用" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSubmit">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="viewDialogVisible" title="面料详情" width="800px">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="面料编码">{{ currentFabric?.fabric_code }}</el-descriptions-item>
        <el-descriptions-item label="面料名称">{{ currentFabric?.fabric_name }}</el-descriptions-item>
        <el-descriptions-item label="分类">{{ currentFabric?.category_name }}</el-descriptions-item>
        <el-descriptions-item label="供应商">{{ currentFabric?.supplier_name }}</el-descriptions-item>
        <el-descriptions-item label="成分">{{ currentFabric?.composition }}</el-descriptions-item>
        <el-descriptions-item label="克重">{{ currentFabric?.weight }} g/m²</el-descriptions-item>
        <el-descriptions-item label="门幅">{{ currentFabric?.width }} cm</el-descriptions-item>
        <el-descriptions-item label="单价">¥{{ currentFabric?.price?.toFixed(2) }}</el-descriptions-item>
        <el-descriptions-item label="库存">{{ currentFabric?.stock_quantity }}</el-descriptions-item>
        <el-descriptions-item label="最小库存">{{ currentFabric?.min_stock }}</el-descriptions-item>
        <el-descriptions-item label="颜色">{{ currentFabric?.color || '-' }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="currentFabric?.is_active ? 'success' : 'info'">
            {{ currentFabric?.is_active ? '启用' : '禁用' }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="描述" :span="2">{{ currentFabric?.description || '-' }}</el-descriptions-item>
        <el-descriptions-item label="创建时间">{{ currentFabric?.created_at }}</el-descriptions-item>
        <el-descriptions-item label="更新时间">{{ currentFabric?.updated_at }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  Plus, Upload, Download, Search, Refresh, View, Edit, Delete
} from '@element-plus/icons-vue'
import { useFabricStore } from '@/store/fabric'
import { fabricApi, type Fabric } from '@/api/fabric'

const fabricStore = useFabricStore()
const loading = ref(false)
const fabrics = ref<Fabric[]>([])
const categories = ref<any[]>([])
const total = ref(0)
const selectedRows = ref<Fabric[]>([])

const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  category_id: undefined as number | undefined,
  supplier_id: undefined as number | undefined,
  is_active: undefined as boolean | undefined
})

const dialogVisible = ref(false)
const viewDialogVisible = ref(false)
const dialogTitle = ref('')
const formRef = ref()
const currentFabric = ref<Fabric | null>(null)
const isEdit = ref(false)

const formData = reactive<Partial<Fabric>>({
  fabric_code: '',
  fabric_name: '',
  category_id: undefined,
  supplier_id: undefined,
  composition: '',
  weight: '',
  width: '',
  color: '',
  price: 0,
  stock_quantity: 0,
  min_stock: 10,
  description: '',
  is_active: true
})

const formRules = {
  fabric_name: [{ required: true, message: '请输入面料名称', trigger: 'blur' }]
}

const fetchData = async () => {
  loading.value = true
  try {
    await fabricStore.fetchFabrics(queryParams)
    fabrics.value = fabricStore.fabrics
    total.value = fabricStore.total
  } catch (error) {
    fabrics.value = [
      {
        id: 1,
        fabric_code: 'FB001',
        fabric_name: '纯棉斜纹布',
        category_name: '棉布',
        composition: '100%棉',
        weight: '200g/m²',
        width: '150cm',
        stock_quantity: 500,
        min_stock: 100,
        price: 25.5,
        supplier_name: '纺织供应商A',
        is_active: true
      },
      {
        id: 2,
        fabric_code: 'FB002',
        fabric_name: '涤纶平纹布',
        category_name: '化纤',
        composition: '100%涤纶',
        weight: '150g/m²',
        width: '145cm',
        stock_quantity: 1200,
        min_stock: 200,
        price: 18.0,
        supplier_name: '化纤供应商B',
        is_active: true
      },
      {
        id: 3,
        fabric_code: 'FB003',
        fabric_name: '真丝缎面',
        category_name: '丝绸',
        composition: '100%桑蚕丝',
        weight: '80g/m²',
        width: '110cm',
        stock_quantity: 50,
        min_stock: 100,
        price: 180.0,
        supplier_name: '丝绸供应商C',
        is_active: true
      }
    ]
    total.value = 3
    ElMessage.info('使用演示数据')
  } finally {
    loading.value = false
  }
}

const fetchCategories = async () => {
  try {
    const res = await fabricApi.getCategories()
    categories.value = res.data
  } catch (error) {
    categories.value = [
      { id: 1, name: '棉布' },
      { id: 2, name: '化纤' },
      { id: 3, name: '丝绸' },
      { id: 4, name: '麻布' },
      { id: 5, name: '混纺' }
    ]
  }
}

const handleQuery = () => {
  queryParams.page = 1
  fetchData()
}

const handleReset = () => {
  queryParams.keyword = ''
  queryParams.category_id = undefined
  queryParams.supplier_id = undefined
  queryParams.is_active = undefined
  handleQuery()
}

const handleCreate = () => {
  isEdit.value = false
  dialogTitle.value = '新建面料'
  Object.assign(formData, {
    fabric_code: '',
    fabric_name: '',
    category_id: undefined,
    supplier_id: undefined,
    composition: '',
    weight: '',
    width: '',
    color: '',
    price: 0,
    stock_quantity: 0,
    min_stock: 10,
    description: '',
    is_active: true
  })
  dialogVisible.value = true
}

const handleEdit = (row: Fabric) => {
  isEdit.value = true
  dialogTitle.value = '编辑面料'
  Object.assign(formData, row)
  dialogVisible.value = true
}

const handleView = (row: Fabric) => {
  currentFabric.value = row
  viewDialogVisible.value = true
}

const handleDelete = async (row: Fabric) => {
  try {
    await ElMessageBox.confirm(
      `确定要删除面料 "${row.fabric_name}" 吗？`,
      '删除确认',
      { type: 'warning' }
    )
    const success = await fabricStore.deleteFabric(row.id)
    if (success) {
      ElMessage.success('删除成功')
      fetchData()
    }
  } catch {
    // User cancelled
  }
}

const handleSubmit = async () => {
  try {
    await formRef.value.validate()
    if (isEdit.value && formData.id) {
      await fabricStore.updateFabric(formData.id, formData)
      ElMessage.success('更新成功')
    } else {
      await fabricStore.createFabric(formData)
      ElMessage.success('创建成功')
    }
    dialogVisible.value = false
    fetchData()
  } catch (error) {
    console.error('Form validation failed:', error)
  }
}

const handleSelectionChange = (selection: Fabric[]) => {
  selectedRows.value = selection
}

const handleImport = () => {
  ElMessage.info('导入功能开发中')
}

const handleExport = () => {
  ElMessage.info('导出功能开发中')
}

onMounted(() => {
  fetchData()
  fetchCategories()
})
</script>

<style scoped>
.fabric-page {
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

.filter-form {
  margin-bottom: 0;
}

.table-card {
  margin-bottom: 20px;
}

.pagination-wrapper {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}

.low-stock {
  color: #f56c6c;
  font-weight: 600;
}

:deep(.el-table) {
  font-size: 14px;
}

:deep(.el-card__header) {
  padding: 16px 20px;
  border-bottom: 1px solid #ebeef5;
}

:deep(.el-card__body) {
  padding: 20px;
}
</style>
