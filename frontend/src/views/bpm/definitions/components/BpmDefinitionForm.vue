<!--
  BpmDefinitionForm.vue - BPM 流程定义新建/编辑对话框（含节点配置子表）
  拆分自 bpm/definitions.vue（P14 批 2 I-3 第 5 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  注意：formData 含 nodes 数组，需要深拷贝以保证本地修改与父组件解耦
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="isEdit ? t('bpm.definitions.form.editTitle') : t('bpm.definitions.form.createTitle')"
    :aria-label="
      isEdit ? t('bpm.definitions.form.editAriaLabel') : t('bpm.definitions.form.createAriaLabel')
    "
    width="900px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form
      ref="formRef"
      :model="localFormData"
      :rules="rules"
      label-width="100px"
      :aria-label="t('bpm.definitions.form.ariaLabel')"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('bpm.definitions.form.processKey')" prop="process_key">
            <el-input
              v-model="localFormData.process_key"
              :disabled="isEdit"
              :placeholder="t('bpm.definitions.form.processKeyPlaceholder')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('bpm.definitions.form.processName')" prop="process_name">
            <el-input
              v-model="localFormData.process_name"
              :placeholder="t('bpm.definitions.form.processNamePlaceholder')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('bpm.definitions.form.category')" prop="category">
            <el-select
              v-model="localFormData.category"
              :placeholder="t('bpm.definitions.form.categoryPlaceholder')"
              style="width: 100%"
            >
              <el-option :label="t('bpm.definitions.category.finance')" value="finance" />
              <el-option :label="t('bpm.definitions.category.hr')" value="hr" />
              <el-option :label="t('bpm.definitions.category.purchase')" value="purchase" />
              <el-option :label="t('bpm.definitions.category.sales')" value="sales" />
              <el-option :label="t('bpm.definitions.category.production')" value="production" />
              <el-option :label="t('bpm.definitions.category.inventory')" value="inventory" />
              <el-option :label="t('bpm.definitions.category.other')" value="other" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('bpm.definitions.form.description')">
        <el-input
          v-model="localFormData.description"
          type="textarea"
          :rows="3"
          :placeholder="t('bpm.definitions.form.descriptionPlaceholder')"
        />
      </el-form-item>

      <!-- 节点配置 -->
      <el-divider content-position="left">{{ t('bpm.definitions.form.nodeConfig') }}</el-divider>
      <!-- 本表单无连线交互，edges 按节点顺序线性生成，界面顺序即审批顺序 -->
      <p class="node-order-hint">{{ t('bpm.definitions.form.nodeOrderHint') }}</p>
      <!-- 审批人候选按关键字远程检索，未搜索时仅第一页，界面如实声明非全集 -->
      <p class="node-order-hint">{{ t('bpm.definitions.form.assigneeHint') }}</p>
      <div class="node-actions">
        <el-button type="primary" size="small" @click="handleAddNode">
          <el-icon><Plus /></el-icon>
          {{ t('bpm.definitions.form.addNode') }}
        </el-button>
      </div>
      <el-table
        :data="localFormData.nodes"
        border
        :aria-label="t('bpm.definitions.form.nodeTableAriaLabel')"
      >
        <el-table-column :label="t('bpm.definitions.form.nodeType')" width="120">
          <template #default="{ row }">
            <!-- value 为审批引擎逐字符识别的节点类型 token（后端 resolve_first_task_node
                 与 try_advance_to_next_node 只识别这三个值）；界面中文名走 i18n，不外泄为提交值 -->
            <el-select v-model="row.type" size="small" @change="handleNodeTypeChange(row)">
              <el-option :label="t('bpm.definitions.nodeType.startEvent')" value="start_event" />
              <el-option :label="t('bpm.definitions.nodeType.userTask')" value="user_task" />
              <el-option :label="t('bpm.definitions.nodeType.endEvent')" value="end_event" />
            </el-select>
          </template>
        </el-table-column>
        <el-table-column :label="t('bpm.definitions.form.nodeName')" min-width="140">
          <template #default="{ row }">
            <el-input v-model="row.name" size="small" />
          </template>
        </el-table-column>
        <el-table-column :label="t('bpm.definitions.form.assignee')" min-width="220">
          <template #default="{ row }">
            <!-- 引擎只支持指定具体用户，且 assignee_value 须解析为正整数用户 ID
                 （后端 resolve_node_assignee），因此不提供审批人类型选项、不允许手填 ID：
                 用户 ID 由可搜索选择带出。候选按 keyword 远程搜索——后端列出用户的
                 page_size 有 clamp(1,100)，大 page_size 会被静默截断，
                 靠 keyword 收窄而非凑全量（见 composables/useAssigneeSearch.ts）。
                 候选数组按行 id 各自持有，避免多行下拉互相串数据。 -->
            <el-select
              v-model="row.assignee_value"
              size="small"
              filterable
              clearable
              remote
              :remote-method="(q: string) => searchAssignees(row, q)"
              :loading="rowLoading[row.id] === true"
              :disabled="row.type !== 'user_task'"
              :placeholder="t('bpm.definitions.form.assigneePlaceholder')"
              @visible-change="
                (v: boolean) => v && rowCandidates[row.id] === undefined && searchAssignees(row, '')
              "
            >
              <el-option
                v-for="u in rowCandidates[row.id] ?? []"
                :key="u.id"
                :label="u.real_name ? `${u.real_name}（${u.username}）` : u.username"
                :value="u.id"
              />
            </el-select>
          </template>
        </el-table-column>
        <el-table-column
          :label="t('bpm.definitions.form.operation')"
          width="80"
          align="center"
          fixed="right"
        >
          <template #default="{ $index }">
            <el-button type="danger" link size="small" @click="handleRemoveNode($index)">{{
              t('bpm.definitions.form.delete')
            }}</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">{{
        t('bpm.definitions.form.cancel')
      }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
        t('bpm.definitions.form.confirm')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { deepClone } from '@/utils';
