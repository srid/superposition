use actix_web::{
    get,
    http::StatusCode,
    post,
    web::{self, Data, Json, Query},
    Scope,
};
use chrono::Utc;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};

use service_utils::service::types::{AppState, AuthenticationInfo, DbConnection};

use super::{
    helpers::{
        add_variant_dimension_to_ctx, check_variant_types,
        check_variants_override_coverage, validate_experiment,
    },
    types::{
        ContextAction, ContextPutReq, ContextPutResp, ExperimentCreateRequest,
        ExperimentCreateResponse,
    },
};
use crate::{
    api::{errors::AppError, experiments::types::ListFilters},
    db::models::{Experiment, ExperimentStatusType, Experiments},
};

pub fn endpoints() -> Scope {
    Scope::new("/experiments")
        .service(create)
        .service(list_experiments)
}

#[post("")]
async fn create(
    state: Data<AppState>,
    req: web::Json<ExperimentCreateRequest>,
    auth_info: AuthenticationInfo,
    db_conn: DbConnection,
) -> actix_web::Result<Json<ExperimentCreateResponse>> {
    use crate::db::schema::cac_v1::experiments::dsl::experiments;

    let DbConnection(mut conn) = db_conn;
    let override_keys = &req.override_keys;
    let mut variants = req.variants.to_vec();

    // Checking if experiment has exactly 1 control variant, and
    // atleast 1 experimental variant
    check_variant_types(&variants)
        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

    // Checking if all the variants are overriding the mentioned keys
    let are_valid_variants = check_variants_override_coverage(&variants, override_keys);
    if !are_valid_variants {
        return Err(actix_web::error::ErrorBadRequest(
            "all variants should contain the keys mentioned override_keys".to_string(),
        ));
    }

    // Checking if context is a key-value pair map
    if !req.context.is_object() {
        return Err(actix_web::error::ErrorBadRequest(
            "context should be map of key value pairs".to_string(),
        ));
    }

    //traffic_percentage should be max 100/length of variants
    // TODO: Add traffic_percentage validation

    // validating experiment against other active experiments based on permission flags
    let flags = &state.experimentation_flags;
    match validate_experiment(&req, &flags, &mut conn) {
        Ok(valid) => {
            if !valid {
                return Err(actix_web::error::ErrorBadRequest(
                    "invalid experiment config".to_string(),
                ));
            }
        }
        Err(_) => {
            return Err(actix_web::error::ErrorInternalServerError(""));
        }
    }

    // generating snowflake id for experiment
    let mut snowflake_generator = state.snowflake_generator.lock().unwrap();
    let experiment_id = snowflake_generator.real_time_generate();

    //create overrides in CAC, if successfull then create experiment in DB
    let mut cac_operations: Vec<ContextAction> = vec![];
    for mut variant in &mut variants {
        let variant_id = experiment_id.to_string() + "-" + &variant.id;

        // updating variant.id to => experiment_id + variant.id
        variant.id = variant_id.to_string();

        let updated_cacccontext =
            add_variant_dimension_to_ctx(&req.context, variant_id.to_string())
                .map_err(|_| actix_web::error::ErrorInternalServerError(""))?;

        let payload = ContextPutReq {
            context: updated_cacccontext
                .as_object()
                .ok_or(actix_web::error::ErrorInternalServerError(""))?
                .clone(),
            r#override: variant.overrides.clone(),
        };
        cac_operations.push(ContextAction::PUT(payload));
    }

    // creating variants' context in CAC
    let http_client = reqwest::Client::new();
    let url = state.cac_host.clone() + "/context/bulk-operations";

    let created_contexts: Vec<ContextPutResp> = http_client
        .put(&url)
        .bearer_auth(&state.admin_token)
        .json(&cac_operations)
        .send()
        .map_err(|e| {
            println!("failed to create contexts in cac: {e}");
            actix_web::error::ErrorInternalServerError("")
        })?
        .json::<Vec<ContextPutResp>>()
        .map_err(|e| {
            println!("failed to parse response: {e}");
            actix_web::error::ErrorInternalServerError("")
        })?;

    // updating variants with context and override ids
    for i in 0..created_contexts.len() {
        let created_context = &created_contexts[i];

        variants[i].context_id = Some(created_context.context_id.clone());
        variants[i].override_id = Some(created_context.override_id.clone());
    }

    // inserting experiment in db
    let AuthenticationInfo(email) = auth_info;
    let new_experiment = Experiment {
        id: experiment_id,
        created_by: email,
        created_at: Utc::now(),
        last_modified: Option::None,
        name: req.name.to_string(),
        override_keys: req.override_keys.to_vec(),
        traffic_percentage: req.traffic_percentage,
        status: ExperimentStatusType::CREATED,
        context: req.context.clone(),
        variants: serde_json::to_value(variants).unwrap(),
    };

    let insert = diesel::insert_into(experiments)
        .values(&new_experiment)
        .get_results(&mut conn);

    match insert {
        Ok(mut inserted_experiments) => {
            let inserted_experiment: Experiment = inserted_experiments.remove(0);
            let response = ExperimentCreateResponse {
                experiment_id: inserted_experiment.id,
            };

            return Ok(Json(response));
        }
        Err(e) => {
            println!("Experiment creation failed with error: {e}");
            return Err(actix_web::error::ErrorInternalServerError(
                "Failed to create experiment".to_string(),
            ));
        }
    }
}

#[get("")]
async fn list_experiments(
    state: Data<AppState>,
    filters: Query<ListFilters>,
) -> actix_web::Result<Json<Experiments>, AppError> {
    let mut conn = match state.db_pool.get() {
        Ok(conn) => conn,
        Err(e) => {
            println!("Unable to get db connection from pool, error: {e}");
            return Err(AppError {
                message: "Could not connect to the database".to_string(),
                possible_fix: "Try after sometime".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            });
            // return an error
        }
    };

    use crate::db::schema::cac_v1::experiments::dsl::*;
    let query = experiments
        .filter(status.eq_any(filters.status.clone()))
        .filter(created_at.ge(filters.from_date))
        .filter(created_at.le(filters.to_date))
        .limit(filters.count)
        .offset((filters.page - 1) * filters.count);

    // println!(
    //     "List filter query: {:?}",
    //     diesel::debug_query::<diesel::pg::Pg, _>(&query)
    // );
    let db_result = query.load::<Experiment>(&mut conn);

    match db_result {
        Ok(response) => return Ok(Json(response)),
        Err(e) => {
            return Err(match e {
                diesel::result::Error::NotFound => AppError {
                    message: String::from("No results found"),
                    possible_fix: String::from("Update your filter parameters"),
                    status_code: StatusCode::NOT_FOUND,
                },
                _ => AppError {
                    message: String::from("Something went wrong"),
                    possible_fix: String::from("Please try again later"),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                },
            })
        }
    };
}
