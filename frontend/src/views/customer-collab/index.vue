<template>
  <div class="page">
    <el-tabs v-model="activeTab" type="border-card">
      <!-- 合同签署 -->
      <el-tab-pane label="合同签署" name="signature">
        <div class="toolbar mb">
          <el-button type="primary" @click="signVisible = true">签署合同</el-button>
        </div>
        <el-table v-loading="loadingSig" :data="signatures" border>
          <el-table-column prop="id" label="ID" width="70" />
          <template v-for="col in sigCols" :key="col">
            <el-table-column
              :prop="col"
              :label="col.replace(/_/g, ' ')"
              min-width="150"
              show-overflow-tooltip
            />
          </template>
          <el-table-column label="操作" width="180" fixed="right">
            <template #default="{ row }">
              <el-button size="small" type="primary" plain @click="onVerify(row)">验证</el-button>
              <el-button size="small" type="danger" plain @click="onRevoke(row)">撤销</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <!-- 客户共享 -->
      <el-tab-pane label="客户共享" name="share">
        <div class="toolbar mb">
          <el-button type="primary" @click="shareVisible = true">共享客户</el-button>
          <el-button plain @click="onExpireOverdue">清理过期共享</el-button>
          <el-button plain @click="onSharesByCustomer">按客户查共享</el-button>
          <el-button plain @click="onSharesByUser">按用户查共享</el-button>
          <el-button plain @click="onCheckSharePermission">共享权限检查</el-button>
        </div>
        <el-table v-loading="loadingShare" :data="shares" border>
          <el-table-column prop="id" label="ID" width="70" />
          <template v-for="col in shareCols" :key="col">
            <el-table-column
              :prop="col"
              :label="col.replace(/_/g, ' ')"
              min-width="150"
              show-overflow-tooltip
            />
          </template>
          <el-table-column label="操作" width="120" fixed="right">
            <template #default="{ row }">
              <el-button size="small" type="danger" plain @click="onRevokeShare(row)"
                >撤销</el-button
              >
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <!-- 客户团队 -->
      <el-tab-pane label="客户团队" name="team">
        <el-form ref="teamQueryRef" :model="teamQuery" :rules="teamQueryRules" inline class="mb">
          <el-form-item label="客户ID" prop="customer_id"
            ><el-input v-model="teamQuery.customer_id" inputmode="numeric" placeholder="客户ID"
          /></el-form-item>
          <el-form-item
            ><el-button type="primary" plain @click="onLoadTeam">查询团队</el-button></el-form-item
          >
          <el-form-item
            ><el-button plain @click="teamVisible = true">添加成员</el-button></el-form-item
          >
          <el-form-item label="用户ID" prop="user_id"
            ><el-input v-model="teamQuery.user_id" inputmode="numeric" placeholder="用户ID"
          /></el-form-item>
          <el-form-item
            ><el-button plain @click="onListUserTeams">按用户查团队</el-button></el-form-item
          >
          <el-form-item
            ><el-button plain @click="onCheckTeamMember">成员校验</el-button></el-form-item
          >
        </el-form>
        <el-table :data="teamMembers" border>
          <el-table-column prop="id" label="ID" width="70" />
          <template v-for="col in teamCols" :key="col">
            <el-table-column
              :prop="col"
              :label="col.replace(/_/g, ' ')"
              min-width="150"
              show-overflow-tooltip
            />
          </template>
          <el-table-column label="操作" width="120" fixed="right">
            <template #default="{ row }">
              <el-button size="small" type="danger" plain @click="onRemoveMember(row)"
                >移除</el-button
              >
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>
    </el-tabs>

    <el-dialog v-model="signVisible" title="签署合同" width="480">
      <el-form ref="signFormRef" :model="signForm" :rules="signRules" label-width="110px">
        <el-form-item label="合同类型" required>
          <el-select v-model="signForm.contract_type" class="w-full">
            <el-option
              :label="t('customerCollab.contractTypes.purchaseContract')"
              value="purchase_contract"
            />
            <el-option
              :label="t('customerCollab.contractTypes.salesContract')"
              value="sales_contract"
            />
            <el-option
              :label="t('customerCollab.contractTypes.outsourcingContract')"
              value="outsourcing_contract"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="合同ID" prop="contract_id" required
          ><el-input
            v-model="signForm.contract_id"
            inputmode="numeric"
            class="w-full"
        /></el-form-item>
        <el-form-item label="签署人ID" prop="signer_id" required
          ><el-input
            v-model="signForm.signer_id"
            inputmode="numeric"
            class="w-full"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="signVisible = false">取消</el-button>
        <el-button type="primary" @click="onSign">签署</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="shareVisible" title="共享客户" width="480">
      <el-form ref="shareFormRef" :model="shareForm" :rules="shareRules" label-width="110px">
        <el-form-item label="客户ID" prop="customer_id" required
          ><el-input
            v-model="shareForm.customer_id"
            inputmode="numeric"
            class="w-full"
        /></el-form-item>
        <el-form-item :label="t('crmCustomerShare.sharedToUserId')" prop="shared_to_user_id" required
          ><el-input
            v-model="shareForm.shared_to_user_id"
            inputmode="numeric"
            class="w-full"
        /></el-form-item>
        <el-form-item label="权限" required>
          <!-- 后端词表唯一真相：customer_share.rs SHARE_PERMISSION_VIEW/EDIT/FULL = view/edit/full，
               服务层 validate_share_permission_type 强校验（旧值 read/write 必 422） -->
          <el-select v-model="shareForm.permission" class="w-full">
            <el-option :label="t('crmCustomerShare.permissionView')" value="view" />
            <el-option :label="t('crmCustomerShare.permissionEdit')" value="edit" />
            <el-option :label="t('crmCustomerShare.permissionFull')" value="full" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="shareVisible = false">取消</el-button>
        <el-button type="primary" @click="onShare">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="teamVisible" title="添加团队成员" width="460">
      <el-form ref="teamFormRef" :model="teamForm" :rules="teamFormRules" label-width="110px">
        <el-form-item label="客户ID" prop="customer_id" required
          ><el-input
            v-model="teamForm.customer_id"
            inputmode="numeric"
            class="w-full"
        /></el-form-item>
        <el-form-item label="用户ID" prop="user_id" required
          ><el-input v-model="teamForm.user_id" inputmode="numeric" class="w-full"
        /></el-form-item>
        <el-form-item label="角色"
          ><el-input v-model="teamForm.role" placeholder="owner / member"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="teamVisible = false">取消</el-button>
        <el-button type="primary" @click="onAddMember">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { isDialogDismissal } from '@/utils/monitor';
