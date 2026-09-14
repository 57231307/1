<template>
  <div class="supplier-enhanced-page">
    <el-card shadow="never">
      <div class="page-header">
        <h2>供应商 360</h2>
        <div class="header-actions">
          <el-input-number
            v-model="supplierId"
            :min="1"
            :precision="0"
            placeholder="供应商 ID"
            style="width: 180px"
          />
          <el-button type="primary" :loading="pageLoading" @click="loadAll">查询</el-button>
        </div>
      </div>

      <el-descriptions v-if="supplier" :column="4" border class="block-gap">
        <el-descriptions-item label="供应商编码">{{ supplier.supplier_code }}</el-descriptions-item>
        <el-descriptions-item label="供应商名称">{{ supplier.supplier_name }}</el-descriptions-item>
        <el-descriptions-item label="等级">{{ supplier.grade || '-' }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag>{{ supplier.status }}</el-tag>
        </el-descriptions-item>
      </el-descriptions>

      <el-empty v-if="!loaded" description="请输入供应商 ID 后点击查询" />

      <el-tabs v-else v-model="activeTab">
        <!-- 页签一：联系人 -->
        <el-tab-pane label="联系人" name="contacts">
          <div class="toolbar">
            <el-button type="primary" @click="openContactDialog">新增联系人</el-button>
          </div>
          <el-table v-loading="contactsLoading" :data="contacts" border>
            <el-table-column prop="contact_name" label="姓名" min-width="100" />
            <el-table-column label="部门" width="110">
              <template #default="{ row }">{{ row.department || '-' }}</template>
            </el-table-column>
            <el-table-column label="职位" width="110">
              <template #default="{ row }">{{ row.position || '-' }}</template>
            </el-table-column>
            <el-table-column prop="mobile_phone" label="手机" width="130" />
            <el-table-column label="电话" width="130">
              <template #default="{ row }">{{ row.tel_phone || '-' }}</template>
            </el-table-column>
            <el-table-column label="邮箱" min-width="150">
              <template #default="{ row }">{{ row.email || '-' }}</template>
            </el-table-column>
            <el-table-column label="微信" width="120">
              <template #default="{ row }">{{ row.wechat || '-' }}</template>
            </el-table-column>
            <el-table-column label="主要联系人" width="110" align="center">
              <template #default="{ row }">
                <el-tag v-if="row.is_primary" type="success" size="small">是</el-tag>
                <span v-else>否</span>
              </template>
            </el-table-column>
            <el-table-column label="备注" min-width="140" show-overflow-tooltip>
              <template #default="{ row }">{{ row.remarks || '-' }}</template>
            </el-table-column>
          </el-table>
        </el-tab-pane>

        <!-- 页签二：资质 -->
        <el-tab-pane label="资质" name="qualifications">
          <div class="toolbar">
            <el-button type="primary" @click="openQualificationDialog">新增资质</el-button>
          </div>
          <el-table v-loading="qualificationsLoading" :data="qualifications" border>
            <el-table-column prop="qualification_name" label="资质名称" min-width="140" />
            <el-table-column prop="qualification_type" label="类型" width="120" />
            <el-table-column prop="qualification_no" label="证照编号" min-width="140" />
            <el-table-column prop="issuing_authority" label="发证机关" min-width="140" />
            <el-table-column prop="issue_date" label="发证日期" width="110" />
            <el-table-column prop="valid_until" label="有效期至" width="110" />
            <el-table-column label="年检" width="90" align="center">
              <template #default="{ row }">{{
                row.need_annual_check ? '需要' : '不需要'
              }}</template>
            </el-table-column>
            <el-table-column label="是否过期" width="100" align="center">
              <template #default="{ row }">
                <el-tag v-if="row.is_expired" type="danger" size="small">已过期</el-tag>
                <el-tag v-else type="success" size="small">有效</el-tag>
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>

        <!-- 页签三：采购历史与余额 -->
        <el-tab-pane label="采购历史与余额" name="history">
          <h3 class="section-title">账户余额</h3>
          <el-empty v-if="!balance" description="暂无余额数据" :image-size="60" />
          <el-descriptions v-else :column="4" border>
            <el-descriptions-item label="订单总额">
              {{ fmtAmount(balance.total_amount) }}
            </el-descriptions-item>
            <el-descriptions-item label="已付款">
              {{ fmtAmount(balance.paid_amount) }}
            </el-descriptions-item>
            <el-descriptions-item label="余额">{{
              fmtAmount(balance.balance)
            }}</el-descriptions-item>
            <el-descriptions-item label="订单数">{{ balance.order_count }}</el-descriptions-item>
          </el-descriptions>

          <h3 class="section-title">采购历史</h3>
          <el-table v-loading="historyLoading" :data="historyRows" border>
            <el-table-column prop="order_no" label="订单编号" min-width="160" />
            <el-table-column prop="order_date" label="订单日期" width="120" />
            <el-table-column label="总金额" width="140" align="right">
              <template #default="{ row }">{{ fmtAmount(row.total_amount) }}</template>
            </el-table-column>
            <el-table-column prop="status" label="状态" width="110" />
            <el-table-column prop="item_count" label="商品项数" width="100" align="right" />
          </el-table>
        </el-tab-pane>
      </el-tabs>
    </el-card>

    <!-- 新增联系人弹窗 -->
    <el-dialog
      v-model="contactDialogVisible"
      title="新增联系人"
      width="520px"
      @close="resetContactForm"
    >
      <el-form ref="contactFormRef" :model="contactForm" :rules="contactRules" label-width="100px">
        <el-form-item label="姓名" prop="contact_name">
          <el-input v-model="contactForm.contact_name" placeholder="必填" />
        </el-form-item>
        <el-form-item label="部门" prop="department">
          <el-input v-model="contactForm.department" placeholder="选填" />
        </el-form-item>
        <el-form-item label="职位" prop="position">
          <el-input v-model="contactForm.position" placeholder="选填" />
        </el-form-item>
        <el-form-item label="手机" prop="mobile_phone">
          <el-input v-model="contactForm.mobile_phone" placeholder="必填" />
        </el-form-item>
        <el-form-item label="电话" prop="tel_phone">
          <el-input v-model="contactForm.tel_phone" placeholder="选填" />
        </el-form-item>
        <el-form-item label="邮箱" prop="email">
          <el-input v-model="contactForm.email" placeholder="选填" />
        </el-form-item>
        <el-form-item label="微信" prop="wechat">
          <el-input v-model="contactForm.wechat" placeholder="选填" />
        </el-form-item>
        <el-form-item label="QQ" prop="qq">
          <el-input v-model="contactForm.qq" placeholder="选填" />
        </el-form-item>
        <el-form-item label="主要联系人" prop="is_primary">
          <el-switch v-model="contactForm.is_primary" />
        </el-form-item>
        <el-form-item label="备注" prop="remarks">
          <el-input v-model="contactForm.remarks" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="contactDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="contactSubmitting" @click="submitContact">
          确定
        </el-button>
      </template>
    </el-dialog>

    <!-- 新增资质弹窗 -->
    <el-dialog
      v-model="qualificationDialogVisible"
      title="新增资质"
      width="520px"
      @close="resetQualificationForm"
    >
      <el-form
        ref="qualificationFormRef"
        :model="qualificationForm"
        :rules="qualificationRules"
        label-width="110px"
      >
        <el-form-item label="资质名称" prop="qualification_name">
          <el-input v-model="qualificationForm.qualification_name" placeholder="必填" />
        </el-form-item>
        <el-form-item label="资质类型" prop="qualification_type">
          <el-input v-model="qualificationForm.qualification_type" placeholder="必填" />
        </el-form-item>
        <el-form-item label="证照编号" prop="qualification_no">
          <el-input v-model="qualificationForm.qualification_no" placeholder="必填" />
        </el-form-item>
        <el-form-item label="发证机关" prop="issuing_authority">
          <el-input v-model="qualificationForm.issuing_authority" placeholder="必填" />
        </el-form-item>
        <el-form-item label="发证日期" prop="issue_date">
          <el-date-picker
            v-model="qualificationForm.issue_date"
            type="date"
            value-format="YYYY-MM-DD"
            placeholder="必填"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="有效期至" prop="valid_until">
          <el-date-picker
            v-model="qualificationForm.valid_until"
            type="date"
            value-format="YYYY-MM-DD"
            placeholder="必填"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="附件路径" prop="attachment_path">
          <el-input v-model="qualificationForm.attachment_path" placeholder="选填" />
        </el-form-item>
        <el-form-item label="需要年检" prop="need_annual_check">
          <el-switch v-model="qualificationForm.need_annual_check" />
        </el-form-item>
        <el-form-item label="年检记录" prop="annual_check_record">
          <el-input v-model="qualificationForm.annual_check_record" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="qualificationDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="qualificationSubmitting" @click="submitQualification">
          确定
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue';
import { ElMessage } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import {
  createSupplierContact,
  createSupplierQualification,
  getSupplierBalance,
  getSupplierById,
  getSupplierContactList,
  getSupplierPurchaseHistory,
  getSupplierQualificationList,
  type PurchaseHistoryItem,
  type Supplier,
  type SupplierBalance,
  type SupplierContact,
  type SupplierContactInput,
  type SupplierQualification,
  type SupplierQualificationInput,
} from '@/api/supplier';

