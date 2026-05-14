<template>
  <div class="customer-credit">
    <el-card>
      <template #header>
        <div class="card-header">
          <span>客户信用管理</span>
        </div>
      </template>
      
      <div class="toolbar">
        <el-button type="primary" @click="handleSetRating">设置信用评级</el-button>
      </div>
      
      <el-table :data="creditList" border stripe>
        <el-table-column prop="customerName" label="客户名称" />
        <el-table-column prop="creditLevel" label="信用等级">
          <template #default="{ row }">
            <el-tag v-if="row.creditLevel === 'AAA'" type="success">AAA</el-tag>
            <el-tag v-else-if="row.creditLevel === 'AA'" type="success">AA</el-tag>
            <el-tag v-else-if="row.creditLevel === 'A'" type="success">A</el-tag>
            <el-tag v-else-if="row.creditLevel === 'BBB'" type="warning">BBB</el-tag>
            <el-tag v-else-if="row.creditLevel === 'BB'" type="warning">BB</el-tag>
            <el-tag v-else-if="row.creditLevel === 'B'" type="warning">B</el-tag>
            <el-tag v-else type="danger">{{ row.creditLevel || '-' }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="creditScore" label="信用分" />
        <el-table-column prop="creditLimit" label="信用额度" />
        <el-table-column prop="creditDays" label="账期(天)" />
        <el-table-column prop="usedAmount" label="已用额度" />
        <el-table-column prop="availableAmount" label="可用额度">
          <template #default="{ row }">
            <span :style="{ color: row.availableAmount && row.availableAmount > 0 ? '#67c23a' : '#f56c6c' }">
              {{ row.availableAmount }}
            </span>
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态">
          <template #default="{ row }">
            <el-tag v-if="row.status === 'active'" type="success">正常</el-tag>
            <el-tag v-else type="danger">停用</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" fixed="right" width="300">
          <template #default="{ row }">
            <el-button link type="primary" @click="handleAdjustLimit(row)">调整额度</el-button>
            <el-button link type="primary" @click="handleOccupy(row)">占用额度</el-button>
            <el-button link type="primary" @click="handleRelease(row)">释放额度</el-button>
            <el-button link type="danger" @click="handleDeactivate(row)" v-if="row.status === 'active'">停用</el-button>
          </template>
        </el-table-column>
      </el-table>
      
      <el-pagination
        v-model:current-page="pagination.page"
        v-model:page-size="pagination.page_size"
        :total="pagination.total"
        layout="total, prev, pager, next, jumper"
        @current-change="fetchCredits"
      />
    </el-card>
    
    <!-- 设置评级对话框 -->
    <el-dialog v-model="ratingDialogVisible" title="设置信用评级" width="500px">
      <el-form :model="ratingForm" :rules="ratingRules" ref="ratingFormRef" label-width="120px">
        <el-form-item label="客户" prop="customerId">
          <el-select v-model="ratingForm.customerId" placeholder="请选择客户" style="width: 100%">
            <el-option label="客户A" :value="1" />
            <el-option label="客户B" :value="2" />
            <el-option label="客户C" :value="3" />
          </el-select>
        </el-form-item>
        <el-form-item label="信用等级" prop="creditLevel">
          <el-select v-model="ratingForm.creditLevel" placeholder="请选择信用等级" style="width: 100%">
            <el-option label="AAA" value="AAA" />
            <el-option label="AA" value="AA" />
            <el-option label="A" value="A" />
            <el-option label="BBB" value="BBB" />
            <el-option label="BB" value="BB" />
            <el-option label="B" value="B" />
            <el-option label="C" value="C" />
            <el-option label="D" value="D" />
          </el-select>
        </el-form-item>
        <el-form-item label="信用分" prop="creditScore">
          <el-input-number v-model="ratingForm.creditScore" :min="0" :max="100" style="width: 100%" />
        </el-form-item>
        <el-form-item label="信用额度" prop="creditLimit">
          <el-input-number v-model="ratingForm.creditLimit" :min="0" style="width: 100%" />
        </el-form-item>
        <el-form-item label="账期(天)" prop="creditDays">
          <el-input-number v-model="ratingForm.creditDays" :min="0" style="width: 100%" />
        </el-form-item>
        <el-form-item label="备注" prop="remark">
          <el-input v-model="ratingForm.remark" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>
      
      <template #footer>
        <el-button @click="ratingDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSaveRating" :loading="submitLoading">保存</el-button>
      </template>
    </el-dialog>
    
    <!-- 调整额度对话框 -->
    <el-dialog v-model="adjustDialogVisible" title="调整信用额度" width="500px">
      <el-form :model="adjustForm" :rules="adjustRules" ref="adjustFormRef" label-width="120px">
        <el-form-item label="调整类型" prop="adjustmentType">
          <el-radio-group v-model="adjustForm.adjustmentType">
            <el-radio value="increase">增加额度</el-radio>
            <el-radio value="decrease">减少额度</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="调整金额" prop="amount">
          <el-input-number v-model="adjustForm.amount" :min="0" style="width: 100%" />
        </el-form-item>
        <el-form-item label="调整原因" prop="reason">
          <el-input v-model="adjustForm.reason" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>
      
      <template #footer>
        <el-button @click="adjustDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSaveAdjust" :loading="submitLoading">确认</el-button>
      </template>
    </el-dialog>
    
    <!-- 占用/释放额度对话框 -->
    <el-dialog v-model="amountDialogVisible" :title="amountOperationType === 'occupy' ? '占用额度' : '释放额度'" width="500px">
      <el-form :model="amountForm" :rules="amountRules" ref="amountFormRef" label-width="120px">
        <el-form-item label="金额" prop="amount">
          <el-input-number v-model="amountForm.amount" :min="0" style="width: 100%" />
        </el-form-item>
      </el-form>
      
      <template #footer>
        <el-button @click="amountDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSaveAmount" :loading="submitLoading">确认</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  listCredits,
  setCreditRating,
  adjustCreditLimit,
  occupyCredit,
  releaseCredit,
  deactivateCredit,
  type CustomerCredit
} from '@/api/customer-credit'

