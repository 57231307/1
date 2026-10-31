import { ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { ElMessage } from 'element-plus';
import { i18n } from '@/i18n';
import { logger, logAuxLoadFailure } from '@/utils/logger';
import { msg } from '@/utils/message';
import { downloadExportWithToken } from '@/utils/export';
import { createApproval, listApprovals, viewTokenOnce, rotateToken } from '@/api/export-approvals';
import type { ExportApprovalRequest } from '@/api/export-approvals';
import { useUserStore } from '@/store/user';
import {
  normalizeExportApprovalResourceType,
  pickExportQueryParams,
  resolveExportDownloadTarget,
  parseExportApprovalJumpback,
} from '@/constants/export-approval';

/**
 * 敏感资源导出审批 composable（各资源导出入口的申请与就地下载单一提交路径）。
 *
 * 调用方：customer/supplier/product/dye_recipe/price_list/finance_report/audit_log
 * 等资源列表页。申请侧职责：按 constants/export-approval.ts 注册表把当前筛选收敛为
 * 该资源导出端点真实读取的参数形状（export_params 落库形状的唯一来源），调用后端
 * POST /export-approvals 创建审批申请（后端同事务发起 BPM 流程，申请进入审批链条，
 * 审批中心可见可办理）；成功后提示用户等待批准。
 * 下载侧职责（downloadApprovedExport）：审批通过后由申请人在原导出入口页就地拉取
 * 「本人在该资源下、status=approved 且未消费且重试额度未尽」的申请，经
 * token-once 端点一次性取回令牌明文（明文已取过时经 rotate 端点重发换取新明文，
 * 旧令牌即刻作废），用注册表解析真实导出端点与申请时刻落库的 export_params，
 * 经 download_token 查询参数调 downloadExportWithToken 下载带水印文件。
 * 令牌作废、重试窗口与次数用尽四类失败态由后端失败信封逐态点名（拦截器外显），
 * 前端不复制状态判定、不自比对时钟做门禁。
 *
 * 入参：resourceType（后端 sensitive_resources 词表值）、params（当前列表筛选，
 * 注册表白名单外的键不透传）、estimatedRows（可选，当前筛选命中总数，决定后端
 * 风险等级与审批层级）。
 * 出参：exportSubmitting / approvedDownloading（提交中状态，供按钮 loading）与
 * requestExport / downloadApprovedExport。
 * 失败处理：错误文案由 request 拦截器统一按后端失败信封 message 外显（不吞不兜底），
 * 此处留显式日志；资源类型不在词表内同样走该日志路径（注册表 normalize 抛错，禁止
 * 静默放行未知资源）；就地下载取不到可用项时按「无已批准申请」「额度未尽但均需重发」
 * 与「额度用尽或已过期」显式提示，绝不静默返回。
 */
export function useExportApproval() {
  const exportSubmitting = ref(false);
  const route = useRoute();
  const router = useRouter();

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
   * 按申请行取回可用的令牌明文。
   *
   * 首选 token-once（首查即得明文，服务端取走后置位并清空明文列）；已取走过
   * （available=false 只回指纹）时经 rotate 重发换取新明文——重发作废旧令牌且
   * 不顺延硬有效期、不刷新重试额度，门禁全在后端。重发被拒（额度用尽/已过期/
   * 已消费）时失败信封文案由拦截器外显，此处只中止流程并留痕。
   */
  async function acquireToken(row: ExportApprovalRequest): Promise<string | null> {
    const id = row.id;
    try {
      const once = await viewTokenOnce(id);
      if (once.data.available && once.data.token) {
        return once.data.token;
      }
      const rotated = await rotateToken(id);
      return rotated.data.token || null;
    } catch (error) {
      // 取 token 失败属受理面真实拒绝（四态文案之一），已由拦截器外显
      logger.info(i18n.global.t('exportApprovals.logAcquireTokenFailed', { id }), error);
      return null;
    }
  }

  /**
   * 就地下载当前用户在指定资源下已批准的导出文件。
   *
   * 后端 GET /export-approvals 对非 admin 强制 applicant_user_id=当前用户；
   * 这里显式传入并逐行核验归属，确保 admin 也不会下载到他人令牌文件。
   * service 侧按 created_at 倒序返回，取首条「status=approved 且 token_consumed=false
   * 且未消费完 download_count<max_downloads」的申请作为最近可用项。令牌是否过期、
   * 重试窗口与额度由后端权威判定（前端不自比对时钟做门禁），四态失败经拦截器外显。
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
      // 可用项：未消费（token_consumed=false 且 download_count<max_downloads）
      // 且重试额度未尽（retry_count<max_retries）。exhausted/expired/consumed
      // 状态的行不会出现在 status=approved 结果集（后端状态机保证）。
      const usable = owned.filter(
        row =>
          !row.token_consumed &&
          row.download_count < row.max_downloads &&
          row.retry_count < row.max_retries
      );
      // 审批「去导出」回跳带 businessId 时优先取那一单（须在本人可用集内，
      // 归属与可用门仍逐行核验，不因回跳而绕过）；无回跳上下文按最近可用项兜前。
      const jumpback = parseExportApprovalJumpback(route.query as Record<string, unknown>);
      const chosen =
        (jumpback ? usable.find(row => row.id === jumpback.businessId) : undefined) ?? usable[0];
      if (!chosen) {
        // 有已批准申请但均不可用 → 区分「额度用尽」与「已消费/其他」提示；
        // 完全无批准申请 → 无已批准可下载。
        const exhausted = owned.some(
          row => row.token_consumed || row.retry_count >= row.max_retries
        );
        const hintKey = exhausted
          ? 'exportApprovals.tokenExhausted'
          : 'exportApprovals.noApprovedExport';
        ElMessage.warning(i18n.global.t(hintKey));
        logger.info(i18n.global.t('exportApprovals.logDownloadNone', { resource: rt }));
        return;
      }
      const token = await acquireToken(chosen);
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
        // 回跳上下文已兑现，清理地址栏的 source/instanceId/businessId，
        // 避免刷新或后退重新触发下载、也不把审批单上下文长期滞留在可见 URL
        if (jumpback) {
          const restQuery = { ...route.query };
          delete restQuery.source;
          delete restQuery.instanceId;
          delete restQuery.businessId;
          void router.replace({ query: restQuery });
        }
      } catch (error) {
        // 失败原因已由 downloadExportWithToken 按后端信封外显（含重试窗口/用尽/消费四态），
        // 此处仅留痕
        logger.error(i18n.global.t('exportApprovals.logDownloadFailed', { id: chosen.id }), error);
      }
    } finally {
      approvedDownloading.value = false;
    }
  }

  return { exportSubmitting, requestExport, approvedDownloading, downloadApprovedExport };
}
