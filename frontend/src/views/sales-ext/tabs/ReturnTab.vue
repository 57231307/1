<!--
  ReturnTab.vue - 销售退货 Tab
  来源：原 sales-ext/index.vue 中 销售退货 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="return-tab">
    <div class="page-header">
      <h2 class="page-title">销售退货管理</h2>
      <el-button type="primary" @click="openReturnDialog()">
        <el-icon><Plus /></el-icon> 新建退货
      </el-button>
    </div>
    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="returnQuery">
        <el-form-item label="退货单号">
          <el-input v-model="returnQuery.returnNo" placeholder="退货单号" clearable />
        </el-form-item>
        <el-form-item label="客户">
          <el-input v-model="returnQuery.customerName" placeholder="客户名称" clearable />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="returnQuery.status" placeholder="选择状态" clearable>
            <el-option label="草稿" value="draft" />
            <el-option label="待审核" value="pending" />
            <el-option label="已批准" value="approved" />
            <el-option label="已拒绝" value="rejected" />
            <el-option label="已完成" value="completed" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="fetchSalesReturns">查询</el-button>
          <el-button @click="resetReturnQuery">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>
    <el-card shadow="hover">
      <el-table v-loading="returnLoading" :data="salesReturns" stripe>
        <el-table-column prop="returnNo" label="退货单号" width="140" />
        <el-table-column prop="customerName" label="客户" min-width="150" />
        <el-table-column prop="salesOrderNo" label="订单号" width="140" />
        <el-table-column prop="returnDate" label="退货日期" width="120" />
        <el-table-column prop="totalAmount" label="总金额" width="120" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.totalAmount) }}
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getReturnStatusType(row.status)" size="small">
              {{ getReturnStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="createdBy" label="创建人" width="100" />
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button size="small" link @click="viewReturn(row as unknown as SalesReturn)"
              >查看</el-button
            >
            <!-- P2-17 修复（批次 86 v2 复审）：编辑按钮补齐 v-permission -->
            <el-button
              v-if="row.status === 'draft'"
              v-permission="'sales_return:update'"
              size="small"
              link
              @click="openReturnDialog(row as unknown as SalesReturn)"
              >编辑</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 扩展指令（批次 86）：补全退货编辑对话框，替换原占位符 -->
    <el-dialog
      v-model="returnDialogVisible"
      :title="returnForm.id ? '编辑销售退货' : '新建销售退货'"
      width="800px"
    >
      <el-form ref="returnFormRef" :model="returnForm" :rules="returnRules" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="退货单号" prop="returnNo">
              <el-input v-model="returnForm.returnNo" :disabled="!!returnForm.id" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="客户" prop="customerName">
              <el-input v-model="returnForm.customerName" placeholder="客户名称" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="关联订单号" prop="salesOrderNo">
              <el-input v-model="returnForm.salesOrderNo" placeholder="销售订单号" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="退货日期" prop="returnDate">
              <el-date-picker
                v-model="returnForm.returnDate"
                type="date"
                style="width: 100%"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="退货原因" prop="reason">
          <el-input v-model="returnForm.reason" type="textarea" />
        </el-form-item>
        <el-divider>退货明细</el-divider>
        <el-table :data="returnForm.items" border style="width: 100%">
          <el-table-column prop="productName" label="产品名称" min-width="150">
            <template #default="{ row }">
              <el-input v-model="row.productName" placeholder="产品名称" />
            </template>
          </el-table-column>
          <el-table-column prop="productCode" label="产品编码" width="120">
            <template #default="{ row }">
              <el-input v-model="row.productCode" placeholder="编码" />
            </template>
          </el-table-column>
          <el-table-column prop="quantity" label="数量" width="100">
            <template #default="{ row }">
              <el-input-number v-model="row.quantity" :min="0" style="width: 100%" />
            </template>
          </el-table-column>
          <el-table-column prop="unitPrice" label="单价" width="100">
            <template #default="{ row }">
              <el-input-number v-model="row.unitPrice" :min="0" :precision="2" style="width: 100%" />
            </template>
          </el-table-column>
          <el-table-column label="金额" width="100">
            <template #default="{ row }">
              {{ formatMoney((row.quantity || 0) * (row.unitPrice || 0)) }}
            </template>
          </el-table-column>
          <el-table-column prop="reason" label="退货原因" min-width="120">
            <template #default="{ row }">
              <el-input v-model="row.reason" placeholder="退货原因" />
            </template>
          </el-table-column>
          <el-table-column label="操作" width="80">
            <template #default="{ $index }">
              <el-button size="small" link type="danger" @click="removeReturnItem($index)"
                >删除</el-button
              >
            </template>
          </el-table-column>
        </el-table>
        <el-button type="primary" link style="margin-top: 8px" @click="addReturnItem"
          >添加产品</el-button
        >
      </el-form>
      <template #footer>
        <el-button @click="returnDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="returnSubmitLoading" @click="submitReturn"
          >确定</el-button
        >
      </template>
    </el-dialog>

    <!-- 退货详情对话框 -->
    <el-dialog v-model="returnViewVisible" title="销售退货详情" width="800px">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="退货单号">{{ currentReturn?.returnNo }}</el-descriptions-item>
        <el-descriptions-item label="客户">{{ currentReturn?.customerName }}</el-descriptions-item>
        <el-descriptions-item label="关联订单">{{
          currentReturn?.salesOrderNo
        }}</el-descriptions-item>
        <el-descriptions-item label="退货日期">{{
          currentReturn?.returnDate
        }}</el-descriptions-item>
        <el-descriptions-item label="总金额">{{
          formatMoney(currentReturn?.totalAmount || 0)
        }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="getReturnStatusType(currentReturn?.status)">
            {{ getReturnStatusLabel(currentReturn?.status) }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="创建人">{{ currentReturn?.createdBy }}</el-descriptions-item>
        <el-descriptions-item label="审批人">{{ currentReturn?.approved_by }}</el-descriptions-item>
      </el-descriptions>
      <el-divider>退货原因</el-divider>
      <p>{{ currentReturn?.reason }}</p>
      <el-divider>退货明细</el-divider>
      <el-table :data="currentReturn?.items || []" stripe>
        <el-table-column prop="productName" label="产品名称" min-width="150" />
        <el-table-column prop="productCode" label="产品编码" width="120" />
        <el-table-column prop="quantity" label="数量" width="100" align="right" />
        <el-table-column prop="unit" label="单位" width="80" />
        <el-table-column prop="price" label="单价" width="100" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.price) }}
          </template>
        </el-table-column>
        <el-table-column prop="amount" label="金额" width="100" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.amount) }}
          </template>
        </el-table-column>
        <el-table-column prop="reason" label="退货原因" min-width="120" />
      </el-table>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'
