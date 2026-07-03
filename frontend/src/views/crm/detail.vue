<!--
  crm/detail.vue - CRM 客户 360 详情页
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-3）：
  原 663 行"上帝组件"已拆分为以下独立 section 子组件，
  位于 views/crm/tabs/ 目录：

  | Section     | 子组件                              |
  | ----------- | ----------------------------------- |
  | 跟进记录    | tabs/FollowUpTab.vue                |
  | 标签管理    | tabs/TagsPanelTab.vue               |

  本主入口承担：路由参数解析 + 数据获取 + 布局 + 公共样式。
-->
<template>
  <div class="detail-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">客户 360 视图</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>CRM</el-breadcrumb-item>
          <el-breadcrumb-item>客户详情</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button @click="handleBack">
          <el-icon><Back /></el-icon>
          返回
        </el-button>
      </div>
    </div>

    <div v-loading="loading" class="detail-content">
      <template v-if="customer">
        <el-row :gutter="20">
          <el-col :span="16">
            <el-card shadow="hover" class="section-card">
              <template #header>
                <div class="card-header">
                  <span>基本信息</span>
                  <el-tag :type="customer.status === 'active' ? 'success' : 'info'" size="small">
                    {{ customer.status === 'active' ? '启用' : '禁用' }}
                  </el-tag>
                </div>
              </template>

              <el-descriptions :column="2" border>
                <el-descriptions-item label="客户编码">{{
                  customer.customer_code
                }}</el-descriptions-item>
                <el-descriptions-item label="客户名称">{{
                  customer.customer_name
                }}</el-descriptions-item>
                <el-descriptions-item label="联系人">{{
                  customer.contact_person
                }}</el-descriptions-item>
                <el-descriptions-item label="电话">{{ customer.phone }}</el-descriptions-item>
                <el-descriptions-item label="邮箱" :span="2">{{
                  customer.email
                }}</el-descriptions-item>
                <el-descriptions-item label="地址" :span="2">{{
                  customer.address
                }}</el-descriptions-item>
                <el-descriptions-item label="客户类型">
                  <el-tag :type="getTypeTag(customer.customer_type)" size="small">
                    {{ getTypeLabel(customer.customer_type) }}
                  </el-tag>
                </el-descriptions-item>
                <el-descriptions-item label="负责人">{{
                  customer.owner_name
                }}</el-descriptions-item>
                <el-descriptions-item label="信用额度">
                  {{ customer.credit_limit ? formatCurrency(customer.credit_limit) : '-' }}
                </el-descriptions-item>
                <el-descriptions-item label="订单总数">{{
                  customer.total_orders
                }}</el-descriptions-item>
                <el-descriptions-item label="累计金额">
                  {{ customer.total_amount ? formatCurrency(customer.total_amount) : '-' }}
                </el-descriptions-item>
                <el-descriptions-item label="最近下单">{{
                  customer.last_order_date || '-'
                }}</el-descriptions-item>
              </el-descriptions>
            </el-card>

            <el-card shadow="hover" class="section-card mt-20">
              <template #header>
                <div class="card-header">
                  <span>开票信息</span>
                </div>
              </template>

              <el-descriptions :column="2" border>
                <el-descriptions-item label="税号" :span="2">{{
                  customer.tax_number || '-'
                }}</el-descriptions-item>
                <el-descriptions-item label="开户银行">{{
                  customer.bank_name || '-'
                }}</el-descriptions-item>
                <el-descriptions-item label="银行账号">{{
                  customer.bank_account || '-'
                }}</el-descriptions-item>
              </el-descriptions>
            </el-card>

            <el-card shadow="hover" class="section-card mt-20">
              <template #header>
                <div class="card-header">
                  <span>联系人列表</span>
                  <el-button type="primary" size="small" @click="handleAddContact">
                    <el-icon><Plus /></el-icon>
                    新增联系人
                  </el-button>
                </div>
              </template>

              <el-table :data="contacts" stripe v-loading="contactsLoading">
                <el-table-column prop="name" label="姓名" width="120" />
                <el-table-column prop="title" label="职务" width="150">
                  <template #default="{ row }">{{ row.title || '-' }}</template>
                </el-table-column>
                <el-table-column prop="phone" label="电话" width="140" />
                <el-table-column prop="email" label="邮箱" min-width="180">
                  <template #default="{ row }">{{ row.email || '-' }}</template>
                </el-table-column>
                <el-table-column prop="is_primary" label="主联系人" width="100" align="center">
                  <template #default="{ row }">
                    <el-tag v-if="row.is_primary" type="warning" size="small">主</el-tag>
                  </template>
                </el-table-column>
                <el-table-column label="操作" width="160" align="center">
                  <template #default="{ row }">
                    <el-button size="small" link type="primary" @click="handleEditContact(row)">
                      编辑
                    </el-button>
                    <el-button size="small" link type="danger" @click="handleDeleteContact(row)">
                      删除
                    </el-button>
                  </template>
                </el-table-column>
              </el-table>
            </el-card>
          </el-col>

          <el-col :span="8">
            <el-card shadow="hover">
              <template #header><div class="card-header">RFM 评分</div></template>
              <div v-if="customer.rfm_score" class="rfm-display">
                <div class="rfm-level">
                  <span class="level-badge">{{ customer.rfm_score.level }}</span>
                  <span class="level-label">{{ customer.rfm_score.label }}</span>
                </div>
                <div class="rfm-scores">
                  <div class="rfm-item">
                    <span class="rfm-label">R (最近消费)</span>
                    <span class="rfm-value">{{ customer.rfm_score.recency }}</span>
                  </div>
                  <div class="rfm-item">
                    <span class="rfm-label">F (消费频率)</span>
                    <span class="rfm-value">{{ customer.rfm_score.frequency }}</span>
                  </div>
                  <div class="rfm-item">
                    <span class="rfm-label">M (消费金额)</span>
                    <span class="rfm-value">{{ customer.rfm_score.monetary }}</span>
                  </div>
                </div>
              </div>
              <el-empty v-else description="暂无 RFM 数据" />
            </el-card>

            <TagsPanelTab
              :customer-id="customerId"
              :tags="customer.tags"
              @updated="fetchCustomer360"
            />

            <el-card shadow="hover" class="mt-20">
              <template #header><div class="card-header">收货地址</div></template>
              <div class="address-list">
                <div
                  v-for="addr in customer.shipping_addresses"
                  :key="addr.id"
                  class="address-item"
                >
                  <div class="address-header">
                    <span class="addr-name">{{ addr.name }}</span>
                    <el-tag v-if="addr.is_default" type="warning" size="small">默认</el-tag>
                  </div>
                  <div class="addr-phone">{{ addr.phone }}</div>
                  <div class="addr-detail">
                    {{ addr.province }} {{ addr.city }} {{ addr.district }} {{ addr.detail }}
                  </div>
                </div>
                <el-empty v-if="!customer.shipping_addresses.length" description="暂无收货地址" />
              </div>
            </el-card>
          </el-col>
        </el-row>

        <FollowUpTab ref="followUpRef" :customer-id="customerId" @updated="fetchCustomer360" />
      </template>
    </div>

    <!-- 批次 90b P2-12：联系人新增/编辑对话框（替代占位符） -->
    <el-dialog
      v-model="contactDialogVisible"
      :title="contactDialogTitle"
      width="500px"
      @closed="resetContactForm"
    >
      <el-form
        ref="contactFormRef"
        :model="contactForm"
        :rules="contactFormRules"
        label-width="80px"
      >
        <el-form-item label="姓名" prop="name">
          <el-input v-model="contactForm.name" placeholder="请输入联系人姓名" maxlength="50" />
        </el-form-item>
        <el-form-item label="职务" prop="title">
          <el-input v-model="contactForm.title" placeholder="请输入职务" maxlength="100" />
        </el-form-item>
        <el-form-item label="电话" prop="phone">
          <el-input v-model="contactForm.phone" placeholder="请输入联系电话" maxlength="50" />
        </el-form-item>
        <el-form-item label="邮箱" prop="email">
          <el-input v-model="contactForm.email" placeholder="请输入邮箱" maxlength="100" />
        </el-form-item>
        <el-form-item label="主联系人" prop="is_primary">
          <el-switch v-model="contactForm.is_primary" />
        </el-form-item>
        <el-form-item label="备注" prop="remarks">
          <el-input
            v-model="contactForm.remarks"
            type="textarea"
            :rows="2"
            placeholder="备注（可选）"
            maxlength="500"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="contactDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="contactSubmitting" @click="submitContactForm">
          确定
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Back, Plus } from '@element-plus/icons-vue'
import crmEnhancedApi, { type Contact, type Customer360 } from '@/api/crm-enhanced'
import { logger } from '@/utils/logger'
import FollowUpTab from './tabs/FollowUpTab.vue'
import TagsPanelTab from './tabs/TagsPanelTab.vue'

