use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use service_utils::service::types::ConfigVersionType;

#[derive(Deserialize, Clone)]
pub struct PutReq {
    pub context: Map<String, Value>,
    pub r#override: Map<String, Value>,
}

#[derive(Deserialize, Clone)]
pub struct MoveReq {
    pub context: Map<String, Value>,
}

#[derive(Deserialize, Clone)]
pub struct DimensionCondition {
    pub var: String,
}

#[derive(Serialize, Debug)]
pub struct PutResp {
    pub context_id: String,
    pub override_id: String,
    pub priority: i32,
}

#[derive(Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub size: Option<u32>,
}

#[derive(serde::Deserialize)]
pub enum ContextAction {
    PUT(PutReq),
    DELETE(String),
    MOVE((String, MoveReq)),
}

#[derive(serde::Serialize)]
pub enum ContextBulkResponse {
    PUT(PutResp),
    DELETE(String),
    MOVE(PutResp),
}

#[derive(Deserialize, Clone)]
pub struct FunctionsInfo {
    pub name: String,
    pub code: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct MoveQParams {
    pub update_type: Option<ConfigVersionType>,
}

#[derive(Deserialize, Clone)]
pub struct BulkOperationQParams {
    pub update_type: Option<ConfigVersionType>,
}

#[derive(Deserialize, Clone)]
pub struct PutQParams {
    pub update_type: Option<ConfigVersionType>,
}

#[derive(Deserialize, Clone)]
pub struct DeleteQParams {
    pub update_type: Option<ConfigVersionType>,
}