const creditList = ref<CustomerCredit[]>([])

const pagination = reactive({
  page: 1,
  page_size: 20,
  total: 0
})

const ratingDialogVisible = ref(false)
const adjustDialogVisible = ref(false)
const amountDialogVisible = ref(false)
const submitLoading = ref(false)
const amountOperationType = ref<'occupy' | 'release'>('occupy')
const currentCustomerId = ref<number | null>(null)

const ratingFormRef = ref()
const adjustFormRef = ref()
const amountFormRef = ref()

const ratingForm = reactive({
  customerId: undefined as number | undefined,
  creditLevel: '',
  creditScore: 0,
  creditLimit: 0,
  creditDays: 0,
  remark: ''
})

const adjustForm = reactive({
  adjustmentType: 'increase',
  amount: 0,
  reason: ''
})

const amountForm = reactive({
  amount: 0
})

const ratingRules = {
  customerId: [{ required: true, message: '请选择客户', trigger: 'change' }],
  creditLevel: [{ required: true, message: '请选择信用等级', trigger: 'change' }],
  creditScore: [{ required: true, message: '请输入信用分', trigger: 'blur' }],
  creditLimit: [{ required: true, message: '请输入信用额度', trigger: 'blur' }],
  creditDays: [{ required: true, message: '请输入账期', trigger: 'blur' }]
}

const adjustRules = {
  adjustmentType: [{ required: true, message: '请选择调整类型', trigger: 'change' }],
  amount: [{ required: true, message: '请输入调整金额', trigger: 'blur' }],
  reason: [{ required: true, message: '请输入调整原因', trigger: 'blur' }]
}

const amountRules = {
  amount: [{ required: true, message: '请输入金额', trigger: 'blur' }]
}

const fetchCredits = async () => {
  try {
    const res: any = await listCredits({
      page: pagination.page,
      page_size: pagination.page_size
    } as any)
    if (res.data) {
      creditList.value = res.data.list || res.data || []
      pagination.total = res.data.total || res.data?.length || 0
    }
  } catch (e) {
    ElMessage.error('获取信用列表失败')
  }
}

const handleSetRating = () => {
  Object.assign(ratingForm, { customerId: undefined, creditLevel: '', creditScore: 0, creditLimit: 0, creditDays: 0, remark: '' })
  ratingDialogVisible.value = true
}

const handleSaveRating = async () => {
  if (!ratingFormRef.value) return
  
  await ratingFormRef.value.validate(async (valid: boolean) => {
    if (!valid) return
    
    submitLoading.value = true
    try {
      await setCreditRating(ratingForm as any)
      ElMessage.success('设置成功')
      ratingDialogVisible.value = false
      fetchCredits()
    } catch (e: any) {
      ElMessage.error(e.message || '设置失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const handleAdjustLimit = (row: CustomerCredit) => {
  if (!row.customerId) return
  currentCustomerId.value = row.customerId
  Object.assign(adjustForm, { adjustmentType: 'increase', amount: 0, reason: '' })
  adjustDialogVisible.value = true
}

const handleSaveAdjust = async () => {
  if (!adjustFormRef.value || !currentCustomerId.value) return
  
  await adjustFormRef.value.validate(async (valid: boolean) => {
    if (!valid) return
    
    submitLoading.value = true
    try {
      await adjustCreditLimit(currentCustomerId.value!, adjustForm as any)
      ElMessage.success('调整成功')
      adjustDialogVisible.value = false
      fetchCredits()
    } catch (e: any) {
      ElMessage.error(e.message || '调整失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const handleOccupy = (row: CustomerCredit) => {
  if (!row.customerId) return
  currentCustomerId.value = row.customerId
  amountOperationType.value = 'occupy'
  Object.assign(amountForm, { amount: 0 })
  amountDialogVisible.value = true
}

const handleRelease = (row: CustomerCredit) => {
  if (!row.customerId) return
  currentCustomerId.value = row.customerId
  amountOperationType.value = 'release'
  Object.assign(amountForm, { amount: 0 })
  amountDialogVisible.value = true
}

const handleSaveAmount = async () => {
  if (!amountFormRef.value || !currentCustomerId.value) return
  
  await amountFormRef.value.validate(async (valid: boolean) => {
    if (!valid) return
    
    submitLoading.value = true
    try {
      const action = amountOperationType.value === 'occupy' ? occupyCredit : releaseCredit
      await action(currentCustomerId.value!, amountForm as any)
      ElMessage.success('操作成功')
      amountDialogVisible.value = false
      fetchCredits()
    } catch (e: any) {
      ElMessage.error(e.message || '操作失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const handleDeactivate = async (row: CustomerCredit) => {
  if (!row.customerId) return
  
  try {
    await ElMessageBox.confirm('确认停用该客户信用？', '提示', {
      confirmButtonText: '确认',
      cancelButtonText: '取消',
      type: 'warning'
    })
    
    await deactivateCredit(row.customerId)
    ElMessage.success('停用成功')
    fetchCredits()
  } catch (e: any) {
    if (e !== 'cancel') {
      ElMessage.error(e.message || '停用失败')
    }
  }
}

onMounted(() => {
  fetchCredits()
})
</script>

<style scoped>
.customer-credit .card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.customer-credit .toolbar {
  margin-bottom: 16px;
}

.customer-credit .el-table {
  margin-bottom: 16px;
}
</style>
