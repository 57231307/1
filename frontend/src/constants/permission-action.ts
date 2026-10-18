/**
 * 权限动作（action）候选词表与 i18n 文案键映射。
 *
 * 真相源是后端可授予动作全集 `assignable_actions()`
 * （backend/src/middleware/permission.rs：方法派生动作 ∪ 路径末段动作关键字 ∪
 * `?action=` 查询白名单，去重后按首次出现顺序），权限目录端点 GET /permissions 的
 * 动作侧即由它派生。本表只做 token → 展示文案键的映射，不新增取值；token 集合与
 * 后端动作词表逐字一致，后端增删动作时本表与语言包必须同步，否则消费页会原样显示
 * token 并留 warn（不猜含义、不伪造文案）。
 *
 * labelKey 写完整 i18n 路径：check-i18n 把 labelKey 字面量当引用校验存在性，
 * 半截路径会落在动态拼接不判定里、拼错就直接把原始键显示到界面。
 */
export interface PermissionActionOption {
  value: string;
  labelKey: string;
}

/** 可授予动作词表，顺序与后端 assignable_actions() 的派生顺序一致 */
export const PERMISSION_ACTION_OPTIONS: PermissionActionOption[] = [
  { value: 'read', labelKey: 'system.permission.action.read' },
  { value: 'create', labelKey: 'system.permission.action.create' },
  { value: 'update', labelKey: 'system.permission.action.update' },
  { value: 'delete', labelKey: 'system.permission.action.delete' },
  { value: 'print', labelKey: 'system.permission.action.print' },
  { value: 'export', labelKey: 'system.permission.action.export' },
  { value: 'import', labelKey: 'system.permission.action.import' },
  { value: 'audit', labelKey: 'system.permission.action.audit' },
  { value: 'approve', labelKey: 'system.permission.action.approve' },
  { value: 'reject', labelKey: 'system.permission.action.reject' },
  { value: 'cancel', labelKey: 'system.permission.action.cancel' },
  { value: 'close', labelKey: 'system.permission.action.close' },
  { value: 'confirm', labelKey: 'system.permission.action.confirm' },
  { value: 'submit', labelKey: 'system.permission.action.submit' },
  { value: 'release', labelKey: 'system.permission.action.release' },
  { value: 'reveal', labelKey: 'system.permission.action.reveal' },
  { value: 'concession', labelKey: 'system.permission.action.concession' },
  { value: 'rejudge', labelKey: 'system.permission.action.rejudge' },
  { value: 'download', labelKey: 'system.permission.action.download' },
];

const LABEL_KEY_BY_ACTION: Record<string, string> = Object.fromEntries(
  PERMISSION_ACTION_OPTIONS.map(item => [item.value, item.labelKey])
);

/**
 * 动作 token 到 i18n 文案键的映射，未知 token 返回 undefined 交由调用方原样显示并告警，
 * 不在此处猜含义，也不静默吞掉后端动作词表漂移。
 */
export function permissionActionLabelKey(value: string): string | undefined {
  return LABEL_KEY_BY_ACTION[value];
}
