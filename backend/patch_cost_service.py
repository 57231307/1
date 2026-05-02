import re

with open('backend/src/services/cost_collection_service.rs', 'r') as f:
    content = f.read()

update_req = '''pub struct UpdateCostCollectionRequest {
    pub collection_date: Option<NaiveDate>,
    pub direct_material: Option<Decimal>,
    pub direct_labor: Option<Decimal>,
    pub manufacturing_overhead: Option<Decimal>,
    pub processing_fee: Option<Decimal>,
    pub dyeing_fee: Option<Decimal>,
    pub output_quantity_meters: Option<Decimal>,
    pub output_quantity_kg: Option<Decimal>,
}'''

if 'UpdateCostCollectionRequest' not in content:
    content = content.replace('pub struct CreateCostCollectionRequest {', update_req + '\n\n#[derive(Debug, Deserialize)]\npub struct CreateCostCollectionRequest {')

new_funcs = '''
    pub async fn update(&self, id: i32, req: UpdateCostCollectionRequest, _user_id: i32) -> Result<cost_collection::Model, AppError> {
        let collection = cost_collection::Entity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("成本归集记录不存在".to_string()))?;

        let mut active_collection: cost_collection::ActiveModel = collection.into();
        
        if let Some(date) = req.collection_date { active_collection.collection_date = Set(date); }
        if let Some(amt) = req.direct_material { active_collection.direct_material = Set(amt); }
        if let Some(amt) = req.direct_labor { active_collection.direct_labor = Set(amt); }
        if let Some(amt) = req.manufacturing_overhead { active_collection.manufacturing_overhead = Set(amt); }
        if let Some(amt) = req.processing_fee { active_collection.processing_fee = Set(amt); }
        if let Some(amt) = req.dyeing_fee { active_collection.dyeing_fee = Set(amt); }
        if let Some(amt) = req.output_quantity_meters { active_collection.output_quantity_meters = Set(Some(amt)); }
        if let Some(amt) = req.output_quantity_kg { active_collection.output_quantity_kg = Set(Some(amt)); }
        
        // Recalculate total cost
        let dm = req.direct_material.unwrap_or(*active_collection.direct_material.as_ref());
        let dl = req.direct_labor.unwrap_or(*active_collection.direct_labor.as_ref());
        let mo = req.manufacturing_overhead.unwrap_or(*active_collection.manufacturing_overhead.as_ref());
        let pf = req.processing_fee.unwrap_or(*active_collection.processing_fee.as_ref());
        let df = req.dyeing_fee.unwrap_or(*active_collection.dyeing_fee.as_ref());
        active_collection.total_cost = Set(dm + dl + mo + pf + df);

        // Recalculate unit costs
        let total = active_collection.total_cost.as_ref();
        if let Some(Some(meters)) = active_collection.output_quantity_meters.as_ref() {
            if *meters > Decimal::ZERO {
                active_collection.unit_cost_meters = Set(Some(total / meters));
            }
        }
        if let Some(Some(kg)) = active_collection.output_quantity_kg.as_ref() {
            if *kg > Decimal::ZERO {
                active_collection.unit_cost_kg = Set(Some(total / kg));
            }
        }

        active_collection.updated_at = Set(Utc::now());

        let result = active_collection.update(&*self.db).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(result)
    }

    pub async fn delete(&self, id: i32, _user_id: i32) -> Result<(), AppError> {
        let collection = cost_collection::Entity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("成本归集记录不存在".to_string()))?;

        cost_collection::Entity::delete_by_id(id)
            .exec(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }
'''

if 'pub async fn delete' not in content:
    content = content.rsplit('}', 1)[0] + new_funcs + '}\n'

with open('backend/src/services/cost_collection_service.rs', 'w') as f:
    f.write(content)
