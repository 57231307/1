//! ElasticClient 文档操作与 SearchClient trait 实现（构造器留在 facade）
use async_trait::async_trait;
use std::collections::HashMap;

use crate::search::elastic::{
    ClientInner, ElasticClient, SearchClient, SearchError, SearchHit, SearchQuery, SearchResult,
};

impl ElasticClient {
    /// 已索引文档数（mock 内存计数 / real ES _count API）
    pub async fn doc_count(&self, index: &str) -> usize {
        match &self.inner {
            ClientInner::Mock(storage) => {
                storage
                    .lock()
                    .await
                    .get(index)
                    .map(|m| m.len())
                    .unwrap_or(0)
            }
            ClientInner::Real { base_url, http } => {
                let url = format!("{}/{}/_count", base_url, index);
                match http.get(&url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        let body: serde_json::Value = resp.json().await.unwrap_or_default();
                        body.get("count")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as usize
                    }
                    _ => 0,
                }
            }
        }
    }
}

#[async_trait]
impl SearchClient for ElasticClient {
    async fn index_doc(
        &self,
        index: &str,
        id: &str,
        doc: &serde_json::Value,
    ) -> Result<(), SearchError> {
        match &self.inner {
            ClientInner::Mock(storage) => {
                let mut storage = storage.lock().await;
                storage
                    .entry(index.to_string())
                    .or_insert_with(HashMap::new)
                    .insert(id.to_string(), doc.clone());
                Ok(())
            }
            ClientInner::Real { base_url, http } => {
                let url = format!("{}/{}/_doc/{}", base_url, index, id);
                let resp = http
                    .put(&url)
                    .json(doc)
                    .send()
                    .await
                    .map_err(|e| SearchError::Connection(format!("ES index_doc 请求失败: {}", e)))?;
                if !resp.status().is_success() {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    return Err(SearchError::Index(format!(
                        "ES index_doc 失败 (status={}): {}",
                        status, body
                    )));
                }
                Ok(())
            }
        }
    }

    async fn search(
        &self,
        index: &str,
        query: &SearchQuery,
    ) -> Result<SearchResult<serde_json::Value>, SearchError> {
        match &self.inner {
            ClientInner::Mock(storage) => {
                let storage = storage.lock().await;
                let docs = storage.get(index).cloned().unwrap_or_default();

                let mut hits: Vec<SearchHit<serde_json::Value>> = docs
                    .iter()
                    .filter(|(_, v)| match &query.q {
                        Some(q) => serde_json::to_string(v)
                            .map(|s| s.contains(q))
                            .unwrap_or(false),
                        None => true,
                    })
                    .map(|(id, value)| SearchHit {
                        id: id.clone(),
                        score: 1.0,
                        source: value.clone(),
                        highlight: None,
                    })
                    .collect();

                let total = hits.len() as i64;
                let from = query.from.max(0) as usize;
                let size = query.size.max(0) as usize;
                let end = (from + size).min(hits.len());
                if from < hits.len() {
                    hits = hits.split_off(from);
                    hits.truncate(end - from);
                } else {
                    hits.clear();
                }

                Ok(SearchResult {
                    total,
                    hits,
                    took_ms: 1,
                })
            }
            ClientInner::Real { base_url, http } => {
                // 构建 ES Query DSL
                let mut body = serde_json::json!({
                    "from": query.from.max(0),
                    "size": query.size.max(0),
                });

                if let Some(q) = &query.q {
                    if !q.is_empty() {
                        body["query"] = serde_json::json!({
                            "multi_match": {
                                "query": q,
                                "fields": ["*"]
                            }
                        });
                    }
                }

                // 添加精确过滤条件
                if !query.filters.is_empty() {
                    let filters: Vec<serde_json::Value> = query
                        .filters
                        .iter()
                        .map(|(k, v)| serde_json::json!({ "term": { k: v } }))
                        .collect();
                    body["query"] = if body.get("query").is_some() {
                        let existing = body["query"].clone();
                        serde_json::json!({
                            "bool": {
                                "must": [existing],
                                "filter": filters
                            }
                        })
                    } else {
                        serde_json::json!({ "bool": { "filter": filters } })
                    };
                }

                if query.highlight {
                    body["highlight"] = serde_json::json!({
                        "fields": { "*": {} }
                    });
                }

                let url = format!("{}/{}/_search", base_url, index);
                let resp = http
                    .post(&url)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| SearchError::Connection(format!("ES search 请求失败: {}", e)))?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let err_body = resp.text().await.unwrap_or_default();
                    return Err(SearchError::Search(format!(
                        "ES search 失败 (status={}): {}",
                        status, err_body
                    )));
                }

                let result: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| SearchError::Search(format!("ES search 响应解析失败: {}", e)))?;