/** 响应解包防御：兼容数组 / { items } / { history } 包装，避免 el-table "r is not iterable" */
const unwrapList = <T,>(payload: unknown): T[] => {
  if (Array.isArray(payload)) return payload as T[];
  const paged = payload as { items?: T[]; history?: T[] } | null;
  return paged?.items ?? paged?.history ?? [];
};

const fmtAmount = (value: number | null | undefined): string =>
  value == null ? '-' : Number(value).toLocaleString('zh-CN', { minimumFractionDigits: 2 });

// ============== 主查询 ==============

const supplierId = ref<number | undefined>(undefined);
const loaded = ref(false);
const pageLoading = ref(false);
const activeTab = ref('contacts');
const supplier = ref<Supplier | null>(null);
const contacts = ref<SupplierContact[]>([]);
const qualifications = ref<SupplierQualification[]>([]);
const balance = ref<SupplierBalance | null>(null);
const historyRows = ref<PurchaseHistoryItem[]>([]);

const contactsLoading = ref(false);
const qualificationsLoading = ref(false);
const historyLoading = ref(false);

const loadSupplierInfo = async () => {
  const id = supplierId.value;
  if (!id) return;
  try {
    const res = await getSupplierById(id);
    supplier.value = res.data ?? null;
  } catch {
    supplier.value = null;
    ElMessage.error('加载供应商信息失败');
  }
};