import { salesReturnApi, type SalesReturn, type SalesReturnItem } from '@/api/sales-return'

const salesReturns = ref<SalesReturn[]>([])
const returnLoading = ref(false)

const returnQuery = reactive({
  returnNo: '',
  customerName: '',
  status: '',
})

const formatMoney = (amount: number | undefined) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const getReturnStatusLabel = (status?: string) => {
  const map: Record<string, string> = {
    draft: '草稿',
    pending: '待审核',
    approved: '已批准',
    rejected: '已拒绝',
    completed: '已完成',
  }
  return map[status || ''] || status || ''
}

const getReturnStatusType = (status?: string) => {
  const map: Record<string, string> = {
    draft: 'info',
    pending: 'warning',
    approved: 'success',
    rejected: 'danger',
    completed: 'success',
  }
  return map[status || ''] || 'info'
}

const fetchSalesReturns = async () => {
  returnLoading.value = true
  try {
    const res = await salesReturnApi.list(returnQuery)
    const d = res.data as
      | { list?: SalesReturn[]; items?: SalesReturn[]; data?: SalesReturn[] }
      | SalesReturn[]
      | undefined
    if (d && typeof d === 'object' && !Array.isArray(d)) {
      salesReturns.value = d.list || d.items || d.data || []
    } else {
      salesReturns.value = (d as SalesReturn[]) || []
    }
  } catch (error) {
    const err = error as { message?: string }
    ElMessage.error(err.message || '获取销售退货失败')
  } finally {
    returnLoading.value = false
  }
}

