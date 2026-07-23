//! SearchSyncer 业务同步方法实现（new 构造器留在 facade）
use crate::search::elastic::{
    indices, CustomerDoc, ProductDoc, SalesOrderDoc, SearchError, SearchSyncer,
};

impl SearchSyncer {
    /// 同步销售订单到 ES
    pub async fn sync_sales_order(&self, doc: &SalesOrderDoc) -> Result<(), SearchError> {
        let value = serde_json::to_value(doc).map_err(|e| SearchError::Serialize(e.to_string()))?;
        self.client
            .index_doc(indices::SALES_ORDERS, &doc.order_no, &value)
            .await
    }

    /// 删除销售订单 ES 文档（硬删除，与客户软删除不同）
    pub async fn delete_sales_order(&self, order_no: &str) -> Result<(), SearchError> {
        self.client
            .delete_doc(indices::SALES_ORDERS, order_no)
            .await
    }

    /// 同步客户到 ES
    pub async fn sync_customer(&self, doc: &CustomerDoc) -> Result<(), SearchError> {
        let id = doc.id.to_string();
        let value = serde_json::to_value(doc).map_err(|e| SearchError::Serialize(e.to_string()))?;
        self.client.index_doc(indices::CUSTOMERS, &id, &value).await
    }

    /// 同步产品到 ES
    pub async fn sync_product(&self, doc: &ProductDoc) -> Result<(), SearchError> {
        let id = doc.id.to_string();
        let value = serde_json::to_value(doc).map_err(|e| SearchError::Serialize(e.to_string()))?;
        self.client.index_doc(indices::PRODUCTS, &id, &value).await
    }

    /// 删除产品 ES 文档（硬删除）
    pub async fn delete_product(&self, product_id: i32) -> Result<(), SearchError> {
        let id = product_id.to_string();
        self.client.delete_doc(indices::PRODUCTS, &id).await
    }
}
