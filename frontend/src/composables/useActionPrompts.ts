/**
 * useActionPrompts.ts — 状态动作类端点所需用户输入的统一采集器。
 *
 * 背景：后端若干动作类端点（取消发票/核销/合同、执行合同、报表订阅启停、
 * 价目与合同的审批通过/拒绝）以 `Json<T>` 或 `Option<Json<T>>` 接收请求体，
 * 其中「取消原因 / 审批通过理由 / 审批拒绝理由 / 执行方式·执行金额·执行日期」
 * 为必填字段。此前前端 `request.post(url)` 空手请求 → 后端必填校验收 400。
 *
 * 本模块用 Element Plus `ElMessageBox` 弹出输入框/选择框真实采集这些值，并做必填校验；
 * 严禁在代码里塞默认值（如 reason='用户取消'）冒充用户输入。
 * 约定：任一采集器在用户主动取消/关闭时返回 null，调用方据此中断整条操作链。
 * 约定：返回值区分「未采集（null，用户取消）」与「采集到的字符串（可能是空串）」，
 *       调用方必须用 `=== null` 判中断，不得用真值判断把空理由误当成取消。
 */
import { ElMessageBox } from 'element-plus';
import { i18n } from '@/i18n';
import { isDialogDismissal } from '@/utils/monitor';

/** 取已翻译文案（i18n 全局实例，无需在采集器内注入 useI18n）。 */
const tt = (key: string): string => String(i18n.global.t(key));

/** 采集器：取消原因（后端 CancelInvoiceRequest/CancelVerificationRequest/CancelReason/CancelContractRequest 必填）。 */
export async function promptCancelReason(): Promise<string | null> {
  try {
    const { value } = await ElMessageBox.prompt(
      tt('actionForm.cancelReasonTip'),
      tt('actionForm.cancelReasonTitle'),
      {
        inputType: 'textarea',
        inputPlaceholder: tt('actionForm.cancelReasonPlaceholder'),
        confirmButtonText: tt('actionForm.confirm'),
        cancelButtonText: tt('actionForm.cancel'),
        inputValidator: (v: string) =>
          v && v.trim() ? true : tt('actionForm.cancelReasonRequired'),
      }
    );
    return value.trim();
  } catch {
    return null;
  }
}

/** 采集器：审批通过理由（后端 approve 端点 approval_reason 必填并真实落库）。
 *  required=true：空值/纯空白不允许继续（后端对缺失与空白一律 400 VALIDATION_ERROR）；
 *  required=false：允许留空提交，返回空串由调用方决定是否省略该键。
 *  取消/关闭返回 null（流程中止，不是错误）；非取消形态的 reject 原样上抛，不静默降级。 */
export async function promptApprovalReason(required: boolean): Promise<string | null> {
  try {
    const { value } = await ElMessageBox.prompt(
      // 提示语按档位切换：选填档沿用必填措辞会误导用户（留空本可通过却以为会被拒）
      required ? tt('actionForm.approvalReasonTip') : tt('actionForm.approvalReasonOptionalTip'),
      tt('actionForm.approvalReasonTitle'),
      {
        inputType: 'textarea',
        inputPlaceholder: tt('actionForm.approvalReasonPlaceholder'),
        confirmButtonText: tt('actionForm.confirm'),
        cancelButtonText: tt('actionForm.cancel'),
        inputValidator: (v: string) => {
          if (v && v.trim()) return true;
          return required ? tt('actionForm.approvalReasonRequired') : true;
        },
      }
    );
    return value.trim();
  } catch (error: unknown) {
    if (isDialogDismissal(error)) return null;
    throw error;
  }
}

/** 采集器：审批拒绝理由（后端 reject 端点 reason 必填并真实落库，全域必填）。
 *  取消/关闭返回 null（流程中止，不是错误）；非取消形态的 reject 原样上抛。 */
export async function promptRejectReason(): Promise<string | null> {
  try {
    const { value } = await ElMessageBox.prompt(
      tt('actionForm.rejectReasonTip'),
      tt('actionForm.rejectReasonTitle'),
      {
        inputType: 'textarea',
        inputPlaceholder: tt('actionForm.rejectReasonPlaceholder'),
        confirmButtonText: tt('actionForm.confirm'),
        cancelButtonText: tt('actionForm.cancel'),
        inputValidator: (v: string) =>
          v && v.trim() ? true : tt('actionForm.rejectReasonRequired'),
      }
    );
    return value.trim();
  } catch (error: unknown) {
    if (isDialogDismissal(error)) return null;
    throw error;
  }
}

