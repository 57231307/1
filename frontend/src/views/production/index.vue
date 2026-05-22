<template>
  <div class="production-container">
    <el-card class="header-card">
      <div class="header-content">
        <h2>生产计划管理 (MRP)</h2>
        <p>管理和跟踪生产订单，制定和执行生产计划</p>
      </div>
    </el-card>

    <!-- 筛选区 -->
    <el-card class="filter-card">
      <el-form :inline="true" :model="queryForm">
        <el-form-item label="订单编号">
          <el-input v-model="queryForm.order_no" placeholder="请输入订单编号" clearable />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="queryForm.status" placeholder="请选择状态" clearable>
            <el-option
              v-for="(item, key) in PRODUCTION_ORDER_STATUS"
              :key="key"
              :label="item.label"
              :value="key"
            />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="fetchOrders">查询</el-button>
          <el-button @click="resetQuery">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- 操作区 -->
    <el-card class="table-card">
      <template #header>
        <div class="card-header">
          <span>生产订单列表</span>
          <el-button type="primary" @click="openDialog('create')">
            <el-icon><Plus /></el-icon>新建订单
          </el-button>
        </div>
      </template>
      
      <el-table
        :data="orderList"
        v-loading="loading"
        stripe
        border
      >
        <el-table-column prop="order_no" label="订单编号" width="160" />
        <el-table-column prop="product_name" label="产品名称" min-width="160" />
        <el-table-column prop="planned_quantity" label="计划数量" width="120" />
        <el-table-column prop="actual_quantity" label="实际数量" width="120" />
        <el-table-column prop="scheduled_start_date" label="计划开始" width="140">
          <template #default="{ row }">
            {{ row.scheduled_start_date ? row.scheduled_start_date.substring(0, 10) : '-' }}
          </template>
        </el-table-column>
        <el-table-column prop="scheduled_end_date" label="计划结束" width="140">
          <template #default="{ row }">
            {{ row.scheduled_end_date ? row.scheduled_end_date.substring(0, 10) : '-' }}
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="120">
          <template #default="{ row }">
            <el-tag :type="PRODUCTION_ORDER_STATUS[row.status as keyof typeof PRODUCTION_ORDER_STATUS]?.type || 'info'">
              {{ PRODUCTION_ORDER_STATUS[row.status as keyof typeof PRODUCTION_ORDER_STATUS]?.label || row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="priority" label="优先级" width="100" />
        <el-table-column label="操作" width="280" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="viewDetail(row)">查看</el-button>
            <el-button type="success" link size="small" @click="openDialog('edit', row)" v-if="row.status === 'draft'">编辑</el-button>
            <el-button type="warning" link size="small" @click="handleStatusChange(row, 'planned')" v-if="row.status === 'draft'">计划</el-button>
            <el-button type="primary" link size="small" @click="handleStatusChange(row, 'in_production')" v-if="row.status === 'planned'">开始生产</el-button>
            <el-button type="success" link size="small" @click="handleStatusChange(row, 'completed')" v-if="row.status === 'in_production'">完成</el-button>
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
          @size-change="fetchOrders"
          @current-change="fetchOrders"
        />
      </div>
    </el-card>

    <!-- 新建/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="dialogType === 'create' ? '新建生产订单' : '编辑生产订单'"
      width="600px"
      @close="resetForm"
    >
      <el-form :model="orderForm" :rules="orderRules" ref="orderFormRef" label-width="120px">
        <el-form-item label="订单编号" prop="order_no">
          <el-input v-model="orderForm.order_no" placeholder="请输入订单编号" />
        </el-form-item>
        <el-form-item label="产品ID" prop="product_id">
          <el-input-number v-model="orderForm.product_id" :min="1" style="width: 100%" />
        </el-form-item>
        <el-form-item label="计划数量" prop="planned_quantity">
          <el-input-number v-model="orderForm.planned_quantity" :min="1" style="width: 100%" />
        </el-form-item>
        <el-form-item label="计划开始">
          <el-date-picker v-model="orderForm.scheduled_start_date" type="date" placeholder="请选择日期" style="width: 100%" value-format="YYYY-MM-DD" />
        </el-form-item>
        <el-form-item label="计划结束">
          <el-date-picker v-model="orderForm.scheduled_end_date" type="date" placeholder="请选择日期" style="width: 100%" value-format="YYYY-MM-DD" />
        </el-form-item>
        <el-form-item label="优先级" prop="priority">
          <el-select v-model="orderForm.priority" placeholder="请选择优先级" style="width: 100%">
            <el-option :label="1" :value="1" />
            <el-option :label="2" :value="2" />
            <el-option :label="3" :value="3" />
            <el-option :label="4" :value="4" />
            <el-option :label="5" :value="5" />
          </el-select>
        </el-form-item>
        <el-form-item label="工作中心">
          <el-input-number v-model="orderForm.work_center_id" :min="1" style="width: 100%" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="orderForm.remark" type="textarea" :rows="3" placeholder="请输入备注" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">确认</el-button>
      </template>
    </el-dialog>

    <!-- 详情对话框 -->
    <el-dialog v-model="detailVisible" title="生产订单详情" width="600px">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="订单编号">{{ currentOrder?.order_no }}</el-descriptions-item>
        <el-descriptions-item label="产品ID">{{ currentOrder?.product_id }}</el-descriptions-item>
        <el-descriptions-item label="计划数量">{{ currentOrder?.planned_quantity }}</el-descriptions-item>
        <el-descriptions-item label="实际数量">{{ currentOrder?.actual_quantity || '-' }}</el-descriptions-item>
        <el-descriptions-item label="计划开始">{{ currentOrder?.scheduled_start_date || '-' }}</el-descriptions-item>
        <el-descriptions-item label="计划结束">{{ currentOrder?.scheduled_end_date || '-' }}</el-descriptions-item>
        <el-descriptions-item label="实际开始">{{ currentOrder?.actual_start_date || '-' }}</el-descriptions-item>
        <el-descriptions-item label="实际结束">{{ currentOrder?.actual_end_date || '-' }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="PRODUCTION_ORDER_STATUS[currentOrder?.status as keyof typeof PRODUCTION_ORDER_STATUS]?.type">
            {{ PRODUCTION_ORDER_STATUS[currentOrder?.status as keyof typeof PRODUCTION_ORDER_STATUS]?.label }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="优先级">{{ currentOrder?.priority }}</el-descriptions-item>
        <el-descriptions-item label="工作中心">{{ currentOrder?.work_center_id || '-' }}</el-descriptions-item>
        <el-descriptions-item label="创建时间">{{ currentOrder?.created_at }}</el-descriptions-item>
        <el-descriptions-item label="备注" :span="2">{{ currentOrder?.remark || '-' }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  listProductionOrders,
  createProductionOrder,
  updateProductionOrder,
  deleteProductionOrder,
  updateProductionOrderStatus,
  type ProductionOrder,
  PRODUCTION_ORDER_STATUS,
} from '@/api/production'

// 响应式数据
const loading = ref(false)
const submitLoading = ref(false)
const dialogVisible = ref(false)
const detailVisible = ref(false)
const dialogType = ref<'create' | 'edit'>('create')
const orderList = ref<ProductionOrder[]>([])
const currentOrder = ref<ProductionOrder | null>(null)
const orderFormRef = ref<FormInstance>()
const total = ref(0)

// 查询表单
const queryForm = reactive({
  page: 1,
  page_size: 20,
  order_no: '',
  status: '',
})

// 订单表单
const orderForm = reactive<Partial<ProductionOrder>>({
  order_no: '',
  product_id: undefined,
  planned_quantity: undefined,
  scheduled_start_date: '',
  scheduled_end_date: '',
  status: 'draft',
  priority: 5,
  work_center_id: undefined,
  remark: '',
})

// 表单验证规则
const orderRules: FormRules = {
  order_no: [{ required: true, message: '请输入订单编号', trigger: 'blur' }],
  product_id: [{ required: true, message: '请输入产品ID', trigger: 'blur' }],
  planned_quantity: [{ required: true, message: '请输入计划数量', trigger: 'blur' }],
  priority: [{ required: true, message: '请选择优先级', trigger: 'change' }],
}

// 获取生产订单列表
const fetchOrders = async () => {
  loading.value = true
  try {
    const res = await listProductionOrders(queryForm)
    orderList.value = res.data!.list || []
    total.value = res.data?.total || 0
  } catch (e: any) {
    ElMessage.error(e.message || '获取订单列表失败')
  } finally {
    loading.value = false
  }
}

// 重置查询
const resetQuery = () => {
  queryForm.page = 1
  queryForm.order_no = ''
  queryForm.status = ''
  fetchOrders()
}

// 打开对话框
const openDialog = (type: 'create' | 'edit', row?: ProductionOrder) => {
  dialogType.value = type
  resetForm()
  
  if (type === 'edit' && row) {
    Object.assign(orderForm, row)
  }
  
  dialogVisible.value = true
}

// 重置表单
const resetForm = () => {
  Object.assign(orderForm, {
    id: undefined,
    order_no: '',
    product_id: undefined,
    planned_quantity: undefined,
    scheduled_start_date: '',
    scheduled_end_date: '',
    status: 'draft',
    priority: 5,
    work_center_id: undefined,
    remark: '',
  })
  orderFormRef.value?.clearValidate()
}

// 提交表单
const handleSubmit = async () => {
  if (!orderFormRef.value) return
  
  await orderFormRef.value.validate(async (valid) => {
    if (!valid) return
    
    submitLoading.value = true
    try {
      if (dialogType.value === 'create') {
        await createProductionOrder(orderForm)
        ElMessage.success('创建成功')
      } else {
        if (orderForm.id) {
          await updateProductionOrder(orderForm.id, orderForm)
          ElMessage.success('更新成功')
        }
      }
      
      dialogVisible.value = false
      fetchOrders()
    } catch (e: any) {
      ElMessage.error(e.message || '操作失败')
    } finally {
      submitLoading.value = false
    }
  })
}

// 查看详情
const viewDetail = (row: ProductionOrder) => {
  currentOrder.value = row
  detailVisible.value = true
}

// 状态变更
const handleStatusChange = async (row: ProductionOrder, status: string) => {
  try {
    await ElMessageBox.confirm(`确认将订单 ${row.order_no} 状态更改为 ${PRODUCTION_ORDER_STATUS[status as keyof typeof PRODUCTION_ORDER_STATUS]?.label} 吗？`, '确认', {
      type: 'warning',
    })
    
    await updateProductionOrderStatus(row.id, status)
    ElMessage.success('状态更新成功')
    fetchOrders()
  } catch (e: any) {
    if (e !== 'cancel') {
      ElMessage.error(e.message || '状态更新失败')
    }
  }
}

// 删除订单
const handleDelete = async (row: ProductionOrder) => {
  try {
    await ElMessageBox.confirm(`确认删除订单 ${row.order_no} 吗？此操作不可恢复。`, '删除确认', {
      type: 'warning',
      confirmButtonText: '确定',
      cancelButtonText: '取消',
    })
    
    await deleteProductionOrder(row.id)
    ElMessage.success('删除成功')
    fetchOrders()
  } catch (e: any) {
    if (e !== 'cancel') {
      ElMessage.error(e.message || '删除失败')
    }
  }
}

// 组件挂载时获取数据
onMounted(() => {
  fetchOrders()
})
</script>

<style scoped>
.production-container {
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
