<template>
  <div class="advanced-filter">
    <div class="filter-header">
      <h3 class="filter-title">高级筛选</h3>
      <el-space>
        <el-button size="small" :disabled="conditions.length === 0" @click="showSaveDialog = true">
          <el-icon><Folder /></el-icon>
          保存方案
        </el-button>
        <el-dropdown v-if="savedSchemes.length > 0" @command="loadScheme">
          <el-button size="small">
            <el-icon><Download /></el-icon>
            加载方案
            <el-icon><ArrowDown /></el-icon>
          </el-button>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item v-for="scheme in savedSchemes" :key="scheme.id" :command="scheme">
                {{ scheme.name }}
              </el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </el-space>
    </div>

    <div class="filter-body">
      <el-card v-for="(group, groupIndex) in conditions" :key="groupIndex" class="condition-group">
        <template #header>
          <div class="group-header">
            <el-select
              v-if="groupIndex > 0"
              v-model="group.logic"
              size="small"
              style="width: 100px"
              @change="handleLogicChange"
            >
              <el-option label="AND" value="AND" />
              <el-option label="OR" value="OR" />
            </el-select>
            <span v-else class="group-label">条件组</span>
            <el-button
              type="danger"
              size="small"
              :icon="Delete"
              circle
              :disabled="conditions.length <= 1"
              @click="removeGroup(groupIndex)"
            />
          </div>
        </template>

        <div v-for="(condition, condIndex) in group.items" :key="condIndex" class="condition-row">
          <el-select
            v-model="condition.field"
            placeholder="选择字段"
            style="width: 160px"
            @change="handleFieldChange(condition)"
          >
            <el-option
              v-for="field in fields"
              :key="field.key"
              :label="field.label"
              :value="field.key"
            />
          </el-select>

          <el-select v-model="condition.operator" placeholder="操作符" style="width: 120px">
            <el-option
              v-for="op in getAvailableOperators(condition.field)"
              :key="op.value"
              :label="op.label"
              :value="op.value"
            />
          </el-select>

          <component
            :is="getValueInput(condition)"
            v-model="condition.value"
            :placeholder="'输入值'"
            style="flex: 1; min-width: 150px"
          />

          <el-button
            type="info"
            size="small"
            :icon="Delete"
            circle
            :disabled="group.items.length <= 1"
            @click="removeCondition(groupIndex, condIndex)"
          />
        </div>

        <el-button
          type="primary"
          link
          size="small"
          class="add-btn"
          @click="addCondition(groupIndex)"
        >
          <el-icon><Plus /></el-icon>
          添加条件
        </el-button>
      </el-card>

      <el-button type="primary" class="add-group-btn" @click="addGroup">
        <el-icon><Plus /></el-icon>
        添加条件组
      </el-button>
    </div>

    <div class="filter-footer">
      <el-space>
        <el-button type="primary" :disabled="!isValid" @click="handleApply"> 应用筛选 </el-button>
        <el-button @click="handleReset">重置</el-button>
      </el-space>
    </div>

    <el-dialog v-model="showSaveDialog" title="保存筛选方案" width="400px">
      <el-form @submit.prevent="saveScheme">
        <el-form-item label="方案名称">
          <el-input v-model="newSchemeName" placeholder="输入方案名称" autofocus />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showSaveDialog = false">取消</el-button>
        <el-button type="primary" @click="saveScheme">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { ElMessage } from 'element-plus'
import { Plus, Delete, Folder, Download, ArrowDown } from '@element-plus/icons-vue'

export interface FilterField {
  key: string
  label: string
  type?: 'text' | 'number' | 'date' | 'select' | 'boolean'
  options?: { label: string; value: any }[]
}

export interface FilterOperator {
  label: string
  value: string
  applicableTypes?: string[]
}

export interface FilterCondition {
  field: string
  operator: string
  value: any
}

export interface FilterGroup {
  logic: 'AND' | 'OR'
  items: FilterCondition[]
}

export interface SavedScheme {
  id: string
  name: string
  groups: FilterGroup[]
  createdAt: string
}

interface Props {
  fields?: FilterField[]
  operators?: FilterOperator[]
  savedSchemes?: SavedScheme[]
}

const props = withDefaults(defineProps<Props>(), {
  fields: () => [
    { key: 'name', label: '名称', type: 'text' },
    {
      key: 'status',
      label: '状态',
      type: 'select',
      options: [
        { label: '启用', value: 'active' },
        { label: '禁用', value: 'inactive' },
      ],
    },
    { key: 'date', label: '日期', type: 'date' },
    { key: 'amount', label: '金额', type: 'number' },
  ],
  operators: () => [],
  savedSchemes: () => [],
})

const emit = defineEmits<{
  apply: [filters: FilterGroup[]]
  reset: []
  schemeSaved: [scheme: SavedScheme]
  schemeLoaded: [scheme: SavedScheme]
}>()

