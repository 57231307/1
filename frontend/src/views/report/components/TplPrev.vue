<script setup lang="ts">
/**
 * TplPrev - 报表预览对话框（纯展示）
 * 任务编号: P13 批 1 B3 I-1（拆分 report/templates.vue 预览对话框）
 */
interface Props {
  modelValue: boolean
  previewData: any
}

defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
}>()
</script>

<template>
  <el-dialog
    :model-value="modelValue"
    title="报表预览"
    width="900px"
    @update:model-value="(v: boolean) => emit('update:modelValue', v)"
  >
    <div v-if="previewData">
      <el-table :data="previewData.list || []" border style="width: 100%">
        <el-table-column
          v-for="col in previewData.columns || []"
          :key="col.key"
          :prop="col.key"
          :label="col.label"
          :width="col.width"
        />
      </el-table>
      <div class="preview-total">共 {{ previewData.total || 0 }} 条记录</div>
    </div>
  </el-dialog>
</template>
