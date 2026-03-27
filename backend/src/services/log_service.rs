use chrono::{Utc, NaiveDate, TimeZone};
// ��־��������
// �ṩ������־��ϵͳ��־����¼��־��API ������־�ļ�¼�Ͳ�ѯ����

use std::sync::Arc;
use chrono::Utc;
use sea_orm::{Set, PaginatorTrait, QuerySelect, QueryFilter, ActiveModelTrait, ColumnTrait, EntityTrait, Order};
use crate::models::{
    operation_log, log_system, log_login, log_api_access
};
use serde::{Deserialize, Serialize};

// ����ģ��
#[derive(Debug, Deserialize, Serialize)]
pub struct LogOperationRequest {
    pub module: String,
    pub operation_type: String,
    pub operation_desc: String,
    pub business_type: Option<String>,
    pub business_id: Option<i32>,
    pub business_no: Option<String>,
    pub user_id: i32,
    pub username: String,
    pub real_name: Option<String>,
    pub department_id: Option<i32>,
    pub department_name: Option<String>,
    pub request_method: Option<String>,
    pub request_url: Option<String>,
    pub request_params: Option<serde_json::Value>,
    pub request_body: Option<serde_json::Value>,
    pub response_status: Option<i32>,
    pub response_body: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub ip_location: Option<String>,
    pub user_agent: Option<String>,
    pub device_type: Option<String>,
    pub browser: Option<String>,
    pub os: Option<String>,
    pub duration_ms: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LogSystemRequest {
    pub log_level: String,
    pub logger_name: String,
    pub message: String,
    pub exception_type: Option<String>,
    pub exception_message: Option<String>,
    pub stack_trace: Option<String>,
    pub log_data: Option<serde_json::Value>,
    pub thread_name: Option<String>,
    pub thread_id: Option<i64>,
    pub file_name: Option<String>,
    pub method_name: Option<String>,
    pub line_number: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LogLoginRequest {
    pub user_id: Option<i32>,
    pub username: String,
    pub real_name: Option<String>,
    pub login_status: String,
    pub failure_reason: Option<String>,
    pub login_type: Option<String>,
    pub ip_address: Option<String>,
    pub ip_location: Option<String>,
    pub user_agent: Option<String>,
    pub device_type: Option<String>,
    pub browser: Option<String>,
    pub os: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LogApiAccessRequest {
    pub request_id: String,
    pub request_method: String,
    pub request_url: String,
    pub request_path: Option<String>,
    pub query_params: Option<serde_json::Value>,
    pub request_headers: Option<serde_json::Value>,
    pub request_body: Option<String>,
    pub content_type: Option<String>,
    pub response_status: i32,
    pub response_headers: Option<serde_json::Value>,
    pub response_body: Option<String>,
    pub response_size: Option<i64>,
    pub duration_ms: i32,
    pub db_query_count: Option<i32>,
    pub db_query_time_ms: Option<i32>,
    pub client_ip: String,
    pub client_location: Option<String>,
    pub user_agent: String,
    pub client_type: Option<String>,
    pub user_id: Option<i32>,
    pub username: Option<String>,
    pub auth_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LogQuery {
    pub module: Option<String>,
    pub operation_type: Option<String>,
    pub user_id: Option<i32>,
    pub username: Option<String>,
    pub business_type: Option<String>,
    pub business_id: Option<i32>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub page: Option<i32>,
    pub page_size: Option<i32>,
}

// ���ģ��
#[derive(Debug, Serialize)]
pub struct LogOperationInfo {
    pub id: i64,
    pub log_no: String,
    pub module: String,
    pub operation_type: String,
    pub operation_desc: String,
    pub business_type: Option<String>,
    pub business_id: Option<i32>,
    pub business_no: Option<String>,
    pub user_id: i32,
    pub username: String,
    pub real_name: Option<String>,
    pub operation_time: chrono::DateTime<chrono::Utc>,
    pub duration_ms: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct LogSystemInfo {
    pub id: i64,
    pub log_no: String,
    pub log_level: String,
    pub logger_name: String,
    pub message: String,
    pub exception_type: Option<String>,
    pub exception_message: Option<String>,
    pub log_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct LogLoginInfo {
    pub id: i64,
    pub log_no: String,
    pub user_id: Option<i32>,
    pub username: String,
    pub real_name: Option<String>,
    pub login_status: String,
    pub failure_reason: Option<String>,
    pub login_type: Option<String>,
    pub ip_address: Option<String>,
    pub ip_location: Option<String>,
    pub user_agent: Option<String>,
    pub browser: Option<String>,
    pub os: Option<String>,
    pub login_time: chrono::DateTime<chrono::Utc>,
    pub logout_time: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize)]
pub struct LogApiAccessInfo {
    pub id: i64,
    pub log_no: String,
    pub request_id: String,
    pub request_method: String,
    pub request_url: String,
    pub request_path: Option<String>,
    pub response_status: i32,
    pub duration_ms: i32,
    pub client_ip: String,
    pub client_type: Option<String>,
    pub user_id: Option<i32>,
    pub username: Option<String>,
    pub access_time: chrono::DateTime<chrono::Utc>,
}

pub struct LogService {
    pub db: Arc<DatabaseConnection>,
}

impl LogService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// ��¼������־
    pub async fn log_operation(&self, req: LogOperationRequest) -> Result<String, DbErr> {
        let db = &*self.db;

        let log_no = format!("OP{}{:010}", 
            Utc::now().format("%Y%m%d"), 
            self.generate_sequence_number().await?
        );

        let log = operation_log::ActiveModel {
            id: Set(0), // ���ݿ���??            log_no: Set(log_no.clone()),
            user_id: Set(Some(req.user_id)),
            username: Set(Some(req.username)),
            real_name: Set(req.real_name),
            module: Set(req.module),
            operation_type: Set(req.operation_type),
            operation_desc: Set(Some(req.operation_desc)),
            business_type: Set(req.business_type),
            business_id: Set(req.business_id),
            business_no: Set(req.business_no),
            request_method: Set(req.request_method),
            request_url: Set(req.request_url),
            request_params: Set(req.request_params),
            request_body: Set(req.request_body),
            response_status: Set(req.response_status),
            response_body: Set(req.response_body),
            ip_address: Set(req.ip_address),
            ip_location: Set(req.ip_location),
            user_agent: Set(req.user_agent),
            device_type: Set(req.device_type),
            browser: Set(req.browser),
            os: Set(req.os),
            duration_ms: Set(req.duration_ms),
            operation_time: Set(Utc::now()),
            created_at: Set(Utc::now()),
        };

        log.insert(db).await?;
        Ok(log_no)
    }

    /// ��¼ϵͳ��־
    pub async fn log_system(&self, req: LogSystemRequest) -> Result<String, DbErr> {
        let db = &*self.db;

        let log_no = format!("SYS{}{:010}", 
            Utc::now().format("%Y%m%d"), 
            self.generate_sequence_number().await?
        );

        let log = log_system::ActiveModel {
            log_no: Set(log_no.clone()),
            log_level: Set(req.log_level),
            logger_name: Set(req.logger_name),
            message: Set(req.message),
            exception_type: Set(req.exception_type),
            exception_message: Set(req.exception_message),
            stack_trace: Set(req.stack_trace),
            log_data: Set(req.log_data),
            thread_name: Set(req.thread_name),
            thread_id: Set(req.thread_id),
            file_name: Set(req.file_name),
            method_name: Set(req.method_name),
            line_number: Set(req.line_number),
            log_time: Set(Utc::now()),
            created_at: Set(Utc::now()),
            ..Default::default()
        };

        log.insert(db).await?;
        Ok(log_no)
    }

    /// ��¼��¼��־
    pub async fn log_login(&self, req: LogLoginRequest) -> Result<String, DbErr> {
        let db = &*self.db;

        let log_no = format!("LOG{}{:010}", 
            Utc::now().format("%Y%m%d"), 
            self.generate_sequence_number().await?
        );

        let log = log_login::ActiveModel {
            log_no: Set(log_no.clone()),
            user_id: Set(req.user_id),
            username: Set(req.username),
            real_name: Set(req.real_name),
            login_status: Set(req.login_status),
            failure_reason: Set(req.failure_reason),
            login_type: Set(req.login_type),
            ip_address: Set(req.ip_address),
            ip_location: Set(req.ip_location),
            user_agent: Set(req.user_agent),
            device_type: Set(req.device_type),
            browser: Set(req.browser),
            os: Set(req.os),
            login_time: Set(Utc::now()),
            created_at: Set(Utc::now()),
            ..Default::default()
        };

        log.insert(db).await?;
        Ok(log_no)
    }

    /// ��¼ API ������־
    pub async fn log_api_access(&self, req: LogApiAccessRequest) -> Result<String, DbErr> {
        let db = &*self.db;

        let log_no = format!("API{}{:010}", 
            Utc::now().format("%Y%m%d"), 
            self.generate_sequence_number().await?
        );

        let log = log_api_access::ActiveModel {
            log_no: Set(log_no.clone()),
            request_id: Set(req.request_id),
            request_method: Set(req.request_method),
            request_url: Set(req.request_url),
            request_path: Set(req.request_path),
            query_params: Set(req.query_params),
            request_headers: Set(req.request_headers),
            request_body: Set(req.request_body),
            content_type: Set(req.content_type),
            response_status: Set(req.response_status),
            response_headers: Set(req.response_headers),
            response_body: Set(req.response_body),
            response_size: Set(req.response_size),
            duration_ms: Set(req.duration_ms),
            db_query_count: Set(req.db_query_count),
            db_query_time_ms: Set(req.db_query_time_ms),
            client_ip: Set(req.client_ip),
            client_location: Set(req.client_location),
            user_agent: Set(req.user_agent),
            client_type: Set(req.client_type),
            user_id: Set(req.user_id),
            username: Set(req.username),
            auth_type: Set(req.auth_type),
            access_time: Set(Utc::now()),
            created_at: Set(Utc::now()),
            ..Default::default()
        };

        log.insert(db).await?;
        Ok(log_no)
    }

    /// ��ѯ������־
    pub async fn query_operation_logs(&self, query: LogQuery) -> Result<Vec<LogOperationInfo>, DbErr> {
        let db = &*self.db;

        let mut query_builder = log_operation::Entity::find();

        if let Some(module) = query.module {
            query_builder = query_builder.filter(log_operation::Column::Module.eq(module));
        }

        if let Some(operation_type) = query.operation_type {
            query_builder = query_builder.filter(log_operation::Column::OperationType.eq(operation_type));
        }

        if let Some(user_id) = query.user_id {
            query_builder = query_builder.filter(log_operation::Column::UserId.eq(user_id));
        }

        if let Some(username) = query.username {
            query_builder = query_builder.filter(log_operation::Column::Username.eq(username));
        }

        if let Some(business_type) = query.business_type {
            query_builder = query_builder.filter(log_operation::Column::BusinessType.eq(business_type));
        }

        if let Some(business_id) = query.business_id {
            query_builder = query_builder.filter(log_operation::Column::BusinessId.eq(business_id));
        }

        if let Some(start_date) = query.start_date {
            let start_datetime = Utc.from_utc_datetime(&start_date.and_hms_opt(0, 0, 0).unwrap());
            query_builder = query_builder.filter(
                log_operation::Column::OperationTime.gte(start_datetime)
            );
        }

        if let Some(end_date) = query.end_date {
            let end_datetime = Utc.from_utc_datetime(&end_date.and_hms_opt(23, 59, 59).unwrap());
            query_builder = query_builder.filter(
                log_operation::Column::OperationTime.lte(end_datetime)
            );
        }

        query_builder = query_builder.order_by(log_operation::Column::OperationTime, Order::Desc);

        if let Some(page) = query.page {
            let page_size = query.page_size.unwrap_or(20);
            let offset = (page - 1) * page_size;
            query_builder = query_builder
                .offset(offset as u64)
                .limit(page_size as u64);
        }

        let logs = query_builder.all(db).await?;

        Ok(logs.into_iter().map(|log| LogOperationInfo {
            id: log.id,
            log_no: log.log_no,
            module: log.module,
            operation_type: log.operation_type,
            operation_desc: log.operation_desc,
            business_type: log.business_type,
            business_id: log.business_id,
            business_no: log.business_no,
            user_id: log.user_id,
            username: log.username,
            real_name: log.real_name,
            operation_time: log.operation_time,
            duration_ms: log.duration_ms,
        }).collect())
    }

    /// ��ѯϵͳ��־
    pub async fn query_system_logs(
        &self,
        log_level: Option<String>,
        logger_name: Option<String>,
        start_date: Option<chrono::NaiveDate>,
        end_date: Option<chrono::NaiveDate>,
        page: Option<i32>,
        page_size: Option<i32>,
    ) -> Result<Vec<LogSystemInfo>, DbErr> {
        let db = &*self.db;

        let mut query_builder = log_system::Entity::find();

        if let Some(log_level) = log_level {
            query_builder = query_builder.filter(log_system::Column::LogLevel.eq(log_level));
        }

        if let Some(logger_name) = logger_name {
            query_builder = query_builder.filter(log_system::Column::LoggerName.eq(logger_name));
        }

        if let Some(start_date) = start_date {
            let start_datetime = Utc.from_utc_datetime(&start_date.and_hms_opt(0, 0, 0).unwrap());
            query_builder = query_builder.filter(
                log_system::Column::LogTime.gte(start_datetime)
            );
        }

        if let Some(end_date) = end_date {
            let end_datetime = Utc.from_utc_datetime(&end_date.and_hms_opt(23, 59, 59).unwrap());
            query_builder = query_builder.filter(
                log_system::Column::LogTime.lte(end_datetime)
            );
        }

        query_builder = query_builder.order_by(log_system::Column::LogTime, Order::Desc);

        if let Some(page) = page {
            let page_size = page_size.unwrap_or(20);
            let offset = (page - 1) * page_size;
            query_builder = query_builder
                .offset(offset as u64)
                .limit(page_size as u64);
        }

        let logs = query_builder.all(db).await?;

        Ok(logs.into_iter().map(|log| LogSystemInfo {
            id: log.id,
            log_no: log.log_no,
            log_level: log.log_level,
            logger_name: log.logger_name,
            message: log.message,
            exception_type: log.exception_type,
            exception_message: log.exception_message,
            log_time: log.log_time,
        }).collect())
    }

    /// ��ѯ��¼��־
    pub async fn query_login_logs(
        &self,
        username: Option<String>,
        login_status: Option<String>,
        start_date: Option<chrono::NaiveDate>,
        end_date: Option<chrono::NaiveDate>,
        page: Option<i32>,
        page_size: Option<i32>,
    ) -> Result<Vec<LogLoginInfo>, DbErr> {
        let db = &*self.db;

        let mut query_builder = log_login::Entity::find();

        if let Some(username) = username {
            query_builder = query_builder.filter(log_login::Column::Username.eq(username));
        }

        if let Some(login_status) = login_status {
            query_builder = query_builder.filter(log_login::Column::LoginStatus.eq(login_status));
        }

        if let Some(start_date) = start_date {
            let start_datetime = start_date.and_hms_opt(0, 0, 0).unwrap();
            query_builder = query_builder.filter(
                log_login::Column::LoginTime.gte(start_datetime.and_utc())
            );
        }

        if let Some(end_date) = end_date {
            let end_datetime = end_date.and_hms_opt(23, 59, 59).unwrap();
            query_builder = query_builder.filter(
                log_login::Column::LoginTime.lte(end_datetime.and_utc())
            );
        }

        query_builder = query_builder.order_by(log_login::Column::LoginTime, Order::Desc);

        if let Some(page) = page {
            let page_size = page_size.unwrap_or(20);
            let offset = (page - 1) * page_size;
            query_builder = query_builder
                .offset(offset as u64)
                .limit(page_size as u64);
        }

        let logs = query_builder.all(db).await?;

        Ok(logs.into_iter().map(|log| LogLoginInfo {
            id: log.id,
            log_no: log.log_no,
            user_id: log.user_id,
            username: log.username,
            real_name: log.real_name,
            login_status: log.login_status,
            failure_reason: log.failure_reason,
            login_type: log.login_type,
            ip_address: log.ip_address,
            ip_location: log.ip_location,
            user_agent: log.user_agent,
            browser: log.browser,
            os: log.os,
            login_time: log.login_time,
            logout_time: log.logout_time,
        }).collect())
    }

    /// ��ѯ API ������־
    pub async fn query_api_logs(
        &self,
        request_method: Option<String>,
        response_status: Option<i32>,
        client_ip: Option<String>,
        user_id: Option<i32>,
        start_date: Option<chrono::NaiveDate>,
        end_date: Option<chrono::NaiveDate>,
        page: Option<i32>,
        page_size: Option<i32>,
    ) -> Result<Vec<LogApiAccessInfo>, DbErr> {
        let db = &*self.db;

        let mut query_builder = log_api_access::Entity::find();

        if let Some(request_method) = request_method {
            query_builder = query_builder.filter(log_api_access::Column::RequestMethod.eq(request_method));
        }

        if let Some(response_status) = response_status {
            query_builder = query_builder.filter(log_api_access::Column::ResponseStatus.eq(response_status));
        }

        if let Some(client_ip) = client_ip {
            query_builder = query_builder.filter(log_api_access::Column::ClientIp.eq(client_ip));
        }

        if let Some(user_id) = user_id {
            query_builder = query_builder.filter(log_api_access::Column::UserId.eq(user_id));
        }

        if let Some(start_date) = start_date {
            let start_datetime = start_date.and_hms_opt(0, 0, 0).unwrap();
            query_builder = query_builder.filter(
                log_api_access::Column::AccessTime.gte(start_datetime.and_utc())
            );
        }

        if let Some(end_date) = end_date {
            let end_datetime = end_date.and_hms_opt(23, 59, 59).unwrap();
            query_builder = query_builder.filter(
                log_api_access::Column::AccessTime.lte(end_datetime.and_utc())
            );
        }

        query_builder = query_builder.order_by(log_api_access::Column::AccessTime, Order::Desc);

        if let Some(page) = page {
            let page_size = page_size.unwrap_or(20);
            let offset = (page - 1) * page_size;
            query_builder = query_builder
                .offset(offset as u64)
                .limit(page_size as u64);
        }

        let logs = query_builder.all(db).await?;

        Ok(logs.into_iter().map(|log| LogApiAccessInfo {
            id: log.id,
            log_no: log.log_no,
            request_id: log.request_id,
            request_method: log.request_method,
            request_url: log.request_url,
            request_path: log.request_path,
            response_status: log.response_status,
            duration_ms: log.duration_ms,
            client_ip: log.client_ip,
            client_type: log.client_type,
            user_id: log.user_id,
            username: log.username,
            access_time: log.access_time,
        }).collect())
    }

    /// ��������??    async fn generate_sequence_number(&self) -> Result<i64, DbErr> {
        Ok((Utc::now().timestamp() % 10000000000) as i64)
    }
}
