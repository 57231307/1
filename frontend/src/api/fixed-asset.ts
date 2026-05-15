
export const batchDepreciateAssets = (data: { asset_ids: number[]; calculation_date: string; user_id: number }) =>
  request.post('/fixed-assets/batch-depreciate', data)
