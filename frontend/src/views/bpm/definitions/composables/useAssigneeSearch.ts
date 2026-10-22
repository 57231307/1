/**
 * useAssigneeSearch.ts - 流程定义表单的审批人候选检索
 *
 * 为什么按 keyword 远程搜索、而不是拉一次大清单：
 * 后端列出用户（handlers/user_handler.rs 的 list_users）对 page_size 做钳制
 * （page_size.unwrap_or(20).clamp(1, 100)），传大于 100 会被静默削成 100——
 * 把大 page_size 当全量是假命题，第 101 个之后的用户永远选不到当审批人。
 * 唯一如实的取法是靠 keyword 收窄：list_users 支持 keyword 模糊匹配
 * username / email / phone，每页取前 50 条候选。
 *
 * 注：keyword 不匹配真实姓名 real_name（后端检索列为 username/email/phone），
 * 界面提示文案按此如实说明可搜索字段。
 */
import { getUserList, type User } from '@/api/user';

/** 单次检索返回的候选上限（后端 clamp 1-100，取 50 与既有分页范式一致） */
export const ASSIGNEE_SEARCH_PAGE_SIZE = 50;

/**
 * 按关键字检索审批人候选；keyword 为空串时返回第一页（request.ts serializeParams
 * 与后端 normalize_empty_query_params 双侧剔除空串参数，等价于不筛选）。
 * 失败原样上抛，由调用方清空该行列表并显式报错——本函数不吞错、不返回空列表兜底，
 * 否则会把"取不到人"掩盖成"没有这个人"。
 */
export async function fetchUserCandidates(keyword: string): Promise<User[]> {
  const res = await getUserList({
    keyword,
    page: 1,
    page_size: ASSIGNEE_SEARCH_PAGE_SIZE,
  });
  return res.data.users;
}
