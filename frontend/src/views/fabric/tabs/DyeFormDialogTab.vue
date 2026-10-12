<!--
  DyeFormDialogTab.vue - 染色批次编辑对话框
  来源：原 fabric/index.vue 中 染色批次编辑对话框
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="
      formData.id ? t('fabric.dyeFormDialog.titleEdit') : t('fabric.dyeFormDialog.titleCreate')
    "
    width="700px"
    :aria-label="
      formData.id ? t('fabric.dyeFormDialog.titleEdit') : t('fabric.dyeFormDialog.titleCreate')
    "
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form
      ref="formRef"
      :model="formData"
      label-width="100px"
      :aria-label="t('fabric.dyeFormDialog.formAriaLabel')"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('fabric.dyeFormDialog.labelBatchNo')" prop="batch_no">
            <el-input v-model="formData.batch_no" readonly />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('fabric.dyeFormDialog.labelColor')" prop="color_no">
            <el-input v-model="formData.color_no" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('fabric.dyeFormDialog.labelGreige')" prop="greige_fabric_id">
        <el-select v-model="formData.greige_fabric_id" style="width: 100%">
          <el-option
            v-for="item in greigeFabrics"
            :key="item.id"
            :label="item.fabric_name"
            :value="item.id"
          />
        </el-select>
      </el-form-item>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item
            :label="t('fabric.dyeFormDialog.labelPlannedQuantity')"
            prop="planned_quantity"
          >
            <el-input-number v-model="formData.planned_quantity" :min="0" style="width: 100%" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('fabric.dyeFormDialog.labelDyeLotNo')" prop="dye_lot_no">
            <el-input v-model="formData.dye_lot_no" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('fabric.dyeFormDialog.labelStartDate')" prop="start_date">
        <el-date-picker
          v-model="formData.start_date"
          type="date"
          value-format="YYYY-MM-DD"
          style="width: 100%"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:modelValue', false)">{{
        t('fabric.common.cancel')
      }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
        t('fabric.common.confirm')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { FormInstance } from 'element-plus';
import {
  createDyeBatch,
  updateDyeBatch,
  type CreateDyeBatchPayload,
  type DyeBatch,
  type UpdateDyeBatchPayload,
} from '@/api/dye-batch';
import { generateUniqueDocNo } from '@/utils/document-no';
import type { GreigeFabric } from '@/api/greige-fabric';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

interface Props {
  modelValue: boolean;
  currentRow: DyeBatch | null;
  greigeFabrics: GreigeFabric[];
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void;
  (e: 'submitted'): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

const formRef = ref<FormInstance>();
const submitLoading = ref(false);

const formData = reactive({
  id: 0,
  batch_no: '',
  color_no: '',
  dye_lot_no: '',
  greige_fabric_id: undefined as number | undefined,
  planned_quantity: 0,
  start_date: '',
  status: 'pending_schedule' as DyeBatch['status'],
});

// 打开编辑时的状态原值：status 提交即触发后端状态机流转校验（同态自转不合法），
// 仅当用户显式改动状态（与 loadedStatus 不同）才随 payload 携带。
const loadedStatus = ref<DyeBatch['status']>(null);

const resetForm = () => {
  formData.id = 0;
  formData.batch_no = '';
  formData.color_no = '';
  formData.dye_lot_no = '';
  formData.greige_fabric_id = undefined;
  formData.planned_quantity = 0;
  formData.start_date = '';
  formData.status = 'pending_schedule';
  loadedStatus.value = null;
};

watch(
  () => props.modelValue,
  val => {
    if (val) {
      if (props.currentRow) {
        // 显式逐字段回填（不再整行 Object.assign）：出参 DyeBatch 与表单字段不是一一对应——
        // planned_quantity 是 rust_decimal 序列化的字符串、started_at 是完整时间戳（表单要
        // YYYY-MM-DD 的 start_date 键）、行里没有 start_date/actual_quantity 键。整行灌入会把
        // 字符串灌进 number 字段、编辑保存按原值回传即 400/丢数据（用户报"再编辑不完整"）。
        const row = props.currentRow;
        formData.id = row.id;
        formData.batch_no = row.batch_no;
        formData.color_no = row.color_no ?? '';
        formData.dye_lot_no = row.dye_lot_no ?? '';
        formData.greige_fabric_id = row.greige_fabric_id ?? undefined;
        formData.planned_quantity = Number(row.planned_quantity ?? 0);
        formData.start_date = row.started_at ? row.started_at.slice(0, 10) : '';
        formData.status = row.status ?? 'pending_schedule';
        loadedStatus.value = row.status;
      } else {
        resetForm();
        // 新建时预生成缸号（查重唯一后只读展示，防手动输入重复）；
        // fail-visible：取号抛错（查重接口异常或重试耗尽）必须用户可见，禁止静默留空
        initBatchNo();
      }
    }
  }
);

/** 生成唯一缸号；失败即显式报错（号码留空时 handleSubmit 阻止提交） */
const initBatchNo = async () => {
  try {
    formData.batch_no = await generateUniqueDocNo('DB', 'dye_batch');
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('fabric.dyeFormDialog.docNoGenerateFailed'));
    logger.error(t('fabric.dyeFormDialog.docNoGenerateFailed'), err.message);
  }
};

