use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::models::user::Model as UserModel;
use crate::services::{
    auth_service::AuthService as AuthServiceImpl, user_service::UserService as UserServiceImpl,
};

// 导入生成的 gRPC 代码
pub mod proto {
    tonic::include_proto!("bingxi");
}

use proto::{
    auth_service_server::AuthService as AuthServiceTrait,
    user_service_server::UserService as UserServiceTrait, CreateUserRequest, CreateUserResponse,
    DeleteUserRequest, DeleteUserResponse, GetUserRequest, GetUserResponse, ListUsersRequest,
    ListUsersResponse, LoginRequest, LoginResponse, UpdateUserRequest, UpdateUserResponse, User,
    VerifyTokenRequest, VerifyTokenResponse,
};

/// gRPC 用户服务实现
#[derive(Clone)]
pub struct GrpcUserService {
    user_service: Arc<UserServiceImpl>,
    auth_service: Arc<AuthServiceImpl>,
}

impl GrpcUserService {
    pub fn new(db: Arc<DatabaseConnection>, jwt_secret: String) -> Self {
        Self {
            user_service: Arc::new(UserServiceImpl::new(db.clone())),
            auth_service: Arc::new(AuthServiceImpl::new(db, jwt_secret)),
        }
    }

    /// 将数据库用户模型转换为 gRPC 用户模型
    fn to_grpc_user(user: UserModel) -> User {
        User {
            id: user.id,
            username: user.username,
            email: user.email.unwrap_or_default(),
            phone: user.phone.unwrap_or_default(),
            role: user.role_id.map(|r| r.to_string()).unwrap_or_default(),
            status: if user.is_active { "active" } else { "inactive" }.to_string(),
            created_at: user.created_at.timestamp(),
            updated_at: user.updated_at.timestamp(),
            last_login_at: user.last_login_at.map(|t| t.timestamp()).unwrap_or(0),
        }
    }
}

#[tonic::async_trait]
impl UserServiceTrait for GrpcUserService {
    async fn get_user(
        &self,
        request: Request<GetUserRequest>,
    ) -> Result<Response<GetUserResponse>, Status> {
        let req = request.into_inner();

        match self.user_service.find_by_id(req.user_id).await {
            Ok(user) => Ok(Response::new(GetUserResponse {
                success: true,
                message: "用户获取成功".to_string(),
                user: Some(Self::to_grpc_user(user)),
            })),
            Err(e) => Err(Status::not_found(format!("用户不存在：{}", e))),
        }
    }

    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<CreateUserResponse>, Status> {
        let req = request.into_inner();

        // 密码哈希
        let password_hash = match AuthServiceImpl::hash_password(&req.password) {
            Ok(hash) => hash,
            Err(e) => {
                return Err(Status::internal(format!("密码加密失败：{}", e)));
            }
        };

        // 解析 role 字符串为 i32
        let role_id = req.role.parse::<i32>().unwrap_or(0);

        // 创建用户
        match self
            .user_service
            .create_user(
                req.username,
                password_hash,
                Some(req.email),
                Some(req.phone),
                Some(role_id),
                None,
            )
            .await
        {
            Ok(user) => Ok(Response::new(CreateUserResponse {
                success: true,
                message: "用户创建成功".to_string(),
                user: Some(Self::to_grpc_user(user)),
            })),
            Err(e) => Err(Status::internal(format!("创建用户失败：{}", e))),
        }
    }

    async fn update_user(
        &self,
        request: Request<UpdateUserRequest>,
    ) -> Result<Response<UpdateUserResponse>, Status> {
        let req = request.into_inner();

        // 先查找用户
        let user_id = req.user_id;
        match self.user_service.find_by_id(user_id).await {
            Ok(_existing_user) => {
                // 解析 role 和 status
                let role_id = req.role.parse::<i32>().ok();
                let is_active = req.status.to_lowercase() == "active";

                // 更新用户信息
                match self
                    .user_service
                    .update_user(
                        user_id,
                        None,
                        Some(req.phone),
                        role_id,
                        None, // department_id 已移除
                        Some(is_active.to_string()),
                    )
                    .await
                {
                    Ok(user) => Ok(Response::new(UpdateUserResponse {
                        success: true,
                        message: "用户更新成功".to_string(),
                        user: Some(Self::to_grpc_user(user)),
                    })),
                    Err(e) => Err(Status::internal(format!("更新用户失败：{}", e))),
                }
            }
            Err(_) => Err(Status::not_found("用户不存在")),
        }
    }

    async fn delete_user(
        &self,
        request: Request<DeleteUserRequest>,
    ) -> Result<Response<DeleteUserResponse>, Status> {
        let req = request.into_inner();

        match self.user_service.delete_user(req.user_id).await {
            Ok(_) => Ok(Response::new(DeleteUserResponse {
                success: true,
                message: "用户删除成功".to_string(),
            })),
            Err(e) => Err(Status::internal(format!("删除用户失败：{}", e))),
        }
    }

    async fn list_users(
        &self,
        request: Request<ListUsersRequest>,
    ) -> Result<Response<ListUsersResponse>, Status> {
        let req = request.into_inner();

        let page = req.page.max(1) as u64;
        let page_size = req.page_size.clamp(1, 100) as u64;

        match self.user_service.list_users(page, page_size).await {
            Ok((users, total)) => {
                let total_pages = ((total as f64) / (page_size as f64)).ceil() as i32;
                let grpc_users: Vec<User> = users.into_iter().map(Self::to_grpc_user).collect();

                Ok(Response::new(ListUsersResponse {
                    success: true,
                    message: "用户列表获取成功".to_string(),
                    users: grpc_users,
                    total: total as i32,
                    page: page as i32,
                    page_size: page_size as i32,
                    total_pages,
                }))
            }
            Err(e) => Err(Status::internal(format!("获取用户列表失败：{}", e))),
        }
    }
}

#[tonic::async_trait]
impl AuthServiceTrait for GrpcUserService {
    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let req = request.into_inner();

        match self
            .auth_service
            .authenticate(&req.username, &req.password)
            .await
        {
            Ok((token, user)) => Ok(Response::new(LoginResponse {
                success: true,
                message: "登录成功".to_string(),
                token,
                user: Some(Self::to_grpc_user(user)),
            })),
            Err(e) => Err(Status::unauthenticated(format!("登录失败：{}", e))),
        }
    }

    async fn verify_token(
        &self,
        request: Request<VerifyTokenRequest>,
    ) -> Result<Response<VerifyTokenResponse>, Status> {
        let req = request.into_inner();

        match self.auth_service.validate_token(&req.token) {
            Ok(claims) => {
                // 从 token 中获取用户 ID (sub 字段)
                match self.user_service.find_by_id(claims.sub).await {
                    Ok(user) => Ok(Response::new(VerifyTokenResponse {
                        success: true,
                        message: "Token 验证成功".to_string(),
                        valid: true,
                        user: Some(Self::to_grpc_user(user)),
                    })),
                    Err(_) => Ok(Response::new(VerifyTokenResponse {
                        success: false,
                        message: "用户不存在".to_string(),
                        valid: false,
                        user: None,
                    })),
                }
            }
            Err(_) => Ok(Response::new(VerifyTokenResponse {
                success: false,
                message: "Token 无效或已过期".to_string(),
                valid: false,
                user: None,
            })),
        }
    }
}
