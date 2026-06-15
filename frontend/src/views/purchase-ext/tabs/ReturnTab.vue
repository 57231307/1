<!--
  ReturnTab.vue - 采购退货 Tab
  来源：原 purchase-ext/index.vue 中 采购退货 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="return-tab">
    <div class="page-header">
      <h2 class="page-title">采购退货管理</h2>
      <el-button type="primary" @click="openReturnDialog()">
        <el-icon><Plus /></el-icon> 新建退货
      </el-button>
    </div>
    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="returnQuery">
        <el-form-item label="退货单号">
          <el-input v-model="returnQuery.returnNo" placeholder="退货单号" clearable />
        </el-form-item>
        <el-form-item label="供应商">
          <el-input v-model="returnQuery.supplierName" placeholder="供应商名称" clearable />
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
          <el-button type="primary" @click="fetchPurchaseReturns">查询</el-button>
          <el-button @click="resetReturnQuery">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>
    <el-card shadow="hover">
      <el-table v-loading="returnLoading" :data="purchaseReturns" stripe>
        <el-table-column prop="returnNo" label="退货单号" width="140" />
        <el-table-column prop="supplierName" label="供应商" min-width="150" />
        <el-table-column prop="purchaseOrderNo" label="订单号" width="140" />
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
            <el-button size="small" link @click="viewReturn(row as unknown as PurchaseReturn)"
              >查看</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              size="small"
              link
              @click="openReturnDialog(row as unknown as PurchaseReturn)"
              >编辑</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 退货编辑对话框 -->
    <el-dialog
      v-model="returnDialogVisible"
      :title="returnForm.id ? '编辑采购退货' : '新建采购退货'"
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
            <el-form-item label="供应商" prop="supplierName">
              <el-input v-model="returnForm.supplierName" placeholder="供应商名称" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="关联订单号" prop="purchaseOrderNo">
              <el-input v-model="returnForm.purchaseOrderNo" placeholder="订单号" />
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
          <el-table-column prop="unit" label="单位" width="80">
            <template #default="{ row }">
              <el-input v-model="row.unit" placeholder="单位" />
            </template>
          </el-table-column>
          <el-table-column prop="price" label="单价" width="100">
            <template #default="{ row }">
              <el-input-number v-model="row.price" :min="0" :precision="2" style="width: 100%" />
            </template>
          </el-table-column>
          <el-table-column prop="amount" label="金额" width="100">
            <template #default="{ row }">
              {{ formatMoney((row.quantity || 0) * (row.price || 0)) }}
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
    <el-dialog v-model="returnViewVisible" title="采购退货详情" width="800px">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="退货单号">{{ currentReturn?.returnNo }}</el-descriptions-item>
        <el-descriptions-item label="供应商">{{
          currentReturn?.supplierName
        }}</el-descriptions-item>
        <el-descriptions-item label="关联订单">{{
          currentReturn?.purchaseOrderNo
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
import {
  purchaseReturnApi,
  type PurchaseReturn,
  type PurchaseReturnItem,
} from '@/api/purchase-return'

const purchaseReturns = ref<PurchaseReturn[]>([])
const returnLoading = ref(false)

const returnQuery = reactive({
  returnNo: '',
  supplierName: '',
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

const fetchPurchaseReturns = async () => {
  returnLoading.value = true
  try {
    const res = await purchaseReturnApi.list(returnQuery)
    purchaseReturns.value = res.data?.list || []
  } catch (error) {
    const err = error as { message?: string }
    ElMessage.error(err.message || '获取采购退货失败')
  } finally {
    returnLoading.value = false
  }
}

const resetReturnQuery = () => {
  returnQuery.returnNo = ''
  returnQuery.supplierName = ''
  returnQuery.status = ''
  fetchPurchaseReturns()
}

const returnDialogVisible = ref(false)
const returnFormRef = ref<FormInstance>()
const returnSubmitLoading = ref(false)
const returnForm = reactive({
  id: 0,
  returnNo: '',
  supplierId: 0,
  supplierName: '',
  orderId: 0,
  purchaseOrderNo: '',
  returnDate: '',
  totalAmount: 0,
  reason: '',
  status: 'draft' as 'draft' | 'pending' | 'approved' | 'rejected' | 'completed',
  items: [] as PurchaseReturnItem[],
})

const returnRules: FormRules = {
  returnNo: [{ required: true, message: '请输入退货单号', trigger: 'blur' }],
  supplierName: [{ required: true, message: '请输入供应商名称', trigger: 'blur' }],
  returnDate: [{ required: true, message: '请选择退货日期', trigger: 'change' }],
  reason: [{ required: true, message: '请输入退货原因', trigger: 'blur' }],
}

const openReturnDialog = async (row?: PurchaseReturn) => {
  if (row) {
    const res = await purchaseReturnApi.getById(row.id!)
    Object.assign(returnForm, res.data!)
  } else {
    Object.assign(returnForm, {
      id: 0,
      returnNo: '',
      supplierId: 0,
      supplierName: '',
      orderId: 0,
      purchaseOrderNo: '',
      returnDate: '',
      totalAmount: 0,
      reason: '',
      status: 'draft',
      items: [
        {
          id: 0,
          returnId: 0,
          productId: 0,
          productName: '',
          productCode: '',
          quantity: 0,
          unit: '',
          price: 0,
          amount: 0,
          reason: '',
        },
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
      await purchaseReturnApi.update(returnForm.id, returnForm)
      ElMessage.success('更新成功')
    } else {
      await purchaseReturnApi.create(returnForm)
      ElMessage.success('创建成功')
    }
    returnDialogVisible.value = false
    fetchPurchaseReturns()
  } catch (error) {
    const err = error as { message?: string }
    ElMessage.error(err.message || '操作失败')
  } finally {
    returnSubmitLoading.value = false
  }
}

const returnViewVisible = ref(false)
const currentReturn = ref<PurchaseReturn | null>(null)

const viewReturn = async (row: PurchaseReturn) => {
  const res = await purchaseReturnApi.getById(row.id!)
  currentReturn.value = res.data!
  returnViewVisible.value = true
}

const addReturnItem = () => {
  returnForm.items.push({
    id: 0,
    returnId: 0,
    productId: 0,
    productName: '',
    productCode: '',
    quantity: 0,
    unitPrice: 0,
    reason: '',
  } as PurchaseReturnItem)
}

const removeReturnItem = (index: number) => {
  if (returnForm.items.length > 1) {
    returnForm.items.splice(index, 1)
  }
}

defineExpose({ refresh: fetchPurchaseReturns })

onMounted(() => {
  fetchPurchaseReturns()
})
</script>
