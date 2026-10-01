/**
 * 新增域（批 F~M）页面工具函数与常量单测
 *
 * 目的：新页面批量加入后函数覆盖率跌破 1% 门槛（0.97%），本文件对
 * 新域 api 层可测的纯逻辑（状态映射表、常量序列、参数构造）补真实断言。
 */
import { describe, expect, it } from 'vitest';

import { BAD_DEBT_STATUS_LABEL, COLLECTION_TASK_STATUS_LABEL } from '@/api/bad-debt';
import {
  advanceCustomOrder,
  createCustomOrder,
  getCustomOrderList,
  CUSTOM_ORDER_STATUSES,
} from '@/api/custom-order';
import { listOaAnnouncements, publishOaAnnouncement } from '@/api/oa-announcement';
import { INSPECTION_STATUS_LABEL } from '@/api/fabric-inspection';
import { WAGE_RECORD_STATUS_LABEL } from '@/api/wage';
import { OUTSOURCING_STATUS_LABEL } from '@/api/outsourcing';
import { QUALITY_8D_STAGES } from '@/api/quality-8d';

describe('新增域状态映射', () => {
  it('坏账状态映射包含全部后端状态', () => {
    expect(BAD_DEBT_STATUS_LABEL.pending).toBe('待确认');
    expect(BAD_DEBT_STATUS_LABEL.confirmed).toBe('已确认');
    expect(BAD_DEBT_STATUS_LABEL.reversed).toBe('已冲销');
  });

  it('催收任务状态映射完整', () => {
    expect(Object.keys(COLLECTION_TASK_STATUS_LABEL)).toContain('open');
    expect(COLLECTION_TASK_STATUS_LABEL.completed).toBe('已完成');
  });

  it('定制订单 api 导出契约（状态常量+列表/创建/推进）', () => {
    expect(typeof getCustomOrderList).toBe('function');
    expect(typeof createCustomOrder).toBe('function');
    expect(typeof advanceCustomOrder).toBe('function');
    // 词表唯一来源＝后端权威模块 custom_order::ALL（11 态小写），
    // 旧手写 8 token 中文 map 已删除；悬空 token pending 不在取值域内
    expect(CUSTOM_ORDER_STATUSES).toHaveLength(11);
    expect(CUSTOM_ORDER_STATUSES).toContain('lab_dip');
    expect(CUSTOM_ORDER_STATUSES).toContain('quotation');
    expect(CUSTOM_ORDER_STATUSES).toContain('change_pending');
    expect(CUSTOM_ORDER_STATUSES).not.toContain('pending');
  });

  it('OA 公告 api 导出契约（列表/发布）', () => {
    expect(typeof listOaAnnouncements).toBe('function');
    expect(typeof publishOaAnnouncement).toBe('function');
  });

  it('验布状态映射', () => {
    expect(INSPECTION_STATUS_LABEL.pending).toBe('待验布');
    expect(INSPECTION_STATUS_LABEL.graded).toBe('已定级');
  });

  it('工资单状态映射', () => {
    expect(WAGE_RECORD_STATUS_LABEL.calculated).toBe('已核算');
    expect(WAGE_RECORD_STATUS_LABEL.paid).toBe('已发放');
  });

  it('委外状态映射', () => {
    expect(OUTSOURCING_STATUS_LABEL.issued).toBe('已发出');
    expect(OUTSOURCING_STATUS_LABEL.settled).toBe('已结算');
  });

  it('8D 阶段序列与后端状态机逐一相等', () => {
    // 取值来源：backend/src/services/quality_8d_service.rs EightDStatus::as_str()（11 态）。
    // 这里锁定完整序列而非单点 contains：前端一旦漂移回 d0~d8 之类的简写，
    // 列表按状态取推进边会全部落空，「推进下一阶段」对真实报告恒不可用。
    expect(QUALITY_8D_STAGES).toEqual([
      'not_started',
      'd0_plan',
      'd1_team',
      'd2_problem',
      'd3_interim',
      'd4_root_cause',
      'd5_permanent',
      'd6_verify',
      'd7_prevent',
      'd8_recognize',
      'closed',
    ]);
  });
});