import { computed, onMounted, reactive, ref } from 'vue';
import { logAuxLoadFailure } from '@/utils/logger';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';
import { numberInputRules } from '@/utils/number-input';
import {
  addTeamMember,
  createCustomerShare,
  expireOverdueShares,
  getSharesByCustomer,
  getSharesByUser,
  checkSharePermission,
  isTeamMember,
  listUserTeams,
  listCustomerShares,
  listSignedContracts,
  listTeamMembers,
  removeTeamMember,
  revokeShare,
  revokeSignature,
  signContract,
  verifySignature,
} from '@/api/customer-share';

const { t } = useI18n({ useScope: 'global' });

const activeTab = ref('signature');
const unwrapList = <T,>(p: unknown): T[] => (p as { data: T[] }).data;
const cols = (rows: Array<Record<string, unknown>>, skip: string[], n: number) =>
  rows.length
    ? Object.keys(rows[0])
        .filter(k => !skip.includes(k) && typeof rows[0][k] !== 'object')
        .slice(0, n)
    : [];

// 签署
const signatures = ref<Array<Record<string, unknown>>>([]);
const sigCols = ref<string[]>([]);
const loadingSig = ref(false);
const signVisible = ref(false);
// 手写录入文本模型（ID 为正整数，范围靠 numberInputRules 单源校验器，不靠步进按钮）；
// 提交前显式 Number() 归一为契约数值
const signForm = reactive({
  contract_type: 'sales_contract',
  contract_id: '',
  signer_id: '',
});
const signFormRef = ref<FormInstance>();
const signRules = computed<FormRules>(() => ({
  contract_id: numberInputRules({ label: '合同ID', required: true, min: 1, integer: true }),
  signer_id: numberInputRules({ label: '签署人ID', required: true, min: 1, integer: true }),
}));

