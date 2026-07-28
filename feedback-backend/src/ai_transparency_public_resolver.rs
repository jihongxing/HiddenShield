use axum::{
    extract::{Path, State},
    http::{header, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::{PgPool, Row};
use tower_http::cors::{Any, CorsLayer};

pub const PUBLIC_RESOLVER_SCHEMA_VERSION: &str = "hs-ai-public-resolver-v1";

#[derive(Clone)]
pub struct AiTransparencyPublicResolverState {
    pub pool: PgPool,
}

pub fn build_ai_transparency_public_resolver_router(
    state: AiTransparencyPublicResolverState,
) -> Router {
    Router::new()
        .route(
            "/v1/ai-transparency/public/resolve/watermarks/:watermark_uid",
            get(resolve_by_watermark_uid),
        )
        .route(
            "/v1/ai-transparency/public/resolve/manifests/:manifest_id",
            get(resolve_by_manifest_id),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET]),
        )
        .with_state(state)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicResolverResponse {
    schema_version: &'static str,
    resolution_status: &'static str,
    manifest_id: String,
    watermark_uid: String,
    manifest_status: String,
    claim_type: String,
    marker_status: &'static str,
    metadata_signature_status: &'static str,
    watermark_detection_status: &'static str,
    issuer_trust_status: &'static str,
    evidence_level: String,
    evidence_verification_status: String,
    generated_at: DateTime<Utc>,
    profiles: Vec<PublicProfileStatus>,
    markers: Vec<PublicMarkerStatus>,
    legal_conclusion: bool,
    warnings: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicProfileStatus {
    profile_id: String,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicMarkerStatus {
    marker_type: String,
    profile_id: String,
    version: String,
    verification_status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicResolverNotFound {
    schema_version: &'static str,
    resolution_status: &'static str,
    legal_conclusion: bool,
    warnings: Vec<&'static str>,
}

enum PublicResolverError {
    NotFound,
    Unavailable,
}

impl IntoResponse for PublicResolverError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound => public_json_response(
                StatusCode::NOT_FOUND,
                Json(PublicResolverNotFound {
                    schema_version: PUBLIC_RESOLVER_SCHEMA_VERSION,
                    resolution_status: "not_found",
                    legal_conclusion: false,
                    warnings: vec![
                        "No confirmed HiddenShield AI transparency record was found.",
                        "Not found does not prove that the media was created by a human.",
                    ],
                }),
                "public, max-age=30",
            ),
            Self::Unavailable => public_json_response(
                StatusCode::SERVICE_UNAVAILABLE,
                Json(PublicResolverNotFound {
                    schema_version: PUBLIC_RESOLVER_SCHEMA_VERSION,
                    resolution_status: "unavailable",
                    legal_conclusion: false,
                    warnings: vec![
                        "The public resolver is temporarily unavailable.",
                        "No legal or authorship conclusion can be made from this result.",
                    ],
                }),
                "no-store",
            ),
        }
    }
}

async fn resolve_by_watermark_uid(
    State(state): State<AiTransparencyPublicResolverState>,
    Path(watermark_uid): Path<String>,
) -> Result<Response, PublicResolverError> {
    if !valid_watermark_uid(&watermark_uid) {
        return Err(PublicResolverError::NotFound);
    }
    resolve(&state.pool, "watermark_uid", &watermark_uid).await
}

async fn resolve_by_manifest_id(
    State(state): State<AiTransparencyPublicResolverState>,
    Path(manifest_id): Path<String>,
) -> Result<Response, PublicResolverError> {
    if manifest_id.is_empty() || manifest_id.len() > 160 {
        return Err(PublicResolverError::NotFound);
    }
    resolve(&state.pool, "transparency_manifest_id", &manifest_id).await
}

async fn resolve(
    pool: &PgPool,
    lookup_column: &'static str,
    lookup_value: &str,
) -> Result<Response, PublicResolverError> {
    let query = match lookup_column {
        "watermark_uid" => {
            "SELECT transparency_manifest_id, watermark_uid, manifest_status, claim_type,
                    generated_at, profile_status_json
             FROM ai_public_confirmed_manifests
             WHERE watermark_uid = $1
             ORDER BY CASE WHEN manifest_status = 'active' THEN 0 ELSE 1 END,
                      manifest_version DESC
             LIMIT 1"
        }
        _ => {
            "SELECT transparency_manifest_id, watermark_uid, manifest_status, claim_type,
                    generated_at, profile_status_json
             FROM ai_public_confirmed_manifests
             WHERE transparency_manifest_id = $1
             LIMIT 1"
        }
    };
    let manifest = sqlx::query(query)
        .bind(lookup_value)
        .fetch_optional(pool)
        .await
        .map_err(|_| PublicResolverError::Unavailable)?
        .ok_or(PublicResolverError::NotFound)?;
    let manifest_id: String = manifest.get("transparency_manifest_id");
    let marker_rows = sqlx::query(
        "SELECT marker_type, marker_profile_id, marker_version, verify_status
         FROM ai_public_confirmed_markers
         WHERE transparency_manifest_id = $1
         ORDER BY marker_type, marker_profile_id",
    )
    .bind(&manifest_id)
    .fetch_all(pool)
    .await
    .map_err(|_| PublicResolverError::Unavailable)?;
    let evidence = sqlx::query(
        "SELECT evidence_level, verification_status
         FROM ai_public_confirmed_evidence_summary
         WHERE transparency_manifest_id = $1
         ORDER BY evidence_level, verification_status
         LIMIT 1",
    )
    .bind(&manifest_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| PublicResolverError::Unavailable)?;
    let markers = marker_rows
        .iter()
        .map(|row| PublicMarkerStatus {
            marker_type: row.get("marker_type"),
            profile_id: row.get("marker_profile_id"),
            version: row.get("marker_version"),
            verification_status: row.get("verify_status"),
        })
        .collect::<Vec<_>>();
    let watermark_detection_status = if markers.iter().any(|marker| {
        marker.marker_type == "blind_watermark" && marker.verification_status == "verified"
    }) {
        "verified"
    } else {
        "not_present"
    };
    let metadata_signature_status = if markers
        .iter()
        .any(|marker| marker.marker_type == "c2pa" && marker.verification_status == "verified")
    {
        "verified"
    } else {
        "not_present"
    };
    let marker_status = if markers
        .iter()
        .any(|marker| marker.verification_status == "verified")
    {
        "present_verified"
    } else if markers.is_empty() {
        "not_present"
    } else {
        "present_unverified"
    };
    let (evidence_level, evidence_verification_status) = evidence
        .map(|row| {
            (
                row.get::<String, _>("evidence_level"),
                row.get::<String, _>("verification_status"),
            )
        })
        .unwrap_or_else(|| ("unknown".to_string(), "not_present".to_string()));
    let profiles = public_profiles(manifest.get("profile_status_json"));
    let response = PublicResolverResponse {
        schema_version: PUBLIC_RESOLVER_SCHEMA_VERSION,
        resolution_status: "confirmed",
        manifest_id,
        watermark_uid: manifest.get("watermark_uid"),
        manifest_status: manifest.get("manifest_status"),
        claim_type: manifest.get("claim_type"),
        marker_status,
        metadata_signature_status,
        watermark_detection_status,
        issuer_trust_status: "not_evaluated",
        evidence_level,
        evidence_verification_status,
        generated_at: manifest.get("generated_at"),
        profiles,
        markers,
        legal_conclusion: false,
        warnings: vec![
            "This result reports a confirmed transparency record, not a legal conclusion.",
            "Absence or failure of a marker does not prove human authorship.",
        ],
    };
    Ok(public_json_response(
        StatusCode::OK,
        Json(response),
        "public, max-age=60",
    ))
}

fn public_profiles(value: Value) -> Vec<PublicProfileStatus> {
    let mut profiles = value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let profile_id = item.get("profileId")?.as_str()?.to_string();
            let status = match item.get("status")?.as_str()? {
                "applied" | "applied_internal_only" => "applied",
                "partially_applied" => "partially_applied",
                "not_applicable" => "not_applicable",
                "configuration_required" => "configuration_required",
                _ => "failed",
            }
            .to_string();
            Some(PublicProfileStatus { profile_id, status })
        })
        .collect::<Vec<_>>();
    profiles.sort_by(|left, right| left.profile_id.cmp(&right.profile_id));
    profiles
}

fn public_json_response<T: Serialize>(
    status: StatusCode,
    body: Json<T>,
    cache_control: &'static str,
) -> Response {
    let mut response = (status, body).into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control),
    );
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    response.headers_mut().insert(
        header::HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    response
}

fn valid_watermark_uid(value: &str) -> bool {
    let parts = value.split('-').collect::<Vec<_>>();
    parts.len() == 5
        && parts[0] == "HS"
        && parts[1..]
            .iter()
            .all(|part| part.len() == 8 && part.bytes().all(|byte| byte.is_ascii_hexdigit()))
}
