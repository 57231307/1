with open("src/services/fund_management_service.rs", "r") as f:
    content = f.read()

# Add CrudService import if not there
if "use crate::services::crud_service::CrudService;" not in content:
    content = content.replace("use crate::services::api::ApiService;", "use crate::services::api::ApiService;\nuse crate::services::crud_service::CrudService;")

# Implement CrudService
if "impl CrudService for FundManagementService" not in content:
    impl_crud = """
impl CrudService for FundManagementService {
    type Model = FundAccount;
    type ListResponse = FundAccountListResponse;
    type CreateRequest = CreateFundAccountRequest;
    type UpdateRequest = ();

    fn base_path() -> &'static str {
        "/fund-accounts"
    }
}
"""
    content = content.replace("pub struct FundManagementService;", "pub struct FundManagementService;\n" + impl_crud)

# Replace list_accounts, get_account, create_account, delete_account
replacement = """    /// 获取资金账户列表
    pub async fn list_accounts(params: FundAccountQueryParams) -> Result<Vec<FundAccount>, String> {
        let resp = <Self as CrudService>::list_with_query(&params).await?;
        Ok(resp.data)
    }

    /// 获取资金账户详情
    pub async fn get_account(id: i32) -> Result<FundAccount, String> {
        <Self as CrudService>::get(id).await
    }

    /// 创建资金账户
    pub async fn create_account(req: CreateFundAccountRequest) -> Result<FundAccount, String> {
        <Self as CrudService>::create(req).await
    }"""

start_idx = content.find("    /// 获取资金账户列表")
end_idx = content.find("    /// 账户存款")
if start_idx != -1 and end_idx != -1:
    content = content[:start_idx] + replacement + "\n\n" + content[end_idx:]

with open("src/services/fund_management_service.rs", "w") as f:
    f.write(content)