const loadContacts = async () => {
  const id = supplierId.value;
  if (!id) return;
  contactsLoading.value = true;
  try {
    const res = await getSupplierContactList(id);
    contacts.value = unwrapList<SupplierContact>(res.data);
  } catch {
    ElMessage.error('加载联系人失败');
  } finally {
    contactsLoading.value = false;
  }
};

const loadQualifications = async () => {
  const id = supplierId.value;
  if (!id) return;
  qualificationsLoading.value = true;
  try {
    const res = await getSupplierQualificationList(id);
    qualifications.value = unwrapList<SupplierQualification>(res.data);
  } catch {
    ElMessage.error('加载资质失败');
  } finally {
    qualificationsLoading.value = false;
  }
};

const loadBalance = async () => {
  const id = supplierId.value;
  if (!id) return;
  try {
    const res = await getSupplierBalance(id);
    balance.value = res.data ?? null;
  } catch {
    balance.value = null;
    ElMessage.error('加载供应商余额失败');
  }
};

const loadHistory = async () => {
  const id = supplierId.value;
  if (!id) return;
  historyLoading.value = true;
  try {
    const res = await getSupplierPurchaseHistory(id, { limit: 50 });
    historyRows.value = unwrapList<PurchaseHistoryItem>(res.data);
  } catch {
    ElMessage.error('加载采购历史失败');
  } finally {
    historyLoading.value = false;
  }
};

const loadAll = async () => {
  if (!supplierId.value) {
    ElMessage.warning('请输入供应商 ID');
    return;
  }
  loaded.value = true;
  pageLoading.value = true;
  const tasks = [
    loadSupplierInfo(),
    loadContacts(),
    loadQualifications(),
    loadBalance(),
    loadHistory(),
  ];
  await Promise.all(tasks);
  pageLoading.value = false;
};

// ============== 新增联系人 ==============

const contactDialogVisible = ref(false);
const contactSubmitting = ref(false);
const contactFormRef = ref<FormInstance>();

const contactForm = reactive<SupplierContactInput>({
  contact_name: '',
  department: '',
  position: '',
  mobile_phone: '',
  tel_phone: '',
  email: '',
  wechat: '',
  qq: '',
  is_primary: false,
  remarks: '',
});

const contactRules: FormRules = {
  contact_name: [{ required: true, message: '请输入姓名', trigger: 'blur' }],
  mobile_phone: [{ required: true, message: '请输入手机号', trigger: 'blur' }],
};

const resetContactForm = () => {
  contactForm.contact_name = '';
  contactForm.department = '';
  contactForm.position = '';
  contactForm.mobile_phone = '';
  contactForm.tel_phone = '';
  contactForm.email = '';
  contactForm.wechat = '';
  contactForm.qq = '';
  contactForm.is_primary = false;
  contactForm.remarks = '';
  contactFormRef.value?.resetFields();
};

