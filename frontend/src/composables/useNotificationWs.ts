import { WebSocketClient, type NotificationPayload } from '@/utils/websocket';
import { getWsTicket, type WsTicketResponse } from '@/api/notification';

/**
 * 通知实时推送 composable（真实接线）
 *
 * 后端能力（均已就绪）：
 * - POST /api/v1/erp/ws/ticket：一次性短时票据签发（JWT 认证）
 * - GET /api/v1/erp/ws/notifications?ticket=xxx：通知实时推送 + 心跳
 * - 广播器：业务事件（新通知/仪表盘更新）触发推送
 *
 * 本 composable 将 utils/websocket.ts 的 WebSocketClient 接入主布局：
 * 登录后建立连接，收到新通知时回调监听器（供 UI 弹提示/刷新未读数）。
 */

let client: WebSocketClient | null = null;
const listeners = new Set<(payload: NotificationPayload) => void>();

function buildWsUrl(): string {
  const base = import.meta.env.VITE_API_BASE_URL || '/api/v1/erp';
  const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${proto}//${window.location.host}${base}/ws/notifications`;
}

export function startNotificationWs(): void {
  if (client) return;

  const ticketFetcher = async () => {
    const res = await getWsTicket();
    const data = res as unknown as WsTicketResponse;
    if (!data?.ticket) throw new Error('WS 票据获取失败');
    return data.ticket;
  };

  client = new WebSocketClient(buildWsUrl(), ticketFetcher);

  client.addEventListener('notification', event => {
    const detail = (event as CustomEvent<{ data: NotificationPayload }>).detail;
    if (detail?.data) {
      listeners.forEach(fn => fn(detail.data));
    }
  });

  client.connect();
}

export function stopNotificationWs(): void {
  client?.disconnect();
  client = null;
  listeners.clear();
}

/** 订阅新通知事件（用于 UI 提示与未读数刷新）；返回取消订阅函数 */
export function onNewNotification(fn: (payload: NotificationPayload) => void): () => void {
  listeners.add(fn);
  return () => listeners.delete(fn);
}
