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
        <el-form inline class="mb">
          <el-form-item label="客户ID"
            ><el-input-number v-model="teamCustomerId" :min="1"
          /></el-form-item>
          <el-form-item
            ><el-button type="primary" plain @click="onLoadTeam">查询团队</el-button></el-form-item
          >
          <el-form-item
            ><el-button plain @click="teamVisible = true">添加成员</el-button></el-form-item
          >
          <el-form-item label="用户ID"
            ><el-input-number v-model="teamUserId" :min="1"
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
      <el-form :model="signForm" label-width="110px">
        <el-form-item label="合同类型" required>
          <el-select v-model="signForm.contract_type" class="w-full">
            <el-option
              v-for="t in ['purchase_contract', 'sales_contract', 'outsourcing_contract']"
              :key="t"
              :label="t"
              :value="t"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="合同ID" required
          ><el-input-number v-model="signForm.contract_id" :min="1" class="w-full"
        /></el-form-item>
        <el-form-item label="签署人ID" required
          ><el-input-number v-model="signForm.signer_id" :min="1" class="w-full"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="signVisible = false">取消</el-button>
        <el-button type="primary" @click="onSign">签署</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="shareVisible" title="共享客户" width="480">
      <el-form :model="shareForm" label-width="110px">
        <el-form-item label="客户ID" required
          ><el-input-number v-model="shareForm.customer_id" :min="1" class="w-full"
        /></el-form-item>
        <el-form-item label="目标用户ID" required
          ><el-input-number v-model="shareForm.target_user_id" :min="1" class="w-full"
        /></el-form-item>
        <el-form-item label="权限" required>
          <el-select v-model="shareForm.permission" class="w-full">
            <el-option label="只读" value="read" />
            <el-option label="读写" value="write" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="shareVisible = false">取消</el-button>
        <el-button type="primary" @click="onShare">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="teamVisible" title="添加团队成员" width="460">
      <el-form :model="teamForm" label-width="110px">
        <el-form-item label="客户ID" required
          ><el-input-number v-model="teamForm.customer_id" :min="1" class="w-full"
        /></el-form-item>
        <el-form-item label="用户ID" required
          ><el-input-number v-model="teamForm.user_id" :min="1" class="w-full"
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
import { onMounted, reactive, ref } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
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
const signForm = reactive({
  contract_type: 'sales_contract',
  contract_id: undefined as number | undefined,
  signer_id: undefined as number | undefined,
});

async function loadSignatures() {
  loadingSig.value = true;
  try {
    signatures.value = unwrapList(await listSignedContracts());
    sigCols.value = cols(signatures.value, ['id'], 6);
  } finally {
    loadingSig.value = false;
  }
}

async function onSign() {
  if (!signForm.contract_id || !signForm.signer_id) {
    ElMessage.warning('请填写合同与签署人');
    return;
  }
  await signContract({ ...signForm });
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
const shareForm = reactive({
  customer_id: undefined as number | undefined,
  target_user_id: undefined as number | undefined,
  permission: 'read',
});

async function loadShares() {
  loadingShare.value = true;
  try {
    shares.value = unwrapList(await listCustomerShares());
    shareCols.value = cols(shares.value, ['id'], 6);
  } finally {
    loadingShare.value = false;
  }
}

async function onShare() {
  if (!shareForm.customer_id || !shareForm.target_user_id) {
    ElMessage.warning('请填写客户与目标用户');
    return;
  }
  await createCustomerShare({
    customer_id: shareForm.customer_id,
    target_user_id: shareForm.target_user_id,
    permission: shareForm.permission,
  });
  ElMessage.success('已共享');
  shareVisible.value = false;
  await loadShares();
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
const teamCustomerId = ref<number | undefined>();
const teamVisible = ref(false);
const teamForm = reactive({
  customer_id: undefined as number | undefined,
  user_id: undefined as number | undefined,
  role: 'member',
});

async function onLoadTeam() {
  if (!teamCustomerId.value) {
    ElMessage.warning('请填写客户ID');
    return;
  }
  teamMembers.value = unwrapList(await listTeamMembers(teamCustomerId.value));
  teamCols.value = cols(teamMembers.value, ['id'], 6);
}

async function onAddMember() {
  if (!teamForm.customer_id || !teamForm.user_id) {
    ElMessage.warning('请填写客户与用户');
    return;
  }
  await addTeamMember({ ...teamForm });
  ElMessage.success('成员已添加');
  teamVisible.value = false;
  teamCustomerId.value = teamForm.customer_id;
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
    if (e !== 'cancel') ElMessage.error((e as Error).message || '查询失败');
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
    if (e !== 'cancel') ElMessage.error((e as Error).message || '查询失败');
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
    if (e !== 'cancel') ElMessage.error((e as Error).message || '检查失败');
  }
}

const teamUserId = ref<number | undefined>();

async function onListUserTeams() {
  if (!teamUserId.value) {
    ElMessage.warning('请填写用户ID');
    return;
  }
  try {
    const res = await listUserTeams(teamUserId.value);
    teamMembers.value = unwrapList(res);
    teamCols.value = cols(teamMembers.value, ['id'], 6);
    ElMessage.success(`用户 ${teamUserId.value} 的团队已加载`);
  } catch (e) {
    ElMessage.error((e as Error).message || '查询失败');
  }
}

async function onCheckTeamMember() {
  if (!teamCustomerId.value || !teamUserId.value) {
    ElMessage.warning('请填写客户ID与用户ID');
    return;
  }
  try {
    const res = await isTeamMember({
      customer_id: teamCustomerId.value,
      user_id: teamUserId.value,
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