const route = useRoute()
const router = useRouter()

const loading = ref(false)
const customer = ref<Customer360 | null>(null)
const customerId = Number(route.params.id)
const followUpRef = ref<InstanceType<typeof FollowUpTab> | null>(null)

// 批次 90b P2-12：联系人列表与对话框状态
const contacts = ref<Contact[]>([])
const contactsLoading = ref(false)
const contactDialogVisible = ref(false)
const contactDialogTitle = ref('新增联系人')
const contactSubmitting = ref(false)
const contactFormRef = ref<FormInstance | null>(null)
const editingContactId = ref<number | null>(null)
const contactForm = ref({
  name: '',
  title: '',
  phone: '',
  email: '',
  is_primary: false,
  remarks: '',
})

const contactFormRules: FormRules = {
  name: [{ required: true, message: '请输入联系人姓名', trigger: 'blur' }],
  phone: [{ required: true, message: '请输入联系电话', trigger: 'blur' }],
  email: [{ type: 'email', message: '邮箱格式不正确', trigger: 'blur' }],
}

const formatCurrency = (amount: number) => `¥${(amount || 0).toFixed(2)}`

const getTypeLabel = (type: string) => {
  const labels: Record<string, string> = {
    normal: '普通客户',
    vip: 'VIP客户',
    wholesale: '批发客户',
  }
  return labels[type] || type
}