async function loadSignatures() {
  loadingSig.value = true;
  try {
    signatures.value = unwrapList(await listSignedContracts());
    sigCols.value = cols(signatures.value, ['id'], 6);
  } catch (e: unknown) {
    logAuxLoadFailure('合同签署列表加载失败', e);
  } finally {
    loadingSig.value = false;
  }
}

async function onSign() {
  if (signForm.contract_id.trim() === '' || signForm.signer_id.trim() === '') {
    ElMessage.warning('请填写合同与签署人');
    return;
  }
  let valid = false;
  await signFormRef.value?.validate((ok) => {
    valid = ok;
  });
  if (!valid) return;
  await signContract({
    contract_type: signForm.contract_type,
    contract_id: Number(signForm.contract_id),
    signer_id: Number(signForm.signer_id),
  });
  ElMessage.success('签署成功');
  signVisible.value = false;
  await loadSignatures();
}

async function onVerify(row: Record<string, unknown>) {
  const res = await verifySignature(row.id as number);
  ElMessageBox.alert(JSON.stringify(res, null, 2), `签名 #${row.id} 验证结果`);
}

async function onRevoke(row: Record<string, unknown>) {
  await ElMessageBox.confirm('确认撤销该签名？', '确认');
  await revokeSignature(row.id as number);
  ElMessage.success('已撤销');
  await loadSignatures();
}

// 共享
const shares = ref<Array<Record<string, unknown>>>([]);
const shareCols = ref<string[]>([]);
const loadingShare = ref(false);
const shareVisible = ref(false);
// 共享权限词表唯一真相 = 后端 validate_share_permission_type
// （services/crm/customer_team_share_service.rs:677-687，仅 view/edit/full；
//  旧值 read/write 必 422）。源保持 string 以便 el-select v-model 回写，
//  提交前用 isSharePermission 显式校验收窄为字面量联合，禁止裸断言。
const SHARE_PERMISSIONS = ['view', 'edit', 'full'] as const;
type SharePermission = (typeof SHARE_PERMISSIONS)[number];
const isSharePermission = (v: string): v is SharePermission =>
  (SHARE_PERMISSIONS as readonly string[]).includes(v);

// 手写录入文本模型（ID 为正整数，范围靠 numberInputRules 单源校验器）；提交前 Number() 归一
const shareForm = reactive({
  customer_id: '',
  // 后端 ShareCustomerRequest.shared_to_user_id（i32 非 Option，必填；
  //  customer_team_share_service.rs:57）；前端旧键名 target_user_id 不在契约内。
  shared_to_user_id: '',
  // 默认值须落在后端词表内（旧值 read 会被 422 拒绝）；后端缺省口径同为 view。
  permission: 'view',
});
const shareFormRef = ref<FormInstance>();
const shareRules = computed<FormRules>(() => ({
  customer_id: numberInputRules({ label: '客户ID', required: true, min: 1, integer: true }),
  shared_to_user_id: numberInputRules({
    label: t('crmCustomerShare.sharedToUserId'),
    required: true,
    min: 1,
    integer: true,
  }),
}));

