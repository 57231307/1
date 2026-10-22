/**
 * 大额资金调拨阈值（元）的前端单一来源。
 *
 * 权威门控在后端：fund_management_service 的 large_transfer_threshold 定义同一数值，
 * 并由 TransferFundRequest.confirm_large 在提交时强制校验，前端常量只决定
 * 何时弹出二次确认交互，不做放行判断（后端仍是最终事实源，两侧数值必须同步修改）。
 * 除本文件外任何视图不得内联重复该数值做各自判断。
 */
export const LARGE_TRANSFER_THRESHOLD = 100000;
