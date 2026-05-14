<template>
  <div class="cost-container">
    <el-card class="header-card">
      <div class="header-content">
        <h2>成本归集管理</h2>
        <p>管理和跟踪成本归集，查看成本分析汇总和批次成本</p>
      </div>
    </el-card>

    <!-- 筛选区 -->
    <el-card class="filter-card">
      <el-form :inline="true" :model="queryForm">
        <el-form-item label="归集编号">
          <el-input v-model="queryForm.collection_no" placeholder="请输入归集编号" clearable />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="queryForm.status" placeholder="请选择状态" clearable>
            <el-option
              v-for="(item, key) in COST_STATUS"
              :key="key"
              :label="item.label"
              :value="key"
            />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="fetchCollections">查询</el-button>
          <el-button @click="resetQuery">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- 操作区 -->
    <el-card class="table-card">
      <template #header>
        <div class="card-header">
          <span>成本归集列表</span>
          <el-button type="primary" @click="openDialog('create')">
            <el-icon><Plus /></el-icon>新建归集
          </el-button>
        </div>
      </template>
      
      <el-table
        :data="collectionList"
        v-loading="loading"
        stripe
        border
      >
        <el-table-column prop="collection_no" label="归集编号" width="160" />
        <el-table-column prop="cost_type" label="成本类型" width="140" />
        <el-table-column prop="period" label="期间" width="120" />
        <el-table-column prop="department_name" label="部门" min-width="160" />
        <el-table-column prop="total_cost" label="总成本" width="140">
          <template #default="{ row }">
            ¥{{ row.total_cost?.toFixed(2) }}
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="120">
          <template #default="{ row }">
            <el-tag :type="COST_STATUS[row.status as keyof typeof COST_STATUS]?.type">
              {{ COST_STATUS[row.status as keyof typeof COST_STATUS]?.label }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="220" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="viewDetail(row)">查看</el-button>
            <el-button type="success" link size="small" @click="openDialog('edit', row)" v-if="row.status === 'draft'">编辑</el-button>
            <el-button type="warning" link size="small" @click="handleSubmit(row)" v-if="row.status === 'draft'">提交</el-button>
            <el-button type="danger" link size="small" @click="handleDelete(row)" v-if="row.status === 'draft'">删除</el-button>
          </template>
        </el-table-column>
      </el-table>

      <!-- 分页 -->
      <div class="pagination-container">
        <el-pagination
          v-model:current-page="queryForm.page"
          v-model:page-size="queryForm.page_size"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="fetchCollections"
          @current-change="fetchCollections"
        />
      </div>
    </el-card>

    <!-- 新建/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="dialogType === 'create' ? '新建成本归集' : '编辑成本归集'"
      width="600px"
      @close="resetForm"
    >
      <el-form :model="costForm" :rules="costRules" ref="costFormRef" label-width="120px">
        <el-form-item label="归集编号" prop="collection_no">
          <el-input v-model="costForm.collection_no" placeholder="请输入归集编号" />
        </el-form-item>
        <el-form-item label="成本类型" prop="cost_type">
          <el-input v-model="costForm.cost_type" placeholder="请输入成本类型" />
        </el-form-item>
        <el-form-item label="期间" prop="period">
          <el-input v-model="costForm.period" placeholder="例如：2024-01" />
        </el-form-item>
        <el-form-item label="部门ID" prop="department_id">
          <el-input-number v-model="costForm.department_id" :min="1" style="width: 100%" />
        </el-form-item>
        <el-form-item label="总成本" prop="total_cost">
          <el-input-number v-model="costForm.total_cost" :min="0" :precision="2" style="width: 100%" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="costForm.remark" type="textarea" :rows="3" placeholder="请输入备注" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmitForm">确认</el-button>
      </template>
    </el-dialog>

    <!-- 详情对话框 -->
    <el-dialog v-model="detailVisible" title="成本归集详情" width="600px">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="归集编号">{{ currentCost?.collection_no }}</el-descriptions-item>
        <el-descriptions-item label="成本类型">{{ currentCost?.cost_type }}</el-descriptions-item>
        <el-descriptions-item label="期间">{{ currentCost?.period }}</el-descriptions-item>
        <el-descriptions-item label="部门ID">{{ currentCost?.department_id }}</el-descriptions-item>
        <el-descriptions-item label="总成本">¥{{ currentCost?.total_cost?.toFixed(2) }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="COST_STATUS[currentCost?.status as keyof typeof COST_STATUS]?.type">
            {{ COST_STATUS[currentCost?.status as keyof typeof COST_STATUS]?.label }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="创建时间">{{ currentCost?.created_at }}</el-descriptions-item>
        <el-descriptions-item label="更新时间">{{ currentCost?.updated_at }}</el-descriptions-item>
        <el-descriptions-item label="备注" :span="2">{{ currentCost?.remark || '-' }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  listCostCollections,
  createCostCollection,
  updateCostCollection,
  deleteCostCollection,
  type CostCollection,
  COST_STATUS,
} from '../../api/cost'

// 响应式数据
const loading = ref(false)
const submitLoading = ref(false)
const dialogVisible = ref(false)
const detailVisible = ref(false)
const dialogType = ref<'create' | 'edit'>('create')
const collectionList = ref<CostCollection[]>([])
const currentCost = ref<CostCollection | null>(null)
const costFormRef = ref<FormInstance>()
const total = ref(0)

// 查询表单
const queryForm = reactive({
  page: 1,
  page_size: 20,
  collection_no: '',
  status: '',
})

// 成本表单
const costForm = reactive<Partial<CostCollection>>({
  collection_no: '',
  cost_type: '',
  period: '',
  department_id: undefined,
  total_cost: undefined,
  status: 'draft',
  remark: '',
})

// 表单验证规则
const costRules: FormRules = {
  collection_no: [{ required: true, message: '请输入归集编号', trigger: 'blur' }],
  cost_type: [{ required: true, message: '请输入成本类型', trigger: 'blur' }],
  period: [{ required: true, message: '请输入期间', trigger: 'blur' }],
  department_id: [{ required: true, message: '请输入部门ID', trigger: 'blur' }],
  total_cost: [{ required: true, message: '请输入总成本', trigger: 'blur' }],
}

// 获取成本归集列表
const fetchCollections = async () => {
  loading.value = true
  try {
    const res = await listCostCollections(queryForm)
    collectionList.value = res.data?.list || []
    total.value = res.data?.total || 0
  } catch (e: any) {
    ElMessage.error(e.message || '获取成本归集列表失败')
  } finally {
    loading.value = false
  }
}

// 重置查询
const resetQuery = () => {
  queryForm.page = 1
  queryForm.collection_no = ''
  queryForm.status = ''
  fetchCollections()
}

// 打开对话框
const openDialog = (type: 'create' | 'edit', row?: CostCollection) => {
  dialogType.value = type
  resetForm()
  
  if (type === 'edit' && row) {
    Object.assign(costForm, row)
  }
  
  dialogVisible.value = true
}

// 重置表单
const resetForm = () => {
  Object.assign(costForm, {
    id: undefined,
    collection_no: '',
    cost_type: '',
    period: '',
    department_id: undefined,
    total_cost: undefined,
    status: 'draft',
    remark: '',
  })
  costFormRef.value?.clearValidate()
}

// 提交表单
const handleSubmitForm = async () => {
  if (!costFormRef.value) return
  
  await costFormRef.value.validate(async (valid) => {
    if (!valid) return
    
    submitLoading.value = true
    try {
      if (dialogType.value === 'create') {
        await createCostCollection(costForm)
        ElMessage.success('创建成功')
      } else {
        if (costForm.id) {
          await updateCostCollection(costForm.id, costForm)
          ElMessage.success('更新成功')
        }
      }
      
      dialogVisible.value = false
      fetchCollections()
    } catch (e: any) {
      ElMessage.error(e.message || '操作失败')
    } finally {
      submitLoading.value = false
    }
  })
}

// 查看详情
const viewDetail = (row: CostCollection) => {
  currentCost.value = row
  detailVisible.value = true
}

// 提交审核
const handleSubmit = async (row: CostCollection) => {
  try {
    await ElMessageBox.confirm(`确认提交归集 ${row.collection_no} 进行审核吗？`, '确认', {
      type: 'warning',
    })
    
    await updateCostCollection(row.id, { status: 'pending' })
    ElMessage.success('提交成功')
    fetchCollections()
  } catch (e: any) {
    if (e !== 'cancel') {
      ElMessage.error(e.message || '提交失败')
    }
  }
}

// 删除成本归集
const handleDelete = async (row: CostCollection) => {
  try {
    await ElMessageBox.confirm(`确认删除归集 ${row.collection_no} 吗？此操作不可恢复。`, '删除确认', {
      type: 'warning',
      confirmButtonText: '确定',
      cancelButtonText: '取消',
    })
    
    await deleteCostCollection(row.id)
    ElMessage.success('删除成功')
    fetchCollections()
  } catch (e: any) {
    if (e !== 'cancel') {
      ElMessage.error(e.message || '删除失败')
    }
  }
}

// 组件挂载时获取数据
onMounted(() => {
  fetchCollections()
})
</script>

<style scoped>
.cost-container {
  padding: 20px;
}

.header-card {
  margin-bottom: 20px;
}

.header-content h2 {
  margin: 0 0 8px 0;
  color: #303133;
}

.header-content p {
  margin: 0;
  color: #909399;
}

.filter-card {
  margin-bottom: 20px;
}

.table-card {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.pagination-container {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}
</style>
