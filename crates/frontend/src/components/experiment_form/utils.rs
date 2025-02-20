use super::types::{
    ExperimentCreateRequest, ExperimentUpdateRequest, VariantUpdateRequest,
};
use crate::components::context_form::utils::construct_context;
use crate::types::{Dimension, Variant};
use crate::utils::get_host;
use reqwest::StatusCode;
use serde_json::json;

pub fn validate_experiment(experiment: &ExperimentCreateRequest) -> Result<bool, String> {
    if experiment.name.is_empty() {
        return Err(String::from("experiment name should not be empty"));
    }
    Ok(true)
}

pub async fn create_experiment(
    conditions: Vec<(String, String, String)>,
    variants: Vec<Variant>,
    name: String,
    tenant: String,
    dimensions: Vec<Dimension>,
) -> Result<String, String> {
    let payload = ExperimentCreateRequest {
        name,
        variants,
        context: construct_context(conditions, dimensions),
    };

    let _ = validate_experiment(&payload)?;

    let client = reqwest::Client::new();
    let host = get_host();
    let url = format!("{host}/experiments");
    let request_payload = json!(payload);
    let response = client
        .post(url)
        .header("x-tenant", tenant)
        .json(&request_payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    match response.status() {
        StatusCode::OK => response.text().await.map_err(|e| e.to_string()),
        StatusCode::BAD_REQUEST => Err("epxeriment data corrupt".to_string()),
        _ => Err("Internal Server Error".to_string()),
    }
}

pub async fn update_experiment(
    experiment_id: String,
    variants: Vec<Variant>,
    tenant: String,
) -> Result<String, String> {
    let payload = ExperimentUpdateRequest {
        variants: variants
            .into_iter()
            .map(|variant| VariantUpdateRequest {
                id: variant.id,
                overrides: variant.overrides,
            })
            .collect::<Vec<VariantUpdateRequest>>(),
    };

    let client = reqwest::Client::new();
    let host = get_host();
    let url = format!("{}/experiments/{}/overrides", host, experiment_id);
    let request_payload = json!(payload);
    let response = client
        .put(url)
        .header("x-tenant", tenant)
        .header("Authorization", "Bearer 12345678")
        .json(&request_payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    match response.status() {
        StatusCode::OK => response.text().await.map_err(|e| e.to_string()),
        StatusCode::BAD_REQUEST => Err("epxeriment data corrupt".to_string()),
        _ => Err("Internal Server Error".to_string()),
    }
}
