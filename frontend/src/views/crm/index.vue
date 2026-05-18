<template>
  <div class="crm-page">
    <el-tabs v-model="activeTab">
      <el-tab-pane label="线索管理" name="lead">
        <div class="page-header">
          <h2 class="page-title">线索管理</h2>
          <el-button type="primary" @click="openLeadDialog()">
            <el-icon><Plus /></el-icon>
            新建线索
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table :data="leads" v-loading="leadLoading" stripe>
            <el-table-column prop="lead_no" label="线索编号" width="120" />
            <el-table-column prop="name" label="联系人" width="120" />
            <el-table-column prop="company" label="公司名称" min-width="150" />
            <el-table-column prop="phone" label="电话" width="130" />
            <el-table-column prop="email" label="邮箱" min-width="180" />
            <el-table-column prop="source" label="来源" width="100" />
            <el-table-column prop="rating" label="评分" width="80" align="center">
              <template #default="{ row }">
                <el-tag :type="row.rating >= 4 ? 'success' : row.rating >= 2 ? 'warning' : 'info'" size="small">
                  {{ '★'.repeat(row.rating) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="status" label="状态" width="100" align="center">
              <template #default="{ row }">
                <el-tag :type="getLeadStatusType(row.status)" size="small">{{ getLeadStatusLabel(row.status) }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="assigned_to_name" label="负责人" width="100" />
            <el-table-column label="操作" width="200" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="viewLead(row)">查看</el-button>
                <el-button type="success" link size="small" @click="convertLead(row)">转化</el-button>
                <el-button type="warning" link size="small" @click="updateLeadStatus(row)">更新状态</el-button>
                <el-button type="danger" link size="small" @click="deleteLead(row)">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="商机管理" name="opportunity">
        <div class="page-header">
          <h2 class="page-title">商机管理</h2>
          <el-button type="primary" @click="openOpportunityDialog()">
            <el-icon><Plus /></el-icon>
            新建商机
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table :data="opportunities" v-loading="opportunityLoading" stripe>
            <el-table-column prop="opportunity_no" label="商机编号" width="120" />
            <el-table-column prop="name" label="商机名称" width="180" />
            <el-table-column prop="customer_name" label="客户" width="150" />
            <el-table-column prop="estimated_amount" label="预估金额" width="120" align="right">
              <template #default="{ row }">{{ formatMoney(row.estimated_amount) }}</template>
            </el-table-column>
            <el-table-column prop="probability" label="成功率" width="100" align="center">
              <template #default="{ row }">{{ row.probability }}%</template>
            </el-table-column>
            <el-table-column prop="stage" label="阶段" width="140">
              <template #default="{ row }">{{ getStageLabel(row.stage) }}</template>
            </el-table-column>
            <el-table-column prop="expected_close_date" label="预计成交" width="120" />
            <el-table-column label="操作" width="200" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="viewOpportunity(row)">查看</el-button>
                <el-button type="danger" link size="small" @click="deleteOpportunity(row)">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="客户画像" name="customer">
        <div class="page-header">
          <h2 class="page-title">客户画像</h2>
        </div>

        <el-row :gutter="20">
          <el-col :span="8">
            <el-card shadow="hover">
              <template #header><div class="card-header">客户概览</div></template>
              <el-statistic title="客户总数" :value="128" />
              <el-divider />
              <el-statistic title="活跃客户" :value="86" />
            </el-card>
          </el-col>
          <el-col :span="8">
            <el-card shadow="hover">
              <template #header><div class="card-header">销售数据</div></template>
              <el-statistic title="本月成交额" :value="128500" prefix="¥" />
              <el-divider />
              <el-statistic title="待跟进商机" :value="24" />
            </el-card>
          </el-col>
          <el-col :span="8">
            <el-card shadow="hover">
              <template #header><div class="card-header">线索数据</div></template>
              <el-statistic title="本月新增线索" :value="36" />
              <el-divider />
              <el-statistic title="线索转化率" :value="28" suffix="%" />
            </el-card>
          </el-col>
        </el-row>

        <el-card shadow="hover" class="mt-20">
          <template #header><div class="card-header">客户分析</div></template>
          <el-empty description="客户分析图表区域" />
        </el-card>
      </el-tab-pane>
    </el-tabs>

    <!-- 线索对话框 -->
    <el-dialog v-model="leadDialogVisible" :title="leadDialogTitle" width="600px">
      <el-form :model="leadForm" label-width="100px">
        <el-form-item label="线索编号">
          <el-input v-model="leadForm.lead_no" placeholder="请输入线索编号" />
        </el-form-item>
        <el-form-item label="联系人">
          <el-input v-model="leadForm.name" placeholder="请输入联系人姓名" />
        </el-form-item>
        <el-form-item label="公司名称">
          <el-input v-model="leadForm.company" placeholder="请输入公司名称" />
        </el-form-item>
        <el-form-item label="电话">
          <el-input v-model="leadForm.phone" placeholder="请输入电话" />
        </el-form-item>
        <el-form-item label="邮箱">
          <el-input v-model="leadForm.email" placeholder="请输入邮箱" />
        </el-form-item>
        <el-form-item label="来源">
          <el-select v-model="leadForm.source" placeholder="请选择来源" style="width: 100%">
            <el-option label="网站" value="website" />
            <el-option label="电话" value="phone" />
            <el-option label="展会" value="exhibition" />
            <el-option label="推荐" value="referral" />
            <el-option label="其他" value="other" />
          </el-select>
        </el-form-item>
        <el-form-item label="评分">
          <el-rate v-model="leadForm.rating" :max="5" />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="leadForm.status" placeholder="请选择状态" style="width: 100%">
            <el-option label="新线索" value="new" />
            <el-option label="已联系" value="contacted" />
            <el-option label="已转化" value="qualified" />
            <el-option label="提案中" value="proposal" />
            <el-option label="已成交" value="converted" />
            <el-option label="已流失" value="lost" />
          </el-select>
        </el-form-item>
        <el-form-item label="地址">
          <el-input v-model="leadForm.address" placeholder="请输入地址" />
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="leadForm.description" type="textarea" placeholder="请输入描述" />
        </el-form-item>
      </el-form>
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="leadDialogVisible = false">取消</el-button>
          <el-button type="primary" @click="submitLead">确定</el-button>
        </span>
      </template>
    </el-dialog>

    <!-- 商机对话框 -->
    <el-dialog v-model="opportunityDialogVisible" :title="opportunityDialogTitle" width="600px">
      <el-form :model="opportunityForm" label-width="100px">
        <el-form-item label="商机编号">
          <el-input v-model="opportunityForm.opportunity_no" placeholder="请输入商机编号" />
        </el-form-item>
        <el-form-item label="商机名称">
          <el-input v-model="opportunityForm.name" placeholder="请输入商机名称" />
        </el-form-item>
        <el-form-item label="客户">
          <el-input v-model="opportunityForm.customer_name" placeholder="请输入客户名称" />
        </el-form-item>
        <el-form-item label="预估金额">
          <el-input-number v-model="opportunityForm.estimated_amount" :min="0" :precision="2" style="width: 100%" />
        </el-form-item>
        <el-form-item label="成功率">
          <el-slider v-model="opportunityForm.probability" :min="0" :max="100" />
        </el-form-item>
        <el-form-item label="阶段">
          <el-select v-model="opportunityForm.stage" placeholder="请选择阶段" style="width: 100%">
            <el-option label="需求评估" value="qualification" />
            <el-option label="需求分析" value="needs_analysis" />
            <el-option label="价值呈现" value="value_proposition" />
            <el-option label="方案阶段" value="proposal" />
            <el-option label="商务谈判" value="negotiation" />
            <el-option label="成交" value="closed_won" />
            <el-option label="流标" value="closed_lost" />
          </el-select>
        </el-form-item>
        <el-form-item label="预计成交">
          <el-date-picker v-model="opportunityForm.expected_close_date" type="date" placeholder="选择日期" style="width: 100%" />
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="opportunityForm.description" type="textarea" placeholder="请输入描述" />
        </el-form-item>
      </el-form>
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="opportunityDialogVisible = false">取消</el-button>
          <el-button type="primary" @click="submitOpportunity">确定</el-button>
        </span>
      </template>
    </el-dialog>

    <!-- 线索状态更新对话框 -->
    <el-dialog v-model="leadStatusDialogVisible" title="更新线索状态" width="400px">
      <el-form :model="leadStatusForm" label-width="80px">
        <el-form-item label="状态">
          <el-select v-model="leadStatusForm.status" placeholder="请选择状态" style="width: 100%">
            <el-option label="新线索" value="new" />
            <el-option label="已联系" value="contacted" />
            <el-option label="已转化" value="qualified" />
            <el-option label="提案中" value="proposal" />
            <el-option label="已成交" value="converted" />
            <el-option label="已流失" value="lost" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="leadStatusDialogVisible = false">取消</el-button>
          <el-button type="primary" @click="submitLeadStatus">确定</el-button>
        </span>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import { 
  listLeads, 
  createLead, 
  updateLead, 
  deleteLead as apiDeleteLead,
  updateLeadStatus as apiUpdateLeadStatus,
  convertLead as apiConvertLead,
  listOpportunities, 
  createOpportunity, 
  updateOpportunity, 
  deleteOpportunity as apiDeleteOpportunity,
  type Lead, 
  type Opportunity 
} from '@/api/crm'

const activeTab = ref('lead')
const leads = ref<Lead[]>([])
const opportunities = ref<Opportunity[]>([])
const leadLoading = ref(false)
const opportunityLoading = ref(false)

// 对话框状态
const leadDialogVisible = ref(false)
const opportunityDialogVisible = ref(false)
const leadStatusDialogVisible = ref(false)

// 对话框标题
const leadDialogTitle = ref('新建线索')
const opportunityDialogTitle = ref('新建商机')

// 表单数据
const leadForm = ref({
  id: null as number | null,
  lead_no: '',
  name: '',
  company: '',
  phone: '',
  email: '',
  source: '',
  rating: 3,
  status: 'new',
  address: '',
  description: ''
})

const opportunityForm = ref({
  id: null as number | null,
  opportunity_no: '',
  name: '',
  customer_name: '',
  estimated_amount: 0,
  probability: 50,
  stage: 'qualification',
  expected_close_date: '',
  description: ''
})

const leadStatusForm = ref({
  id: null as number | null,
  status: ''
})

const fetchLeads = async () => {
  leadLoading.value = true
  try {
    const res = await listLeads()
    leads.value = res.data! || []
  } finally {
    leadLoading.value = false
  }
}

const fetchOpportunities = async () => {
  opportunityLoading.value = true
  try {
    const res = await listOpportunities()
    opportunities.value = res.data! || []
  } finally {
    opportunityLoading.value = false
  }
}

const formatMoney = (amount: number) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const getLeadStatusType = (status: string) => {
  const map: Record<string, any> = { new: 'info', contacted: 'primary', qualified: 'success', proposal: 'warning', converted: 'success', lost: 'danger' }
  return map[status] || 'info'
}

const getLeadStatusLabel = (status: string) => {
  const map: Record<string, string> = { new: '新线索', contacted: '已联系', qualified: '已转化', proposal: '提案中', converted: '已成交', lost: '已流失' }
  return map[status] || status
}

const getStageLabel = (stage: string) => {
  const map: Record<string, string> = { qualification: '需求评估', needs_analysis: '需求分析', value_proposition: '价值呈现', proposal: '方案阶段', negotiation: '商务谈判', closed_won: '成交', closed_lost: '流标' }
  return map[stage] || stage
}

// 线索操作
const openLeadDialog = (row?: Lead) => {
  if (row) {
    leadDialogTitle.value = '编辑线索'
    leadForm.value = { ...row }
  } else {
    leadDialogTitle.value = '新建线索'
    leadForm.value = {
      id: null,
      lead_no: '',
      name: '',
      company: '',
      phone: '',
      email: '',
      source: '',
      rating: 3,
      status: 'new',
      address: '',
      description: ''
    }
  }
  leadDialogVisible.value = true
}

const viewLead = (row: Lead) => {
  openLeadDialog(row)
}

const submitLead = async () => {
  try {
    const { id, status, ...rest } = leadForm.value
    if (id) {
      await updateLead(id, { ...rest, status: status as Lead['status'] })
      ElMessage.success('更新成功')
    } else {
      await createLead({ ...rest, status: status as Lead['status'] })
      ElMessage.success('创建成功')
    }
    leadDialogVisible.value = false
    fetchLeads()
  } catch (error) {
    ElMessage.error('操作失败')
  }
}

const convertLead = async (row: Lead) => {
  try {
    await ElMessageBox.confirm('确定转化此线索为客户和商机吗？', '确认转化', { type: 'info' })
    await apiConvertLead(row.id)
    ElMessage.success('转化成功')
    fetchLeads()
  } catch (e) { if (e !== 'cancel') console.error(e) }
}

const updateLeadStatus = (row: Lead) => {
  leadStatusForm.value = {
    id: row.id,
    status: row.status
  }
  leadStatusDialogVisible.value = true
}

const submitLeadStatus = async () => {
  try {
    if (leadStatusForm.value.id) {
      await apiUpdateLeadStatus(leadStatusForm.value.id, { status: leadStatusForm.value.status })
      ElMessage.success('状态更新成功')
      leadStatusDialogVisible.value = false
      fetchLeads()
    }
  } catch (error) {
    ElMessage.error('操作失败')
  }
}

const deleteLead = async (row: Lead) => {
  try {
    await ElMessageBox.confirm('确定删除此线索吗？', '确认', { type: 'warning' })
    await apiDeleteLead(row.id)
    ElMessage.success('删除成功')
    fetchLeads()
  } catch (e) { if (e !== 'cancel') console.error(e) }
}

// 商机操作
const openOpportunityDialog = (row?: Opportunity) => {
  if (row) {
    opportunityDialogTitle.value = '编辑商机'
    opportunityForm.value = { ...row }
  } else {
    opportunityDialogTitle.value = '新建商机'
    opportunityForm.value = {
      id: null,
      opportunity_no: '',
      name: '',
      customer_name: '',
      estimated_amount: 0,
      probability: 50,
      stage: 'qualification',
      expected_close_date: '',
      description: ''
    }
  }
  opportunityDialogVisible.value = true
}

const viewOpportunity = (row: Opportunity) => {
  openOpportunityDialog(row)
}

const submitOpportunity = async () => {
  try {
    const { id, stage, ...rest } = opportunityForm.value
    if (id) {
      await updateOpportunity(id, { ...rest, stage: stage as Opportunity['stage'] })
      ElMessage.success('更新成功')
    } else {
      await createOpportunity({ ...rest, stage: stage as Opportunity['stage'] })
      ElMessage.success('创建成功')
    }
    opportunityDialogVisible.value = false
    fetchOpportunities()
  } catch (error) {
    ElMessage.error('操作失败')
  }
}

const deleteOpportunity = async (row: Opportunity) => {
  try {
    await ElMessageBox.confirm('确定删除此商机吗？', '确认', { type: 'warning' })
    await apiDeleteOpportunity(row.id)
    ElMessage.success('删除成功')
    fetchOpportunities()
  } catch (e) { if (e !== 'cancel') console.error(e) }
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
.card-header { font-weight: 600; }
.mt-20 { margin-top: 20px; }
</style>
