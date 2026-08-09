/**
 * 色卡发放 store 测试夹具
 * batch-20 P3: fixtures 测试夹具补充
 */

export const mockIssueRecord = {
  id: 1,
  color_card_id: 1,
  customer_id: 1,
  issued_at: '2026-08-01T10:00:00Z',
  returned_at: null,
  status: 'issued',
  notes: '测试发放',
  created_at: '2026-08-01T10:00:00Z',
  updated_at: '2026-08-01T10:00:00Z',
};

export const mockIssueRecordList = [
  mockIssueRecord,
  {
    ...mockIssueRecord,
    id: 2,
    status: 'returned',
    returned_at: '2026-08-05T10:00:00Z',
  },
];

export const mockCreateIssueDto = {
  color_card_id: 1,
  customer_id: 1,
  notes: '测试发放',
};

export const mockReturnIssueDto = {
  notes: '测试归还',
};