const getTypeTag = (type: string) => {
  const typeMap: Record<string, string> = { normal: '', vip: 'warning', wholesale: 'success' }
  return typeMap[type] || ''
}

const fetchCustomer360 = async () => {
  loading.value = true
  try {
    const res = await crmEnhancedApi.getCustomer360(customerId)
    customer.value = res.data
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '获取客户详情失败')
  } finally {
    loading.value = false
  }
}

// 批次 90b P2-12：拉取联系人列表（独立于 360 视图，避免每次刷新 360 都重复请求）
const fetchContacts = async () => {
  contactsLoading.value = true
  try {
    const res = await crmEnhancedApi.listContacts(customerId)
    contacts.value = res.data || []
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error)
    ElMessage.error(msg || '获取联系人列表失败')
  } finally {
    contactsLoading.value = false
  }
}

const handleBack = () => {
  router.back()
}

// 批次 90b P2-12：打开新增联系人对话框
const handleAddContact = () => {
  editingContactId.value = null
  contactDialogTitle.value = '新增联系人'
  contactDialogVisible.value = true
}

// 批次 90b P2-12：打开编辑联系人对话框
const handleEditContact = (row: Contact) => {
  editingContactId.value = row.id
  contactDialogTitle.value = '编辑联系人'
  contactForm.value = {
    name: row.name || '',
    title: row.title || '',
    phone: row.phone || '',
    email: row.email || '',
    is_primary: !!row.is_primary,
    remarks: '',
  }
  contactDialogVisible.value = true
}

// 批次 90b P2-12：删除联系人
const handleDeleteContact = async (row: Contact) => {
  try {
    await ElMessageBox.confirm(`确定删除联系人 "${row.name}"？`, '确认删除', {
      type: 'warning',
    })
    await crmEnhancedApi.deleteContact(customerId, row.id)
    ElMessage.success('删除成功')
    fetchContacts()
  } catch (error) {
    if (error === 'cancel') return
    const msg = error instanceof Error ? error.message : String(error)
    ElMessage.error(msg || '删除失败')
  }
}

// 批次 90b P2-12：重置表单
const resetContactForm = () => {
  contactForm.value = {
    name: '',
    title: '',
    phone: '',
    email: '',
    is_primary: false,
    remarks: '',
  }
  editingContactId.value = null
  contactFormRef.value?.clearValidate()
}

// 批次 90b P2-12：提交表单（新增/编辑）
const submitContactForm = async () => {
  if (!contactFormRef.value) return
  // Element Plus validate(callback) 形式下外层 await 不会等待 callback 内 async，故改为 try/catch 形式
  try {
    await contactFormRef.value.validate()
  } catch {
    return // 校验失败，el-form 会自动显示错误
  }
  contactSubmitting.value = true
  try {
    const payload = {
      name: contactForm.value.name,
      title: contactForm.value.title || undefined,
      phone: contactForm.value.phone,
      email: contactForm.value.email || undefined,
      is_primary: contactForm.value.is_primary,
      remarks: contactForm.value.remarks || undefined,
    }
    if (editingContactId.value === null) {
      await crmEnhancedApi.createContact(customerId, payload)
      ElMessage.success('联系人创建成功')
    } else {
      await crmEnhancedApi.updateContact(customerId, editingContactId.value, payload)
      ElMessage.success('联系人更新成功')
    }
    contactDialogVisible.value = false
    fetchContacts()
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error)
    ElMessage.error(msg || '操作失败')
  } finally {
    contactSubmitting.value = false
  }
}

onMounted(() => {
  if (!customerId) {
    ElMessage.error('缺少客户 ID 参数')
    router.back()
    return
  }
  fetchCustomer360()
  fetchContacts()
  logger.info('客户详情页加载完成', { customerId })
})
</script>

<style scoped>
.detail-page {
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
.detail-content {
  min-height: 400px;
}
.section-card {
  margin-bottom: 0;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-weight: 600;
}
.mt-20 {
  margin-top: 20px;
}

.rfm-display {
  padding: 12px 0;
}
.rfm-level {
  text-align: center;
  margin-bottom: 20px;
}
.level-badge {
  display: inline-block;
  width: 60px;
  height: 60px;
  line-height: 60px;
  border-radius: 50%;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: #fff;
  font-size: 28px;
  font-weight: 700;
}
.level-label {
  display: block;
  margin-top: 8px;
  font-size: 14px;
  color: #606266;
}
.rfm-scores {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.rfm-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  background: #fafafa;
  border-radius: 6px;
}
.rfm-label {
  font-size: 13px;
  color: #606266;
}
.rfm-value {
  font-size: 18px;
  font-weight: 600;
  color: #303133;
}

.address-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.address-item {
  padding: 12px;
  background: #fafafa;
  border-radius: 6px;
}
.address-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}
.addr-name {
  font-weight: 600;
  color: #303133;
}
.addr-phone {
  font-size: 13px;
  color: #606266;
  margin-bottom: 4px;
}
.addr-detail {
  font-size: 13px;
  color: #909399;
}
</style>
