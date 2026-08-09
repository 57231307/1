/**
 * 面料 store 测试夹具
 * batch-20 P3: fixtures 测试夹具补充
 */

export const mockFabric = {
  id: 1,
  code: 'FAB-001',
  name: '纯棉坯布',
  category: '坯布',
  composition: '100%棉',
  width: 150,
  weight: 200,
  unit: '米',
  price: 15.5,
  status: 'active',
  created_at: '2026-08-01T10:00:00Z',
  updated_at: '2026-08-01T10:00:00Z',
};

export const mockFabricList = [
  mockFabric,
  {
    ...mockFabric,
    id: 2,
    code: 'FAB-002',
    name: '涤纶面料',
    composition: '100%涤纶',
  },
];

export const mockCreateFabricDto = {
  code: 'FAB-003',
  name: '混纺面料',
  category: '坯布',
  composition: '65%涤纶 35%棉',
  width: 150,
  weight: 180,
  unit: '米',
  price: 18.0,
};

export const mockUpdateFabricDto = {
  name: '混纺面料（更新）',
  price: 19.0,
};