                let took_ms = result
                    .get("took")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let total = result
                    .get("hits")
                    .and_then(|h| h.get("total"))
                    .and_then(|t| t.get("value"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);

                let hits: Vec<SearchHit<serde_json::Value>> = result
                    .get("hits")
                    .and_then(|h| h.get("hits"))
                    .and_then(|h| h.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|hit| {
                                let id = hit
                                    .get("_id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let score = hit
                                    .get("_score")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0);
                                let source = hit.get("_source").cloned().unwrap_or_default();
                                let highlight = hit
                                    .get("highlight")
                                    .map(|h| serde_json::from_value(h.clone()).unwrap_or_default());
                                SearchHit {
                                    id,
                                    score,
                                    source,
                                    highlight,
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                Ok(SearchResult {
                    total,
                    hits,
                    took_ms,
                })
            }
        }
    }

    async fn delete_doc(&self, index: &str, id: &str) -> Result<(), SearchError> {
        match &self.inner {
            ClientInner::Mock(storage) => {
                let mut storage = storage.lock().await;
                if let Some(map) = storage.get_mut(index) {
                    map.remove(id);
                }
                Ok(())
            }
            ClientInner::Real { base_url, http } => {
                let url = format!("{}/{}/_doc/{}", base_url, index, id);
                let resp = http
                    .delete(&url)
                    .send()
                    .await
                    .map_err(|e| SearchError::Connection(format!("ES delete_doc 请求失败: {}", e)))?;
                // ES DELETE 返回 404 表示文档不存在，视为成功（幂等删除）
                if !resp.status().is_success() && resp.status().as_u16() != 404 {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    return Err(SearchError::Index(format!(
                        "ES delete_doc 失败 (status={}): {}",
                        status, body
                    )));
                }
                Ok(())
            }
        }
    }

    async fn bulk_index(
        &self,
        index: &str,
        docs: &[(String, serde_json::Value)],
    ) -> Result<usize, SearchError> {
        match &self.inner {
            ClientInner::Mock(_) => {
                // Mock 模式：逐条调用 index_doc 写入内存 HashMap
                let mut count = 0;
                for (id, doc) in docs {
                    self.index_doc(index, id, doc).await?;
                    count += 1;
                }
                Ok(count)
            }
            ClientInner::Real { base_url, http } => {
                // ES _bulk API 要求 NDJSON 格式：action_header\n source\n
                let mut body = String::new();
                for (id, doc) in docs {
                    let action = serde_json::json!({
                        "index": { "_index": index, "_id": id }
                    });
                    body.push_str(&action.to_string());
                    body.push('\n');
                    body.push_str(&doc.to_string());
                    body.push('\n');
                }

                let url = format!("{}/_bulk", base_url);
                let resp = http
                    .post(&url)
                    .header("Content-Type", "application/x-ndjson")
                    .body(body)
                    .send()
                    .await
                    .map_err(|e| {
                        SearchError::Connection(format!("ES bulk_index 请求失败: {}", e))
                    })?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let err_body = resp.text().await.unwrap_or_default();
                    return Err(SearchError::Index(format!(
                        "ES bulk_index 失败 (status={}): {}",
                        status, err_body
                    )));
                }

                let result: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| SearchError::Search(format!("ES bulk_index 响应解析失败: {}", e)))?;

                let count = result
                    .get("items")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter(|item| {
                                item.get("index")
                                    .and_then(|i| i.get("status"))
                                    .and_then(|s| s.as_i64())
                                    .map(|s| (200..300).contains(&s))
                                    .unwrap_or(false)
                            })
                            .count()
                    })
                    .unwrap_or(0);

                Ok(count)
            }
        }
    }

    /// trait 方法委托给 ElasticClient::doc_count 固有方法
    async fn doc_count(&self, index: &str) -> usize {
        ElasticClient::doc_count(self, index).await
    }
}