const resetReturnQuery = () => {
  returnQuery.returnNo = ''
  returnQuery.customerName = ''
  returnQuery.status = ''
  fetchSalesReturns()
}

// 扩展指令（批次 86）：补全退货编辑表单状态与提交逻辑，替换原占位符
const returnDialogVisible = ref(false)
const returnFormRef = ref<FormInstance>()
const returnSubmitLoading = ref(false)
const returnForm = reactive({
  id: 0,
  returnNo: '',
  customerId: 0,
  customerName: '',
  salesOrderId: 0,
  salesOrderNo: '',
  returnDate: '',
  reason: '',
  status: 'draft',
  items: [] as SalesReturnItem[],
})

const returnRules: FormRules = {
  returnNo: [{ required: true, message: '请输入退货单号', trigger: 'blur' }],
  customerName: [{ required: true, message: '请输入客户名称', trigger: 'blur' }],
  returnDate: [{ required: true, message: '请选择退货日期', trigger: 'change' }],
  reason: [{ required: true, message: '请输入退货原因', trigger: 'blur' }],
}

const openReturnDialog = async (row?: SalesReturn) => {
  if (row) {
    const res = await salesReturnApi.getById(row.id!)
    // 安全检查：防止后端返回 data 为 null 时崩溃
    if (res.data) Object.assign(returnForm, res.data)
  } else {
    Object.assign(returnForm, {
      id: 0,
      returnNo: '',
      customerId: 0,
      customerName: '',
      salesOrderId: 0,
      salesOrderNo: '',
      returnDate: '',
      reason: '',
      status: 'draft',
      items: [
        {
          productId: 0,
          productName: '',
          productCode: '',
          quantity: 0,
          unitPrice: 0,
          reason: '',
        } as SalesReturnItem,
      ],
    })
  }
  returnDialogVisible.value = true
}

const submitReturn = async () => {
  const valid = await returnFormRef.value?.validate()
  if (!valid) return

  returnSubmitLoading.value = true
  try {
    if (returnForm.id) {
      await salesReturnApi.update(returnForm.id, returnForm)
      ElMessage.success('更新成功')
    } else {
      await salesReturnApi.create(returnForm)
      ElMessage.success('创建成功')
    }
    returnDialogVisible.value = false
    fetchSalesReturns()
  } catch (error) {
    const err = error as { message?: string }
    ElMessage.error(err.message || '操作失败')
  } finally {
    returnSubmitLoading.value = false
  }
}

const addReturnItem = () => {
  returnForm.items.push({
    productId: 0,
    productName: '',
    productCode: '',
    quantity: 0,
    unitPrice: 0,
    reason: '',
  } as SalesReturnItem)
}

const removeReturnItem = (index: number) => {
  if (returnForm.items.length > 1) {
    returnForm.items.splice(index, 1)
  }
}

const returnViewVisible = ref(false)
const currentReturn = ref<SalesReturn | null>(null)

const viewReturn = async (row: SalesReturn) => {
  try {
    const res = await salesReturnApi.getById(row.id!)
    currentReturn.value = res.data || row
    returnViewVisible.value = true
  } catch (_e) {
    currentReturn.value = row
    returnViewVisible.value = true
  }
}

defineExpose({ refresh: fetchSalesReturns })

onMounted(() => {
  fetchSalesReturns()
})
</script>
