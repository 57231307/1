import { ref } from 'vue';
import { i18n } from '@/i18n';
import { logger } from '@/utils/logger';
import { msg } from '@/utils/message';
import { createApproval } from '@/api/export-approvals';
import {
  normalizeExportApprovalResourceType,
  pickExportQueryParams,
} from '@/constants/export-approval';

/**
 * 敏感资源导出审批申请 composable（各资源导出入口的单一提交路径）。
 *
 * 调用方：customer/supplier/product/dye_recipe/price_list/finance_report/audit_log
 * 等资源列表页的导出按钮。职责：按 constants/export-approval.ts 注册表把当前筛选
 * 收敛为该资源导出端点真实读取的参数形状（export_params 落库形状的唯一来源），
 * 调用后端 POST /export-approvals 创建审批申请；成功后提示用户前往「导出审批」页
 * 在批准后凭一次性令牌下载（下载走审批页集中入口，不在各页直连无令牌必 403 的
 * 原始导出端点）。
 *
 * 入参：resourceType（后端 sensitive_resources 词表值）、params（当前列表筛选，
 * 注册表白名单外的键不透传）、estimatedRows（可选，当前筛选命中总数，决定后端
 * 风险等级与审批层级）。
 * 出参：exportSubmitting（提交中状态，供按钮 loading）与 requestExport。
 * 失败处理：错误文案由 request 拦截器统一按后端失败信封 message 外显（不吞不
 * 兜底），此处留显式日志；资源类型不在词表内同样走该日志路径（注册表 normalize
 * 抛错，禁止静默放行未知资源）。
 */
export function useExportApproval() {
  const exportSubmitting = ref(false);

  async function requestExport(
    resourceType: string,
    params: Record<string, unknown>,
    estimatedRows?: number
  ): Promise<void> {
    if (exportSubmitting.value) return;
    exportSubmitting.value = true;
    try {
      const rt = normalizeExportApprovalResourceType(resourceType);
      const exportParams = pickExportQueryParams(rt, params);
      const res = await createApproval({
        resource_type: rt,
        export_params: exportParams,
        estimated_rows: estimatedRows,
        // 后端 create 校验 file_format 仅接受 xlsx/pdf（export_approval_service
        // validate 分支）；成品导出规则统一 xlsx
        file_format: 'xlsx',
      });
      logger.info(
        i18n.global.t('exportApprovals.logRequestSubmitted', {
          resource: rt,
          id: res.data.id,
        })
      );
      msg.success('exportApprovalSubmittedGuide');
    } catch (error) {
      logger.error(i18n.global.t('exportApprovals.logRequestFailed'), error);
    } finally {
      exportSubmitting.value = false;
    }
  }

  return { exportSubmitting, requestExport };
}