async function loadShares() {
  loadingShare.value = true;
  try {
    shares.value = unwrapList(await listCustomerShares());
    shareCols.value = cols(shares.value, ['id'], 6);
  } catch (e: unknown) {
    logAuxLoadFailure('客户共享列表加载失败', e);
  } finally {
    loadingShare.value = false;
  }
}

async function onShare() {
  if (shareForm.customer_id.trim() === '' || shareForm.shared_to_user_id.trim() === '') {
    ElMessage.warning(t('crmCustomerShare.fillCustomerAndUser'));
    return;
  }
  let valid = false;
  await shareFormRef.value?.validate((ok) => {
    valid = ok;
  });
  if (!valid) return;
  // 权限经显式类型守卫收窄为 SharePermission 后再赋值，匹配 CreateCustomerShareInput，
  // 不做 as any/裸断言（后端 read/write 会 422）。
  const permission = shareForm.permission;
  if (!isSharePermission(permission)) {
    ElMessage.warning(t('crmCustomerShare.invalidPermission'));
    return;
  }
  try {
    await createCustomerShare({
      customer_id: Number(shareForm.customer_id),
      shared_to_user_id: Number(shareForm.shared_to_user_id),
      permission,
    });
    ElMessage.success(t('crmCustomerShare.shared'));
    shareVisible.value = false;
    await loadShares();
  } catch (e) {
    ElMessage.error((e as Error).message || t('common.failed'));
  }
}

async function onRevokeShare(row: Record<string, unknown>) {
  await ElMessageBox.confirm('确认撤销该共享？', '确认');
  await revokeShare(row.id as number);
  ElMessage.success('已撤销');
  await loadShares();
}

async function onExpireOverdue() {
  const res = await expireOverdueShares();
  ElMessage.success(`清理完成：${JSON.stringify(res).slice(0, 60)}`);
  await loadShares();
}

// 团队
const teamMembers = ref<Array<Record<string, unknown>>>([]);
const teamCols = ref<string[]>([]);
// 工具栏查询条件手写录入文本模型（正整数由 numberInputRules 单源校验，消费点 Number() 归一）
const teamQuery = reactive({ customer_id: '', user_id: '' });
const teamQueryRef = ref<FormInstance>();
const teamQueryRules = computed<FormRules>(() => ({
  customer_id: numberInputRules({ label: '客户ID', min: 1, integer: true }),
  user_id: numberInputRules({ label: '用户ID', min: 1, integer: true }),
}));
const teamVisible = ref(false);
const teamForm = reactive({
  customer_id: '',
  user_id: '',
  role: 'member',
});
const teamFormRef = ref<FormInstance>();
const teamFormRules = computed<FormRules>(() => ({
  customer_id: numberInputRules({ label: '客户ID', required: true, min: 1, integer: true }),
  user_id: numberInputRules({ label: '用户ID', required: true, min: 1, integer: true }),
}));

async function onLoadTeam() {
  if (teamQuery.customer_id.trim() === '') {
    ElMessage.warning('请填写客户ID');
    return;
  }
  const valid = await validateTeamQuery();
  if (!valid) return;
  teamMembers.value = unwrapList(await listTeamMembers(Number(teamQuery.customer_id)));
  teamCols.value = cols(teamMembers.value, ['id'], 6);
}

/** 工具栏查询表单级校验（数字范围与整数性由单源校验器判定），失败即拦截动作 */
async function validateTeamQuery(): Promise<boolean> {
  let ok = false;
  await teamQueryRef.value?.validate((valid) => {
    ok = valid;
  });
  return ok;
}

async function onAddMember() {
  if (teamForm.customer_id.trim() === '' || teamForm.user_id.trim() === '') {
    ElMessage.warning('请填写客户与用户');
    return;
  }
  let valid = false;
  await teamFormRef.value?.validate((ok) => {
    valid = ok;
  });
  if (!valid) return;
  await addTeamMember({
    customer_id: Number(teamForm.customer_id),
    user_id: Number(teamForm.user_id),
    role: teamForm.role,
  });
  ElMessage.success('成员已添加');
  teamVisible.value = false;
  teamQuery.customer_id = teamForm.customer_id;
  await onLoadTeam();
}

