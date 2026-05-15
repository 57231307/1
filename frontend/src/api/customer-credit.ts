
export interface CreditEvaluationRequest {
  customer_id: number
  evaluation_date: string
}

export interface CreditEvaluationResult {
  customer_id: number
  customer_name: string
  credit_score: number
  credit_rating: string
  recommended_limit: number
  evaluation_factors: Array<{
    factor_name: string
    weight: number
    score: number
    description: string
  }>
  evaluation_date: string
}

export const evaluateCustomerCredit = (data: CreditEvaluationRequest) =>
  request.post('/customer-credits/evaluate', data)