import { ref, watch, nextTick } from 'vue';
import { ElMessage, type FormInstance, type FormRules } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import { useI18n } from 'vue-i18n';
import type { ProcessNode } from '@/api/bpm-enhanced';
import type { User } from '@/api/user';
import { fetchUserCandidates } from '../composables/useAssigneeSearch';
import { logAuxLoadFailure } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

// 表单数据类型
interface BpmDefinitionFormData {
  id?: number;
  process_key: string;
  process_name: string;
  description: string;
  category: string;
  nodes: ProcessNode[];
}

/**
 * 表单组件（含节点配置子表）
 */
const props = defineProps<{
  // 可见性
  visible: boolean;
  // 是否编辑
  isEdit: boolean;
  // 表单数据（由父组件管理，子组件通过 emit 回写）
  formData: BpmDefinitionFormData;
  // 验证规则
  rules: FormRules;
  // 提交加载
  submitLoading: boolean;
}>();

const emit = defineEmits<{
  'update:visible': [v: boolean];
  // 添加节点
  'add-node': [];
  // 删除节点
  'remove-node': [index: number];
  // 整体回写表单数据（父组件监听此事件并 Object.assign 到自己的 formData）
  'update:formData': [v: BpmDefinitionFormData];
  // 提交（父组件处理 API）
  submit: [];
}>();

// 表单 ref
const formRef = ref<FormInstance>();

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
// 注意：formData 含 nodes 数组，需要深拷贝以保证本地修改与父组件解耦
const localFormData = ref<BpmDefinitionFormData>(deepClone(props.formData));

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false;

// 外部 prop 变化时同步到 local（如父组件打开新建/编辑时填充数据）
watch(
  () => props.formData,
  newData => {
    if (syncing) return;
    syncing = true;
    localFormData.value = deepClone(newData);
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);

// 本地变化时通知父组件（用户输入）
watch(
  localFormData,
  newData => {
    if (syncing) return;
    syncing = true;
    emit('update:formData', deepClone(newData));
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);

// 暴露给父组件
defineExpose({ formRef });

/**
 * 审批人候选按行独立持有（键 = 节点 id）。多行下拉若共享同一候选数组，
 * 某一行的搜索会改写另一行的可见项（串数据），因此每行各自检索、各自存放。
 */
const rowCandidates = ref<Record<string, User[]>>({});
const rowLoading = ref<Record<string, boolean>>({});

/**
 * 按 keyword 远程检索该行的审批人候选。上限由后端列出用户的 page_size
 * clamp(1, 100) 决定，靠 keyword 收窄而不是
 * 靠大 page_size 凑全量；空 keyword 返回第一页（未搜索态，界面如实提示非全集）。
 * 失败清空该行列表并显式报错——不兜底成"空列表也能提交"，
 * 缺审批人的 user_task 会被提交边界的 validateLinearChain 拦下。
 */
const searchAssignees = async (node: ProcessNode, keyword: string): Promise<void> => {
  rowLoading.value[node.id] = true;
  try {
    rowCandidates.value[node.id] = await fetchUserCandidates(keyword);
  } catch (error) {
    rowCandidates.value[node.id] = [];
    const text = error instanceof Error ? error.message : String(error);
    logAuxLoadFailure(text, error);
    ElMessage.error(text);
  } finally {
    rowLoading.value[node.id] = false;
  }
};

/** 添加节点（父组件处理节点对象） */
const handleAddNode = () => {
  emit('add-node');
};

/**
 * 节点类型切换：非 user_task 节点引擎不读取审批人（后端 resolve_node_assignee 只对建了工单的
 * user_task 节点解析 assignee），清掉残留值，避免用户误以为填了会生效
 */
const handleNodeTypeChange = (row: ProcessNode) => {
  if (row.type !== 'user_task') {
    row.assignee_value = undefined;
    row.assignee_type = undefined;
  } else if (row.assignee_type === undefined) {
    // 引擎仅 user 有执行路径（后端 resolve_node_assignee），显式写入而非依赖后端默认
    row.assignee_type = 'user';
  }
};

/** 删除节点 */
const handleRemoveNode = (index: number) => {
  emit('remove-node', index);
};

/** 提交（先校验，再通知父组件） */
const handleSubmit = async () => {
  if (!formRef.value) return;
  try {
    await formRef.value.validate();
    emit('submit');
  } catch {
    // 校验失败
  }
};
</script>

<style scoped>
.node-actions {
  margin-bottom: 12px;
}
.node-order-hint {
  margin: 0 0 8px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
</style>