const defaultOperators: FilterOperator[] = [
  { label: '等于', value: 'eq', applicableTypes: ['text', 'number', 'date', 'select', 'boolean'] },
  {
    label: '不等于',
    value: 'neq',
    applicableTypes: ['text', 'number', 'date', 'select', 'boolean'],
  },
  { label: '包含', value: 'contains', applicableTypes: ['text'] },
  { label: '不包含', value: 'notContains', applicableTypes: ['text'] },
  { label: '大于', value: 'gt', applicableTypes: ['number', 'date'] },
  { label: '大于等于', value: 'gte', applicableTypes: ['number', 'date'] },
  { label: '小于', value: 'lt', applicableTypes: ['number', 'date'] },
  { label: '小于等于', value: 'lte', applicableTypes: ['number', 'date'] },
  { label: '为空', value: 'null', applicableTypes: ['text', 'number', 'date'] },
  { label: '不为空', value: 'notNull', applicableTypes: ['text', 'number', 'date'] },
]

const conditions = ref<FilterGroup[]>([
  {
    logic: 'AND',
    items: [{ field: '', operator: '', value: '' }],
  },
])

const showSaveDialog = ref(false)
const newSchemeName = ref('')

const isValid = computed(() => {
  return conditions.value.every((group) => group.items.every((item) => item.field && item.operator))
})

const getAvailableOperators = (fieldKey: string): FilterOperator[] => {
  if (props.operators.length > 0) return props.operators
  const field = props.fields.find((f) => f.key === fieldKey)
  if (!field) return defaultOperators
  return defaultOperators.filter(
    (op) => !op.applicableTypes || op.applicableTypes.includes(field.type || 'text')
  )
}

const handleFieldChange = (condition: FilterCondition) => {
  condition.operator = ''
  condition.value = ''
}

const handleLogicChange = () => {}

const getValueInput = (condition: FilterCondition) => {
  if (['null', 'notNull'].includes(condition.operator)) {
    return 'span'
  }
  const field = props.fields.find((f) => f.key === condition.field)
  if (!field) return 'el-input'

  switch (field.type) {
    case 'number':
      return 'el-input-number'
    case 'date':
      return 'el-date-picker'
    case 'select':
      return 'el-select'
    case 'boolean':
      return 'el-switch'
    default:
      return 'el-input'
  }
}

const addCondition = (groupIndex: number) => {
  conditions.value[groupIndex].items.push({ field: '', operator: '', value: '' })
}

const removeCondition = (groupIndex: number, condIndex: number) => {
  conditions.value[groupIndex].items.splice(condIndex, 1)
}

const addGroup = () => {
  conditions.value.push({
    logic: 'AND',
    items: [{ field: '', operator: '', value: '' }],
  })
}

const removeGroup = (groupIndex: number) => {
  if (conditions.value.length > 1) {
    conditions.value.splice(groupIndex, 1)
  }
}

const handleApply = () => {
  emit('apply', conditions.value)
  ElMessage.success('筛选已应用')
}

const handleReset = () => {
  conditions.value = [
    {
      logic: 'AND',
      items: [{ field: '', operator: '', value: '' }],
    },
  ]
  emit('reset')
}

const saveScheme = () => {
  if (!newSchemeName.value.trim()) {
    ElMessage.warning('请输入方案名称')
    return
  }
  const scheme: SavedScheme = {
    id: Date.now().toString(),
    name: newSchemeName.value,
    groups: JSON.parse(JSON.stringify(conditions.value)),
    createdAt: new Date().toISOString(),
  }
  emit('schemeSaved', scheme)
  showSaveDialog.value = false
  newSchemeName.value = ''
  ElMessage.success('方案已保存')
}

const loadScheme = (scheme: SavedScheme) => {
  conditions.value = JSON.parse(JSON.stringify(scheme.groups))
  emit('schemeLoaded', scheme)
  ElMessage.success(`已加载方案: ${scheme.name}`)
}

defineExpose({ conditions, isValid })
</script>

<style scoped>
.advanced-filter {
  border: 1px solid #ebeef5;
  border-radius: 8px;
  background: #fafafa;
}

.filter-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  background: #fff;
  border-bottom: 1px solid #ebeef5;
  border-radius: 8px 8px 0 0;
}

.filter-title {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: #303133;
}

.filter-body {
  padding: 16px 20px;
}

.condition-group {
  margin-bottom: 16px;
}

.group-header {
  display: flex;
  align-items: center;
  gap: 12px;
}

.group-label {
  font-weight: 500;
  color: #606266;
}

.condition-row {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.add-btn {
  margin-top: 8px;
}

.add-group-btn {
  width: 100%;
  margin-top: 8px;
  border-style: dashed;
}

.filter-footer {
  padding: 16px 20px;
  background: #fff;
  border-top: 1px solid #ebeef5;
  border-radius: 0 0 8px 8px;
  display: flex;
  justify-content: flex-end;
}
</style>