async function onRemoveMember(row: Record<string, unknown>) {
  await ElMessageBox.confirm('确认移除该成员？', '确认');
  await removeTeamMember(row.id as number);
  ElMessage.success('已移除');
  await onLoadTeam();
}

// ===== 共享/团队查询与校验工具（getSharesByCustomer/getSharesByUser/checkSharePermission/isTeamMember/listUserTeams） =====
async function onSharesByCustomer() {
  try {
    const { value } = await ElMessageBox.prompt('请输入客户ID', '按客户查共享', {
      inputPattern: /^\d+$/,
      inputErrorMessage: '请输入数字ID',
    });
    const res = await getSharesByCustomer(Number(value));
    shares.value = unwrapList(res);
    ElMessage.success(`客户 ${value} 的共享已加载`);
  } catch (e) {
    if (!isDialogDismissal(e)) ElMessage.error((e as Error).message || '查询失败');
  }
}

async function onSharesByUser() {
  try {
    const { value } = await ElMessageBox.prompt('请输入用户ID', '按用户查共享', {
      inputPattern: /^\d+$/,
      inputErrorMessage: '请输入数字ID',
    });
    const res = await getSharesByUser(Number(value));
    shares.value = unwrapList(res);
    ElMessage.success(`用户 ${value} 的共享已加载`);
  } catch (e) {
    if (!isDialogDismissal(e)) ElMessage.error((e as Error).message || '查询失败');
  }
}

async function onCheckSharePermission() {
  try {
    const { value } = await ElMessageBox.prompt(
      '请输入检查参数 JSON，如 {"customer_id":1,"user_id":2}',
      '共享权限检查',
      { inputValue: '{"customer_id":1,"user_id":2}' }
    );
    let params: Record<string, unknown>;
    try {
      params = JSON.parse(value);
    } catch {
      ElMessage.warning('JSON 格式有误');
      return;
    }
    const res = await checkSharePermission(params);
    ElMessageBox.alert(JSON.stringify(res.data ?? res, null, 2), '权限检查结果');
  } catch (e) {
    if (!isDialogDismissal(e)) ElMessage.error((e as Error).message || '检查失败');
  }
}

async function onListUserTeams() {
  if (teamQuery.user_id.trim() === '') {
    ElMessage.warning('请填写用户ID');
    return;
  }
  const valid = await validateTeamQuery();
  if (!valid) return;
  try {
    const res = await listUserTeams(Number(teamQuery.user_id));
    teamMembers.value = unwrapList(res);
    teamCols.value = cols(teamMembers.value, ['id'], 6);
    ElMessage.success(`用户 ${teamQuery.user_id} 的团队已加载`);
  } catch (e) {
    ElMessage.error((e as Error).message || '查询失败');
  }
}

async function onCheckTeamMember() {
  if (teamQuery.customer_id.trim() === '' || teamQuery.user_id.trim() === '') {
    ElMessage.warning('请填写客户ID与用户ID');
    return;
  }
  const valid = await validateTeamQuery();
  if (!valid) return;
  try {
    const res = await isTeamMember({
      customer_id: Number(teamQuery.customer_id),
      user_id: Number(teamQuery.user_id),
    });
    const d = res.data as { is_member?: boolean } | undefined;
    ElMessageBox.alert(
      d?.is_member ? '该用户是此客户团队成员' : '该用户不是此客户团队成员',
      '成员校验结果'
    );
  } catch (e) {
    ElMessage.error((e as Error).message || '校验失败');
  }
}

onMounted(() => {
  loadSignatures();
  loadShares();
});
</script>

<style scoped>
.mb {
  margin-bottom: 12px;
}
.toolbar {
  display: flex;
  gap: 8px;
}
.w-full {
  width: 100%;
}
</style>
