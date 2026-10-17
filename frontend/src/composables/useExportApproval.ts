import { ref } from 'vue';
import { ElMessage } from 'element-plus';
import { i18n } from '@/i18n';
import { logger, logAuxLoadFailure } from '@/utils/logger';
import { msg } from '@/utils/message';
import { downloadExportWithToken } from '@/utils/export';
import { createApproval, listApprovals } from '@/api/export-approvals';
import type { ExportApprovalRequest } from '@/api/export-approvals';
import { useUserStore } from '@/store/user';
import {
  normalizeExportApprovalResourceType,
  pickExportQueryParams,
  resolveExportDownloadTarget,
} from '@/constants/export-approval';

/**
 * 敏感资源导出审批 composable（各资源导出入口的申请与就地下载单一提交路径）。
 *
 * 调用方：customer/supplier/product/dye_recipe/price_list/finance_report/audit_log
 * 等资源列表页。申请侧职责：按 constants/export-approval.ts 注册表把当前筛选收敛为
 * 该资源导出端点真实读取的参数形状（export_params 落库形状的唯一来源），调用后端
 * POST /export-approvals 创建审批申请；成功后提示用户待批准后在本页就地下载。
 * 下载侧职责（downloadApprovedExport）：审批通过后由申请人在原导出入口页就地拉取
 * 「本人在该资源下、status=approved 且持有一次性令牌且未达下载次数」的申请，用注册表
 * 解析真实导出端点与申请时刻落库的 export_params，经 download_token 查询参数调
 * downloadExportWithToken 下载带水印文件——下载入口分散在各资源页，不再集中在审批页。
 *
 * 入参：resourceType（后端 sensitive_resources 词表值）、params（当前列表筛选，
 * 注册表白名单外的键不透传）、estimatedRows（可选，当前筛选命中总数，决定后端
 * 风险等级与审批层级）。
 * 出参：exportSubmitting / approvedDownloading（提交中状态，供按钮 loading）与
 * requestExport / downloadApprovedExport。
 * 失败处理：错误文案由 request 拦截器统一按后端失败信封 message 外显（不吞不兜底），
 * 此处留显式日志；资源类型不在词表内同样走该日志路径（注册表 normalize 抛错，禁止
 * 静默放行未知资源）；就地下载取不到可用项时按「无已批准申请」与「令牌已用尽或过期」
 * 两类显式提示，绝不静默返回。
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

  const approvedDownloading = ref(false);

  /**
   * 就地下载当前用户在指定资源下已批准的导出文件。
   *
   * 后端 GET /export-approvals 对非 admin 强制 applicant_user_id=当前用户、对 admin
   * 尊重传入键（handlers/export_approval_handler.rs list_approval_requests 覆盖逻辑）；
   * 这里显式传入 applicant_user_id 并按出参逐行核验归属，确保 admin 也不会下载到他人
   * 令牌文件。service 侧 list_requests 按 created_at 倒序返回，取首条「status=approved
   * 且 download_token 非空且 download_count < max_downloads」的申请作为最近一条可用项
   * 下载（多命中取最近一条，符合需求「让用户选或取最近一条」的取最近一条分支）。令牌
   * 是否过期由后端 verify_download_token 权威判定（前端不自比对时钟，避免客户端时钟
   * 偏差误判放行）。取不到可用项时区分两类显式提示，禁止静默返回。
   */
  async function downloadApprovedExport(resourceType: string): Promise<void> {
    if (approvedDownloading.value) return;
    approvedDownloading.value = true;
    try {
      const rt = normalizeExportApprovalResourceType(resourceType);
      const myUserId = useUserStore().userInfo?.id;
      // 身份未就绪（首屏缓存哨兵/未登录）时不得放行下载他人令牌文件，显式提示而非静默
      if (myUserId === undefined) {
        ElMessage.warning(i18n.global.t('exportApprovals.noApprovedExport'));
        logger.warn(i18n.global.t('exportApprovals.logDownloadNoUser'));
        return;
      }
      let rows: ExportApprovalRequest[];
      try {
        const res = await listApprovals({
          status: 'approved',
          resource_type: rt,
          applicant_user_id: myUserId,
          page: 1,
          // service 侧 page_size clamp 上限 100；申请按时间倒序，最近可用项必落在首屏
          page_size: 100,
        });
        rows = res.data.items;
      } catch (error) {
        // 列表拉取失败属辅助加载失败，按分级留痕并外显（后端信封文案由拦截器外显）
        logAuxLoadFailure(i18n.global.t('exportApprovals.logDownloadListFailed'), error);
        return;
      }
      // 逐行核验归属，防御 admin 传入键被绕开而下载到他人令牌文件
      const owned = rows.filter(row => row.applicant_user_id === myUserId);
      const usable = owned.filter(
        row => row.download_token !== null && row.download_count < row.max_downloads
      );
      const chosen = usable[0];
      if (!chosen) {
        // 有已批准申请但令牌不可用 → 令牌已用尽或过期；完全无批准申请 → 无已批准可下载
        const hintKey =
          owned.length > 0 ? 'exportApprovals.tokenExhausted' : 'exportApprovals.noApprovedExport';
        ElMessage.warning(i18n.global.t(hintKey));
        logger.info(i18n.global.t('exportApprovals.logDownloadNone', { resource: rt }));
        return;
      }
      const token = chosen.download_token;
      // usable 已保证非 null，此处仅收窄类型并防御直达调用
      if (token === null) return;
      let target: ReturnType<typeof resolveExportDownloadTarget>;
      try {
        target = resolveExportDownloadTarget(rt, chosen.export_params);
      } catch (error) {
        // 端点解析失败（如 finance_report 申请缺 report_type）单独外显，不进入下载分支
        logger.error(i18n.global.t('exportApprovals.logEndpointFailed'), error);
        ElMessage.error(
          error instanceof Error && error.message
            ? error.message
            : i18n.global.t('exportApprovals.logEndpointFailed')
        );
        return;
      }
      try {
        await downloadExportWithToken({
          apiPath: target.apiPath,
          params: pickExportQueryParams(rt, chosen.export_params),
          token,
          filenamePrefix: target.filenamePrefix,
        });
        logger.info(i18n.global.t('exportApprovals.logDownloaded', { id: chosen.id }));
      } catch (error) {
        // 失败原因已由 downloadExportWithToken 按后端信封外显，此处仅留痕
        logger.error(i18n.global.t('exportApprovals.logDownloadFailed', { id: chosen.id }), error);
      }
    } finally {
      approvedDownloading.value = false;
    }
  }

  return { exportSubmitting, requestExport, approvedDownloading, downloadApprovedExport };
}
