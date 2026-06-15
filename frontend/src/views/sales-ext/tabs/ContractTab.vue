<!--
  ContractTab.vue - 销售合同 Tab
  来源：原 sales-ext/index.vue 中 销售合同 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="contract-tab">
    <div class="page-header">
      <h2 class="page-title">销售合同管理</h2>
      <el-button type="primary" @click="openContractDialog()">
        <el-icon><Plus /></el-icon> 新建合同
      </el-button>
    </div>
    <el-card shadow="hover">
      <el-table v-loading="contractLoading" :data="salesContracts" stripe>
        <el-table-column prop="contract_no" label="合同编号" width="140" />
        <el-table-column prop="customer_name" label="客户" min-width="150" />
        <el-table-column prop="contract_date" label="合同日期" width="120" />
        <el-table-column prop="start_date" label="开始日期" width="120" />
        <el-table-column prop="end_date" label="结束日期" width="120" />
        <el-table-column prop="total_amount" label="总金额" width="120" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.total_amount) }}
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getContractStatusType(row.status)" size="small">
              {{ getContractStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="createdBy" label="创建人" width="100" />
        <el-table-column label="操作" width="240" fixed="right">
          <template #default="{ row }">
            <el-button size="small" link @click="viewContract(row as unknown as SalesContract)"
              >查看</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              size="small"
              link
              @click="openContractDialog(row as unknown as SalesContract)"
              >编辑</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              size="small"
              link
              type="success"
              @click="approveContract(row as unknown as SalesContract)"
              >审批</el-button
            >
            <el-button
              v-if="row.status === 'pending'"
              size="small"
              link
              type="warning"
              @click="executeContract(row as unknown as SalesContract)"
              >执行</el-button
            >
            <el-button
              v-if="['draft', 'pending'].includes(row.status)"
              size="small"
              link
              type="danger"
              @click="cancelContract(row as unknown as SalesContract)"
              >取消</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog
      v-model="contractDialogVisible"
      :title="contractForm.id ? '编辑销售合同' : '新建销售合同'"
      width="800px"
    >
      <el-form
        ref="contractFormRef"
        :model="contractForm"
        :rules="contractRules"
        label-width="100px"
      >
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="合同编号" prop="contract_no">
              <el-input v-model="contractForm.contract_no" :disabled="!!contractForm.id" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="客户" prop="customer_name">
              <el-input v-model="contractForm.customer_name" placeholder="请选择客户" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="8">
            <el-form-item label="合同日期" prop="contract_date">
              <el-date-picker
                v-model="contractForm.contract_date"
                type="date"
                style="width: 100%"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="开始日期" prop="start_date">
              <el-date-picker
                v-model="contractForm.start_date"
                type="date"
                style="width: 100%"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="结束日期" prop="end_date">
              <el-date-picker
                v-model="contractForm.end_date"
                type="date"
                style="width: 100%"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="货币" prop="currency">
              <el-select v-model="contractForm.currency" placeholder="选择货币" style="width: 100%">
                <el-option label="CNY" value="CNY" />
                <el-option label="USD" value="USD" />
                <el-option label="EUR" value="EUR" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="总金额" prop="total_amount">
              <el-input-number
                v-model="contractForm.total_amount"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-divider>合同明细</el-divider>
        <el-table :data="contractForm.items" border style="width: 100%">
          <el-table-column prop="product_name" label="产品名称" min-width="150">
            <template #default="{ row }">
              <el-input v-model="row.product_name" placeholder="产品名称" />
            </template>
          </el-table-column>
          <el-table-column prop="product_code" label="产品编码" width="120">
            <template #default="{ row }">
              <el-input v-model="row.product_code" placeholder="编码" />
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
          <el-table-column label="操作" width="80">
            <template #default="{ $index }">
              <el-button size="small" link type="danger" @click="removeContractItem($index)"
                >删除</el-button
              >
            </template>
          </el-table-column>
        </el-table>
        <el-button type="primary" link style="margin-top: 8px" @click="addContractItem"
          >添加产品</el-button
        >
        <el-form-item label="付款条款" prop="payment_terms">
          <el-input v-model="contractForm.payment_terms" type="textarea" />
        </el-form-item>
        <el-form-item label="交货条款" prop="delivery_terms">
          <el-input v-model="contractForm.delivery_terms" type="textarea" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="contractDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="contractSubmitLoading" @click="submitContract"
          >确定</el-button
        >
      </template>
    </el-dialog>

    <el-dialog v-model="contractViewVisible" title="销售合同详情" width="800px">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="合同编号">{{
          currentContract?.contract_no
        }}</el-descriptions-item>
        <el-descriptions-item label="客户">{{
          currentContract?.customer_name
        }}</el-descriptions-item>
        <el-descriptions-item label="合同日期">{{
          currentContract?.contract_date
        }}</el-descriptions-item>
        <el-descriptions-item label="有效日期"
          >{{ currentContract?.start_date }} ~ {{ currentContract?.end_date }}</el-descriptions-item
        >
        <el-descriptions-item label="货币">{{ currentContract?.currency }}</el-descriptions-item>
        <el-descriptions-item label="总金额">{{
          formatMoney(currentContract?.total_amount || 0)
        }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="getContractStatusType(currentContract?.status)">
            {{ getContractStatusLabel(currentContract?.status) }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="创建人">{{
          currentContract?.created_by_name
        }}</el-descriptions-item>
      </el-descriptions>
      <el-divider>合同明细</el-divider>
      <el-table :data="currentContract?.items || []" stripe>
        <el-table-column prop="product_name" label="产品名称" min-width="150" />
        <el-table-column prop="product_code" label="产品编码" width="120" />
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
        <el-table-column prop="remark" label="备注" min-width="120" />
      </el-table>
      <el-divider>条款</el-divider>
      <el-descriptions :column="1" border>
        <el-descriptions-item label="付款条款">{{
          currentContract?.payment_terms
        }}</el-descriptions-item>
        <el-descriptions-item label="交货条款">{{
          currentContract?.delivery_terms
        }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'
import {
  listSalesContracts,
  getSalesContract,
  createSalesContract,
  updateSalesContract,
  approveSalesContract,
  executeSalesContract,
  cancelSalesContract,
  type SalesContract,
  type ContractItem as SalesContractItem,
} from '@/api/sales-contract'

const salesContracts = ref<SalesContract[]>([])
const contractLoading = ref(false)

const formatMoney = (amount: number | undefined) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const getContractStatusLabel = (status?: string) => {
  const map: Record<string, string> = {
    draft: '草稿',
    pending: '待审核',
    active: '执行中',
    completed: '已完成',
    cancelled: '已取消',
  }
  return map[status || ''] || status || ''
}

const getContractStatusType = (status?: string) => {
  const map: Record<string, string> = {
    draft: 'info',
    pending: 'warning',
    active: 'primary',
    completed: 'success',
    cancelled: 'danger',
  }
  return map[status || ''] || 'info'
}

const fetchSalesContracts = async () => {
  contractLoading.value = true
  try {
    const res = await listSalesContracts()
    salesContracts.value = res.data?.list || []
  } catch (error) {
    const err = error as { message?: string }
    ElMessage.error(err.message || '获取销售合同失败')
  } finally {
    contractLoading.value = false
  }
}

const contractDialogVisible = ref(false)
const contractFormRef = ref<FormInstance>()
const contractSubmitLoading = ref(false)
const contractForm = reactive({
  id: 0,
  contract_no: '',
  customer_id: 0,
  customer_name: '',
  contract_date: '',
  start_date: '',
  end_date: '',
  total_amount: 0,
  currency: 'CNY',
  status: 'draft' as 'draft' | 'pending' | 'active' | 'completed' | 'cancelled',
  items: [] as SalesContractItem[],
  payment_terms: '',
  delivery_terms: '',
})

const contractRules: FormRules = {
  contract_no: [{ required: true, message: '请输入合同编号', trigger: 'blur' }],
  customer_name: [{ required: true, message: '请输入客户名称', trigger: 'blur' }],
  contract_date: [{ required: true, message: '请选择合同日期', trigger: 'change' }],
  total_amount: [{ required: true, message: '请输入总金额', trigger: 'blur' }],
}

const openContractDialog = async (row?: SalesContract) => {
  if (row) {
    const res = await getSalesContract(row.id)
    Object.assign(contractForm, res.data!)
  } else {
    Object.assign(contractForm, {
      id: 0,
      contract_no: '',
      customer_id: 0,
      customer_name: '',
      contract_date: '',
      start_date: '',
      end_date: '',
      total_amount: 0,
      currency: 'CNY',
      status: 'draft',
      items: [
        {
          id: 0,
          contract_id: 0,
          product_id: 0,
          product_name: '',
          product_code: '',
          quantity: 0,
          unit: '',
          price: 0,
          amount: 0,
          remark: '',
        },
      ],
      payment_terms: '',
      delivery_terms: '',
    })
  }
  contractDialogVisible.value = true
}

const submitContract = async () => {
  const valid = await contractFormRef.value?.validate()
  if (!valid) return
  contractSubmitLoading.value = true
  try {
    if (contractForm.id) {
      await updateSalesContract(contractForm.id, contractForm)
      ElMessage.success('更新成功')
    } else {
      await createSalesContract(contractForm)
      ElMessage.success('创建成功')
    }
    contractDialogVisible.value = false
    fetchSalesContracts()
  } catch (error) {
    const err = error as { message?: string }
    ElMessage.error(err.message || '操作失败')
  } finally {
    contractSubmitLoading.value = false
  }
}

const contractViewVisible = ref(false)
const currentContract = ref<SalesContract | null>(null)

const viewContract = async (row: SalesContract) => {
  const res = await getSalesContract(row.id)
  currentContract.value = res.data!
  contractViewVisible.value = true
}

const approveContract = async (row: SalesContract) => {
  try {
    await ElMessageBox.confirm('确定审批此合同吗？', '确认', { type: 'info' })
    await approveSalesContract(row.id)
    ElMessage.success('审批成功')
    fetchSalesContracts()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as { message?: string }
      ElMessage.error(err.message || '操作失败')
    }
  }
}

const executeContract = async (row: SalesContract) => {
  try {
    await ElMessageBox.confirm('确定执行此合同吗？', '确认', { type: 'info' })
    await executeSalesContract(row.id)
    ElMessage.success('执行成功')
    fetchSalesContracts()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as { message?: string }
      ElMessage.error(err.message || '操作失败')
    }
  }
}

const cancelContract = async (row: SalesContract) => {
  try {
    await ElMessageBox.confirm('确定取消此合同吗？', '确认', { type: 'warning' })
    await cancelSalesContract(row.id)
    ElMessage.success('取消成功')
    fetchSalesContracts()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as { message?: string }
      ElMessage.error(err.message || '操作失败')
    }
  }
}

const addContractItem = () => {
  contractForm.items.push({
    id: 0,
    contract_id: 0,
    product_id: 0,
    product_name: '',
    product_code: '',
    quantity: 0,
    unit: '',
    price: 0,
    amount: 0,
    remark: '',
  })
}

const removeContractItem = (index: number) => {
  if (contractForm.items.length > 1) {
    contractForm.items.splice(index, 1)
  }
}

defineExpose({ refresh: fetchSalesContracts })

onMounted(() => {
  fetchSalesContracts()
})
</script>
