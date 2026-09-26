/**
 * useActionPrompts.ts — 状态动作类端点所需用户输入的统一采集器。
 *
 * 背景：后端若干动作类端点（取消发票/核销/合同、执行合同、审批定价、报表订阅启停）
 * 以 `Json<T>` 强类型接收请求体，其中「取消原因 / 执行方式·执行金额·执行日期 / 审批通过与否」
 * 为必填字段。此前前端 `request.post(url)` 空手请求 → Axum 直接 400。
 *
 * 本模块用 Element Plus `ElMessageBox` 弹出输入框/选择框真实采集这些值，并做必填校验；
 * 严禁在代码里塞默认值（如 reason='用户取消'）冒充用户输入。
 * 约定：任一采集器在用户主动取消/关闭时返回 null，调用方据此中断整条操作链。
 */
import { ElMessageBox } from 'element-plus';
import { i18n } from '@/i18n';

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

/** 审批结果：approved 必填布尔（后端 ApprovePriceRequest）；remark 可选。 */
export interface ApprovalResult {
  approved: boolean;
  remark?: string;
}

/**
 * 采集器：定价审批通过/不通过 + 可选审批意见。
 * 通过=confirm，不通过=cancel 按钮，关闭(ESC/X)=中断返回 null，避免误提交。
 */
export async function promptApproval(): Promise<ApprovalResult | null> {
  let approved: boolean;
  try {
    await ElMessageBox.confirm(tt('actionForm.approveAsk'), tt('actionForm.approveTitle'), {
      type: 'warning',
      confirmButtonText: tt('actionForm.approvePass'),
      cancelButtonText: tt('actionForm.approveReject'),
      distinguishCancelAndClose: true,
    });
    approved = true;
  } catch (action) {
    if (action === 'cancel') approved = false;
    else return null;
  }
  // 审批意见可选：取消/空 → undefined（不发该字段，后端 remark 为 Option）。
  let remark: string | undefined;
  try {
    const { value } = await ElMessageBox.prompt(
      tt('actionForm.approveRemarkTip'),
      tt('actionForm.approveRemarkTitle'),
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
    // 用户跳过审批意见：视为无意见，继续提交（approved 决策已确定）。
  }
  return { approved, remark };
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
