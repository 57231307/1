<template>
  <div class="crm-page">
    <el-tabs v-model="activeTab">
      <el-tab-pane label="线索管理" name="lead">
        <div class="page-header">
          <h2 class="page-title">线索管理</h2>
          <el-button type="primary" @click="openLeadDialog">
            <el-icon><Plus /></el-icon>
            新建线索
          </el-button>
        </div>

        <el-card shadow="hover" class="filter-card">
          <el-form :inline="true" :model="leadQuery">
            <el-form-item label="姓名">
              <el-input v-model="leadQuery.name" placeholder="姓名" clearable />
            </el-form-item>
            <el-form-item label="来源">
              <el-input v-model="leadQuery.source" placeholder="来源" clearable />
            </el-form-item>
            <el-form-item label="状态">
              <el-select v-model="leadQuery.status" placeholder="选择状态" clearable>
                <el-option label="新线索" value="new" />
                <el-option label="已联系" value="contacted" />
                <el-option label="已合格" value="qualified" />
                <el-option label="已转化" value="converted" />
                <el-option label="已流失" value="lost" />
              </el-select>
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="fetchLeads">查询</el-button>
              <el-button @click="resetLeadQuery">重置</el-button>
            </el-form-item>
          </el-form>
        </el-card>

        <el-card shadow="hover">
          <el-table :data="leads" v-loading="leadLoading" stripe>
            <el-table-column prop="lead_no" label="线索编号" width="140" />
            <el-table-column prop="name" label="姓名" width="120" />
            <el-table-column prop="phone" label="电话" width="120" />
            <el-table-column prop="email" label="邮箱" min-width="150" />
            <el-table-column prop="company" label="公司" min-width="120" />
            <el-table-column prop="source" label="来源" width="100" />
            <el-table-column prop="status" label="状态" width="100" align="center">
              <template #default="{ row }">
                <el-tag :type="getLeadStatusType(row.status)" size="small">
                  {{ getLeadStatusLabel(row.status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="rating" label="评分" width="80" align="center" />
            <el-table-column label="操作" width="200" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="openLeadDialog(row)">编辑</el-button>
                <el-button v-if="row.status !== 'converted'" type="success" link size="small" @click="convertLead(row)">转化</el-button>
                <el-button type="danger" link size="small" @click="deleteLead(row)">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="商机管理" name="opportunity">
        <div class="page-header">
          <h2 class="page-title">商机管理</h2>
          <el-button type="primary" @click="openOpportunityDialog">
            <el-icon><Plus /></el-icon>
            新建商机
          </el-button>
        </div>

        <el-card shadow="hover" class="filter-card">
          <el-form :inline="true" :model="opportunityQuery">
            <el-form-item label="商机名称">
              <el-input v-model="opportunityQuery.name" placeholder="商机名称" clearable />
            </el-form-item>
            <el-form-item label="客户">
              <el-input v-model="opportunityQuery.customer_name" placeholder="客户名称" clearable />
            </el-form-item>
            <el-form-item label="阶段">
              <el-select v-model="opportunityQuery.stage" placeholder="选择阶段" clearable>
                <el-option label="需求确认" value="qualification" />
                <el-option label="需求分析" value="needs_analysis" />
                <el-option label="价值提案" value="value_proposition" />
                <el-option label="方案演示" value="proposal" />
                <el-option label="商务谈判" value="negotiation" />
                <el-option label="成功关闭" value="closed_won" />
                <el-option label="失败关闭" value="closed_lost" />
              </el-select>
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="fetchOpportunities">查询</el-button>
              <el-button @click="resetOpportunityQuery">重置</el-button>
            </el-form-item>
          </el-form>
        </el-card>

        <el-card shadow="hover">
          <el-table :data="opportunities" v-loading="opportunityLoading" stripe>
            <el-table-column prop="opportunity_no" label="商机编号" width="140" />
            <el-table-column prop="name" label="商机名称" min-width="150" />
            <el-table-column prop="customer_name" label="客户" min-width="120" />
            <el-table-column prop="stage" label="阶段" width="120">
              <template #default="{ row }">
                <el-tag :type="getOpportunityStageType(row.stage)" size="small">
                  {{ getOpportunityStageLabel(row.stage) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="estimated_amount" label="预估金额" width="120" align="right">
              <template #default="{ row }">
                {{ formatMoney(row.estimated_amount) }}
              </template>
            </el-table-column>
            <el-table-column prop="probability" label="成功率" width="100" align="center">
              <template #default="{ row }">
                {{ row.probability }}%
              </template>
            </el-table-column>
            <el-table-column prop="expected_close_date" label="预计关闭日期" width="120" />
            <el-table-column label="操作" width="150" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="openOpportunityDialog(row)">编辑</el-button>
                <el-button type="danger" link size="small" @click="deleteOpportunity(row)">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>
    </el-tabs>

    <el-dialog v-model="leadDialogVisible" :title="leadForm.id ? '编辑线索' : '新建线索'" width="600px">
      <el-form ref="leadFormRef" :model="leadForm" :rules="leadRules" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="线索编号" prop="lead_no">
              <el-input v-model="leadForm.lead_no" :disabled="!!leadForm.id" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="姓名" prop="name">
              <el-input v-model="leadForm.name" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="电话" prop="phone">
              <el-input v-model="leadForm.phone" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="邮箱" prop="email">
              <el-input v-model="leadForm.email" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="公司" prop="company">
              <el-input v-model="leadForm.company" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="来源" prop="source">
              <el-input v-model="leadForm.source" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="状态" prop="status">
              <el-select v-model="leadForm.status" placeholder="选择状态" style="width: 100%">
                <el-option label="新线索" value="new" />
                <el-option label="已联系" value="contacted" />
                <el-option label="已合格" value="qualified" />
                <el-option label="已转化" value="converted" />
                <el-option label="已流失" value="lost" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="评分" prop="rating">
              <el-input-number v-model="leadForm.rating" :min="1" :max="5" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="地址" prop="address">
          <el-input v-model="leadForm.address" />
        </el-form-item>
        <el-form-item label="描述" prop="description">
          <el-input v-model="leadForm.description" type="textarea" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="leadDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="leadSubmitLoading" @click="submitLead">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="opportunityDialogVisible" :title="opportunityForm.id ? '编辑商机' : '新建商机'" width="600px">
      <el-form ref="opportunityFormRef" :model="opportunityForm" :rules="opportunityRules" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="商机编号" prop="opportunity_no">
              <el-input v-model="opportunityForm.opportunity_no" :disabled="!!opportunityForm.id" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="商机名称" prop="name">
              <el-input v-model="opportunityForm.name" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="客户名称" prop="customer_name">
              <el-input v-model="opportunityForm.customer_name" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="阶段" prop="stage">
              <el-select v-model="opportunityForm.stage" placeholder="选择阶段" style="width: 100%">
                <el-option label="需求确认" value="qualification" />
                <el-option label="需求分析" value="needs_analysis" />
                <el-option label="价值提案" value="value_proposition" />
                <el-option label="方案演示" value="proposal" />
                <el-option label="商务谈判" value="negotiation" />
                <el-option label="成功关闭" value="closed_won" />
                <el-option label="失败关闭" value="closed_lost" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="预估金额" prop="estimated_amount">
              <el-input-number v-model="opportunityForm.estimated_amount" :min="0" :precision="2" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="成功率(%)" prop="probability">
              <el-input-number v-model="opportunityForm.probability" :min="0" :max="100" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="预计关闭日期" prop="expected_close_date">
          <el-date-picker v-model="opportunityForm.expected_close_date" type="date" style="width: 100%" value-format="YYYY-MM-DD" />
        </el-form-item>
        <el-form-item label="描述" prop="description">
          <el-input v-model="opportunityForm.description" type="textarea" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="opportunityDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="opportunitySubmitLoading" @click="submitOpportunity">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'
import {
  listLeads,
  getLead,
  createLead,
  updateLead,
  deleteLead,
  convertLead,
  type Lead
} from '@/api/crm'
import {
  listOpportunities,
  getOpportunity,
  createOpportunity,
  updateOpportunity,
  deleteOpportunity,
  type Opportunity
} from '@/api/crm'

const activeTab = ref('lead')

const leads = ref<Lead[]>([])
const opportunities = ref<Opportunity[]>([])
const leadLoading = ref(false)
const opportunityLoading = ref(false)

const leadQuery = reactive({
  name: '',
  source: '',
  status: ''
})

const opportunityQuery = reactive({
  name: '',
  customer_name: '',
  stage: ''
})

const formatMoney = (amount: number) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const fetchLeads = async () => {
  leadLoading.value = true
  try {
    const res = await listLeads(leadQuery)
    leads.value = res.data || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取线索列表失败')
  } finally {
    leadLoading.value = false
  }
}

const fetchOpportunities = async () => {
  opportunityLoading.value = true
  try {
    const res = await listOpportunities(opportunityQuery)
    opportunities.value = res.data || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取商机列表失败')
  } finally {
    opportunityLoading.value = false
  }
}

const resetLeadQuery = () => {
  leadQuery.name = ''
  leadQuery.source = ''
  leadQuery.status = ''
  fetchLeads()
}

const resetOpportunityQuery = () => {
  opportunityQuery.name = ''
  opportunityQuery.customer_name = ''
  opportunityQuery.stage = ''
  fetchOpportunities()
}

const getLeadStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    new: '新线索',
    contacted: '已联系',
    qualified: '已合格',
    converted: '已转化',
    lost: '已流失'
  }
  return map[status] || status
}

const getLeadStatusType = (status: string) => {
  const map: Record<string, any> = {
    new: 'info',
    contacted: 'warning',
    qualified: 'primary',
    converted: 'success',
    lost: 'danger'
  }
  return map[status] || 'info'
}

const getOpportunityStageLabel = (stage: string) => {
  const map: Record<string, string> = {
    qualification: '需求确认',
    needs_analysis: '需求分析',
    value_proposition: '价值提案',
    proposal: '方案演示',
    negotiation: '商务谈判',
    closed_won: '成功关闭',
    closed_lost: '失败关闭'
  }
  return map[stage] || stage
}

const getOpportunityStageType = (stage: string) => {
  const map: Record<string, any> = {
    qualification: 'info',
    needs_analysis: 'primary',
    value_proposition: 'primary',
    proposal: 'warning',
    negotiation: 'warning',
    closed_won: 'success',
    closed_lost: 'danger'
  }
  return map[stage] || 'info'
}

const leadDialogVisible = ref(false)
const leadFormRef = ref<FormInstance>()
const leadSubmitLoading = ref(false)
const leadForm = reactive({
  id: 0,
  lead_no: '',
  name: '',
  phone: '',
  email: '',
  company: '',
  source: '',
  status: 'new' as 'new' | 'contacted' | 'qualified' | 'converted' | 'lost',
  rating: 3,
  address: '',
  description: '',
  created_by: 0,
  created_by_name: '',
  assigned_to: 0,
  assigned_to_name: ''
})

const leadRules: FormRules = {
  lead_no: [{ required: true, message: '请输入线索编号', trigger: 'blur' }],
  name: [{ required: true, message: '请输入姓名', trigger: 'blur' }],
  phone: [{ required: true, message: '请输入电话', trigger: 'blur' }],
  source: [{ required: true, message: '请输入来源', trigger: 'blur' }]
}

const openLeadDialog = async (row?: Lead) => {
  if (row) {
    const res = await getLead(row.id)
    Object.assign(leadForm, res.data)
  } else {
    Object.assign(leadForm, {
      id: 0,
      lead_no: '',
      name: '',
      phone: '',
      email: '',
      company: '',
      source: '',
      status: 'new',
      rating: 3,
      address: '',
      description: '',
      created_by: 0,
      created_by_name: '',
      assigned_to: 0,
      assigned_to_name: ''
    })
  }
  leadDialogVisible.value = true
}

const submitLead = async () => {
  const valid = await leadFormRef.value?.validate()
  if (!valid) return

  leadSubmitLoading.value = true
  try {
    if (leadForm.id) {
      await updateLead(leadForm.id, leadForm)
      ElMessage.success('更新成功')
    } else {
      await createLead(leadForm)
      ElMessage.success('创建成功')
    }
    leadDialogVisible.value = false
    fetchLeads()
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
  } finally {
    leadSubmitLoading.value = false
  }
}

const convertLead = async (row: Lead) => {
  try {
    await ElMessageBox.confirm('确定将此线索转化为客户和商机吗？', '确认', { type: 'info' })
    await convertLead(row.id)
    ElMessage.success('转化成功')
    fetchLeads()
  } catch (error: any) {
    if (error !== 'cancel') ElMessage.error(error.message || '操作失败')
  }
}

const deleteLead = async (row: Lead) => {
  try {
    await ElMessageBox.confirm('确定删除此线索吗？', '删除确认', { type: 'warning' })
    await deleteLead(row.id)
    ElMessage.success('删除成功')
    fetchLeads()
  } catch (error: any) {
    if (error !== 'cancel') ElMessage.error(error.message || '删除失败')
  }
}

const opportunityDialogVisible = ref(false)
const opportunityFormRef = ref<FormInstance>()
const opportunitySubmitLoading = ref(false)
const opportunityForm = reactive({
  id: 0,
  opportunity_no: '',
  name: '',
  customer_id: 0,
  customer_name: '',
  stage: 'qualification' as 'qualification' | 'needs_analysis' | 'value_proposition' | 'proposal' | 'negotiation' | 'closed_won' | 'closed_lost',
  estimated_amount: 0,
  probability: 50,
  expected_close_date: '',
  description: '',
  created_by: 0,
  created_by_name: ''
})

const opportunityRules: FormRules = {
  opportunity_no: [{ required: true, message: '请输入商机编号', trigger: 'blur' }],
  name: [{ required: true, message: '请输入商机名称', trigger: 'blur' }],
  customer_name: [{ required: true, message: '请输入客户名称', trigger: 'blur' }],
  stage: [{ required: true, message: '请选择阶段', trigger: 'change' }],
  estimated_amount: [{ required: true, message: '请输入预估金额', trigger: 'blur' }]
}

const openOpportunityDialog = async (row?: Opportunity) => {
  if (row) {
    const res = await getOpportunity(row.id)
    Object.assign(opportunityForm, res.data)
  } else {
    Object.assign(opportunityForm, {
      id: 0,
      opportunity_no: '',
      name: '',
      customer_id: 0,
      customer_name: '',
      stage: 'qualification',
      estimated_amount: 0,
      probability: 50,
      expected_close_date: '',
      description: '',
      created_by: 0,
      created_by_name: ''
    })
  }
  opportunityDialogVisible.value = true
}

const submitOpportunity = async () => {
  const valid = await opportunityFormRef.value?.validate()
  if (!valid) return

  opportunitySubmitLoading.value = true
  try {
    if (opportunityForm.id) {
      await updateOpportunity(opportunityForm.id, opportunityForm)
      ElMessage.success('更新成功')
    } else {
      await createOpportunity(opportunityForm)
      ElMessage.success('创建成功')
    }
    opportunityDialogVisible.value = false
    fetchOpportunities()
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
  } finally {
    opportunitySubmitLoading.value = false
  }
}

const deleteOpportunity = async (row: Opportunity) => {
  try {
    await ElMessageBox.confirm('确定删除此商机吗？', '删除确认', { type: 'warning' })
    await deleteOpportunity(row.id)
    ElMessage.success('删除成功')
    fetchOpportunities()
  } catch (error: any) {
    if (error !== 'cancel') ElMessage.error(error.message || '删除失败')
  }
}

onMounted(() => {
  fetchLeads()
  fetchOpportunities()
})
</script>

<style scoped>
.crm-page { padding: 24px; background-color: #f5f7fa; min-height: 100%; }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; }
.page-title { font-size: 20px; font-weight: 600; color: #303133; margin: 0; }
.filter-card { margin-bottom: 20px; }
</style>