/** 采集器：执行合同入参。销售无执行日期（hasDate=false），采购有执行日期（hasDate=true）。 */
export interface ExecuteFormResult {
  execution_type: string;
  execution_amount: number;
  execution_date?: string;
  remark?: string;
}

/** 执行方式候选项（value 须与后端词表一致；label 由调用方用 t() 预先翻译，便于 i18n 门禁校验）。 */
export interface ExecutionTypeOption {
  value: string;
  label: string;
}

const DATE_PATTERN = /^\d{4}-\d{2}-\d{2}$/;

/**
 * 采集器：合同执行（execution_type 二选一 + 执行金额必填>0 + 可选执行日期 + 可选备注）。
 * 要求恰好传入两个执行方式选项：confirm 按钮选第一个，cancel 按钮选第二个，关闭=中断。
 */
export async function promptContractExecute(
  typeOptions: [ExecutionTypeOption, ExecutionTypeOption],
  hasDate: boolean
): Promise<ExecuteFormResult | null> {
  // 1) 执行方式（后端词表二选一）
  let execution_type: string;
  try {
    await ElMessageBox.confirm(tt('actionForm.executeTypeTip'), tt('actionForm.executeTypeTitle'), {
      type: 'info',
      confirmButtonText: typeOptions[0].label,
      cancelButtonText: typeOptions[1].label,
      distinguishCancelAndClose: true,
    });
    execution_type = typeOptions[0].value;
  } catch (action) {
    if (action === 'cancel') execution_type = typeOptions[1].value;
    else return null;
  }

  // 2) 执行金额（必填，数值 >0）
  let execution_amount: number;
  try {
    const { value } = await ElMessageBox.prompt(
      tt('actionForm.executeAmountTip'),
      tt('actionForm.executeAmountTitle'),
      {
        inputType: 'number',
        inputPlaceholder: tt('actionForm.executeAmountPlaceholder'),
        confirmButtonText: tt('actionForm.confirm'),
        cancelButtonText: tt('actionForm.cancel'),
        inputValidator: (v: string) => {
          if (!v || !v.trim()) return tt('actionForm.executeAmountRequired');
          const n = Number(v);
          if (!Number.isFinite(n) || n <= 0) return tt('actionForm.executeAmountPositive');
          return true;
        },
      }
    );
    execution_amount = Number(value);
  } catch {
    return null;
  }

  // 3) 执行日期（仅采购，必填 YYYY-MM-DD）
  let execution_date: string | undefined;
  if (hasDate) {
    try {
      const { value } = await ElMessageBox.prompt(
        tt('actionForm.executeDateTip'),
        tt('actionForm.executeDateTitle'),
        {
          inputPlaceholder: tt('actionForm.executeDatePlaceholder'),
          confirmButtonText: tt('actionForm.confirm'),
          cancelButtonText: tt('actionForm.cancel'),
          inputValidator: (v: string) => {
            if (!v || !v.trim()) return tt('actionForm.executeDateRequired');
            if (!DATE_PATTERN.test(v.trim())) return tt('actionForm.executeDateInvalid');
            return true;
          },
        }
      );
      execution_date = value.trim();
    } catch {
      return null;
    }
  }

  // 4) 备注（可选）
  let remark: string | undefined;
  try {
    const { value } = await ElMessageBox.prompt(
      tt('actionForm.executeRemarkTip'),
      tt('actionForm.executeRemarkTitle'),
      {
        inputType: 'textarea',
        inputPlaceholder: tt('actionForm.optionalPlaceholder'),
        confirmButtonText: tt('actionForm.confirm'),
        cancelButtonText: tt('actionForm.cancel'),
      }
    );
    const trimmed = value?.trim();
    if (trimmed) remark = trimmed;
  } catch {
    // 跳过备注：不影响执行提交。
  }

  return { execution_type, execution_amount, execution_date, remark };
}