const handleSubmit = async () => {
  // 取号失败时缸号为空：阻止提交（后端 batch_no NOT NULL，空号提交只会得到库级报错）
  if (!formData.id && !formData.batch_no) {
    ElMessage.error(t('fabric.dyeFormDialog.docNoGenerateFailed'));
    return;
  }
  submitLoading.value = true;
  try {
    // 显式构造真实契约载荷（api/dye-batch.ts Create/UpdateDyeBatchPayload ↔ 后端
    // dye_batch_handler.rs 两个请求 DTO），不再用类型断言把整份表单硬塞进接口绕过契约检查：
    // - color_no/dye_lot_no 空值整键省略（后端按"色号空=白坯"归一，提交 '' 是契约外形态）；
    // - 表单 start_date 对应后端 dye_date 键（落库映射 started_at）；
    // - status 仅在与打开时原值（loadedStatus）不同才携带，避免触发同态自转的状态机拒绝；
    // - id/batch_no(编辑)/start_date(编辑) 等不在更新 DTO 的键自然不出现在载荷里。
    if (formData.id) {
      const payload: UpdateDyeBatchPayload = {
        planned_quantity: formData.planned_quantity,
      };
      const colorNo = formData.color_no.trim();
      if (colorNo) payload.color_no = colorNo;
      const dyeLotNo = formData.dye_lot_no.trim();
      if (dyeLotNo) payload.dye_lot_no = dyeLotNo;
      if (formData.greige_fabric_id !== undefined) {
        payload.greige_fabric_id = formData.greige_fabric_id;
      }
      if (formData.status && formData.status !== loadedStatus.value) {
        payload.status = formData.status;
      }
      await updateDyeBatch(formData.id, payload);
    } else {
      const payload: CreateDyeBatchPayload = {
        batch_no: formData.batch_no,
        planned_quantity: formData.planned_quantity,
      };
      const colorNo = formData.color_no.trim();
      if (colorNo) payload.color_no = colorNo;
      const dyeLotNo = formData.dye_lot_no.trim();
      if (dyeLotNo) payload.dye_lot_no = dyeLotNo;
      if (formData.greige_fabric_id !== undefined) {
        payload.greige_fabric_id = formData.greige_fabric_id;
      }
      if (formData.start_date) payload.dye_date = formData.start_date;
      await createDyeBatch(payload);
    }
    ElMessage.success(t('fabric.common.success'));
    emit('update:modelValue', false);
    emit('submitted');
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('fabric.common.failed'));
    logger.error(t('fabric.dyeFormDialog.saveFailed'), err.message);
  } finally {
    submitLoading.value = false;
  }
};
</script>
