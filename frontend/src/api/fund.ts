
export const listFundTransfers = (params?: QueryParams) =>
  request.get('/fund-management/transfers', { params })

export const getFundTransfer = (id: number) =>
  request.get(`/fund-management/transfers/${id}`)