const openContactDialog = () => {
  resetContactForm();
  contactDialogVisible.value = true;
};

const submitContact = async () => {
  const id = supplierId.value;
  if (!id) {
    ElMessage.warning('请先输入供应商 ID 并查询');
    return;
  }
  if (!contactFormRef.value) return;
  await contactFormRef.value.validate(async valid => {
    if (!valid) return;
    contactSubmitting.value = true;
    try {
      await createSupplierContact(id, {
        contact_name: contactForm.contact_name,
        department: contactForm.department || undefined,
        position: contactForm.position || undefined,
        mobile_phone: contactForm.mobile_phone,
        tel_phone: contactForm.tel_phone || undefined,
        email: contactForm.email || undefined,
        wechat: contactForm.wechat || undefined,
        qq: contactForm.qq || undefined,
        is_primary: contactForm.is_primary,
        remarks: contactForm.remarks || undefined,
      });
      ElMessage.success('联系人创建成功');
      contactDialogVisible.value = false;
      await loadContacts();
    } catch {
      ElMessage.error('联系人创建失败');
    } finally {
      contactSubmitting.value = false;
    }
  });
};

// ============== 新增资质 ==============

const qualificationDialogVisible = ref(false);
const qualificationSubmitting = ref(false);
const qualificationFormRef = ref<FormInstance>();

const qualificationForm = reactive<SupplierQualificationInput>({
  qualification_name: '',
  qualification_type: '',
  qualification_no: '',
  issuing_authority: '',
  issue_date: '',
  valid_until: '',
  attachment_path: '',
  need_annual_check: false,
  annual_check_record: '',
});

const qualificationRules: FormRules = {
  qualification_name: [{ required: true, message: '请输入资质名称', trigger: 'blur' }],
  qualification_type: [{ required: true, message: '请输入资质类型', trigger: 'blur' }],
  qualification_no: [{ required: true, message: '请输入证照编号', trigger: 'blur' }],
  issuing_authority: [{ required: true, message: '请输入发证机关', trigger: 'blur' }],
  issue_date: [{ required: true, message: '请选择发证日期', trigger: 'change' }],
  valid_until: [{ required: true, message: '请选择有效期至', trigger: 'change' }],
};

const resetQualificationForm = () => {
  qualificationForm.qualification_name = '';
  qualificationForm.qualification_type = '';
  qualificationForm.qualification_no = '';
  qualificationForm.issuing_authority = '';
  qualificationForm.issue_date = '';
  qualificationForm.valid_until = '';
  qualificationForm.attachment_path = '';
  qualificationForm.need_annual_check = false;
  qualificationForm.annual_check_record = '';
  qualificationFormRef.value?.resetFields();
};

const openQualificationDialog = () => {
  resetQualificationForm();
  qualificationDialogVisible.value = true;
};

const submitQualification = async () => {
  const id = supplierId.value;
  if (!id) {
    ElMessage.warning('请先输入供应商 ID 并查询');
    return;
  }
  if (!qualificationFormRef.value) return;
  await qualificationFormRef.value.validate(async valid => {
    if (!valid) return;
    qualificationSubmitting.value = true;
    try {
      await createSupplierQualification(id, {
        qualification_name: qualificationForm.qualification_name,
        qualification_type: qualificationForm.qualification_type,
        qualification_no: qualificationForm.qualification_no,
        issuing_authority: qualificationForm.issuing_authority,
        issue_date: qualificationForm.issue_date,
        valid_until: qualificationForm.valid_until,
        attachment_path: qualificationForm.attachment_path || undefined,
        need_annual_check: qualificationForm.need_annual_check,
        annual_check_record: qualificationForm.annual_check_record || undefined,
      });
      ElMessage.success('资质创建成功');
      qualificationDialogVisible.value = false;
      await loadQualifications();
    } catch {
      ElMessage.error('资质创建失败');
    } finally {
      qualificationSubmitting.value = false;
    }
  });
};
</script>

<style scoped>
.supplier-enhanced-page {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.page-header h2 {
  margin: 0;
  font-size: 18px;
}

.header-actions {
  display: flex;
  gap: 12px;
  align-items: center;
}

.section-title {
  margin: 20px 0 12px;
  font-size: 15px;
  font-weight: 600;
}

.toolbar {
  display: flex;
  gap: 12px;
  align-items: center;
  margin-bottom: 12px;
}

.block-gap {
  margin-bottom: 16px;
}
</style>
