<template>
  <div class="budget-container">
    <el-card class="header-card">
      <div class="header-content">
        <h2>预算管理</h2>
        <p>管理和跟踪预算计划，进行预算审批和执行</p>
      </div>
    </el-card>

    <el-card class="table-card">
      <template #header>
        <div class="card-header">
          <span>预算列表</span>
          <el-button type="primary" @click="openDialog('create')">
            <el-icon><Plus /></el-icon>新建预算
          </el-button>
        </div>
      </template>

      <el-table v-loading="loading" :data="budgetList" stripe border>
        <el-table-column prop="budget_no" label="预算编号" width="160" />
        <el-table-column prop="name" label="预算名称" min-width="160" />
        <el-table-column prop="period" label="期间" width="120" />
        <el-table-column prop="department_name" label="部门" min-width="160" />
        <el-table-column prop="total_amount" label="总金额" width="140">
          <template #default="{ row }"> ¥{{ row.total_amount?.toFixed(2) }} </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="120">
          <template #default="{ row }">
            <el-tag :type="BUDGET_STATUS[row.status as keyof typeof BUDGET_STATUS]?.type">
              {{ BUDGET_STATUS[row.status as keyof typeof BUDGET_STATUS]?.label }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="220" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="viewDetail(row as any)"
              >查看</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              type="success"
              link
              size="small"
              @click="openDialog('edit', row)"
              >编辑</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              type="warning"
              link
              size="small"
              @click="handleSubmit(row as any)"
              >提交</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              type="danger"
              link
              size="small"
              @click="handleDelete(row as any)"
              >删除</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-container">
        <el-pagination
          v-model:current-page="queryForm.page"
          v-model:page-size="queryForm.page_size"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="fetchBudgets"
          @current-change="fetchBudgets"
        />
      </div>
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="dialogType === 'create' ? '新建预算' : '编辑预算'"
      width="600px"
      @close="resetForm"
    >
      <el-form ref="budgetFormRef" :model="budgetForm" :rules="budgetRules" label-width="120px">
        <el-form-item label="预算编号" prop="budget_no">
          <el-input v-model="budgetForm.budget_no" placeholder="请输入预算编号" />
        </el-form-item>
        <el-form-item label="预算名称" prop="name">
          <el-input v-model="budgetForm.name" placeholder="请输入预算名称" />
        </el-form-item>
        <el-form-item label="期间" prop="period">
          <el-input v-model="budgetForm.period" placeholder="例如：2024-01" />
        </el-form-item>
        <el-form-item label="部门ID" prop="department_id">
          <el-input-number v-model="budgetForm.department_id" :min="1" style="width: 100%" />
        </el-form-item>
        <el-form-item label="总金额" prop="total_amount">
          <el-input-number
            v-model="budgetForm.total_amount"
            :min="0"
            :precision="2"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="备注">
          <el-input
            v-model="budgetForm.remark"
            type="textarea"
            :rows="3"
            placeholder="请输入备注"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmitForm"
          >确认</el-button
        >
      </template>
    </el-dialog>

    <el-dialog v-model="detailVisible" title="预算详情" width="600px">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="预算编号">{{ currentBudget?.budget_no }}</el-descriptions-item>
        <el-descriptions-item label="预算名称">{{ currentBudget?.name }}</el-descriptions-item>
        <el-descriptions-item label="期间">{{ currentBudget?.period }}</el-descriptions-item>
        <el-descriptions-item label="部门ID">{{
          currentBudget?.department_id
        }}</el-descriptions-item>
        <el-descriptions-item label="总金额"
          >¥{{ currentBudget?.total_amount?.toFixed(2) }}</el-descriptions-item
        >
        <el-descriptions-item label="状态">
          <el-tag :type="BUDGET_STATUS[currentBudget?.status as keyof typeof BUDGET_STATUS]?.type">
            {{ BUDGET_STATUS[currentBudget?.status as keyof typeof BUDGET_STATUS]?.label }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="备注" :span="2">{{
          currentBudget?.remark || '-'
        }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  listBudgets,
  createBudget,
  updateBudget,
  deleteBudget,
  type Budget,
  BUDGET_STATUS,
} from '../../api/budget'

const loading = ref(false)
const submitLoading = ref(false)
const dialogVisible = ref(false)
const detailVisible = ref(false)
const dialogType = ref<'create' | 'edit'>('create')
const budgetList = ref<Budget[]>([])
const currentBudget = ref<Budget | null>(null)
const budgetFormRef = ref<FormInstance>()
const total = ref(0)

const queryForm = reactive({
  page: 1,
  page_size: 20,
})

const budgetForm = reactive<Partial<Budget>>({
  budget_no: '',
  name: '',
  period: '',
  department_id: undefined,
  total_amount: undefined,
  status: 'draft',
  remark: '',
})

const budgetRules: FormRules = {
  budget_no: [{ required: true, message: '请输入预算编号', trigger: 'blur' }],
  name: [{ required: true, message: '请输入预算名称', trigger: 'blur' }],
  period: [{ required: true, message: '请输入期间', trigger: 'blur' }],
  department_id: [{ required: true, message: '请输入部门ID', trigger: 'blur' }],
  total_amount: [{ required: true, message: '请输入总金额', trigger: 'blur' }],
}

const fetchBudgets = async () => {
  loading.value = true
  try {
    const res = await listBudgets(queryForm)
    budgetList.value = res.data!.list || []
    total.value = res.data?.total || 0
  } catch (e: any) {
    ElMessage.error(e.message || '获取预算列表失败')
  } finally {
    loading.value = false
  }
}

const openDialog = (type: 'create' | 'edit', row?: Budget) => {
  dialogType.value = type
  resetForm()

  if (type === 'edit' && row) {
    Object.assign(budgetForm, row)
  }

  dialogVisible.value = true
}

const resetForm = () => {
  Object.assign(budgetForm, {
    id: undefined,
    budget_no: '',
    name: '',
    period: '',
    department_id: undefined,
    total_amount: undefined,
    status: 'draft',
    remark: '',
  })
  budgetFormRef.value?.clearValidate()
}

const handleSubmitForm = async () => {
  if (!budgetFormRef.value) return

  await budgetFormRef.value.validate(async (valid) => {
    if (!valid) return

    submitLoading.value = true
    try {
      if (dialogType.value === 'create') {
        await createBudget(budgetForm)
        ElMessage.success('创建成功')
      } else {
        if (budgetForm.id) {
          await updateBudget(budgetForm.id, budgetForm)
          ElMessage.success('更新成功')
        }
      }

      dialogVisible.value = false
      fetchBudgets()
    } catch (e: any) {
      ElMessage.error(e.message || '操作失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const viewDetail = (row: Budget) => {
  currentBudget.value = row
  detailVisible.value = true
}

const handleSubmit = async (row: Budget) => {
  try {
    await ElMessageBox.confirm(`确认提交预算 ${row.budget_no} 进行审核吗？`, '确认', {
      type: 'warning',
    })
    await updateBudget(row.id, { status: 'pending' })
    ElMessage.success('提交成功')
    fetchBudgets()
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(e.message || '提交失败')
  }
}

const handleDelete = async (row: Budget) => {
  try {
    await ElMessageBox.confirm(`确认删除预算 ${row.budget_no} 吗？`, '删除确认', {
      type: 'warning',
      confirmButtonText: '确定',
      cancelButtonText: '取消',
    })

    await deleteBudget(row.id)
    ElMessage.success('删除成功')
    fetchBudgets()
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(e.message || '删除失败')
  }
}

onMounted(() => {
  fetchBudgets()
})
</script>

<style scoped>
.budget-container {
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
