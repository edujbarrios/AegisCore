mod state;

use crate::config::AegisConfig;
use crate::modules::core::CoreModule;
use crate::modules::AegisModule;
use crate::registry::{SkillRegistry, SkillRegistryEntry, ToolContext, ToolRegistry};
use crate::runtime::AgentRuntime;
use crate::skills::{SkillFormat, SkillSpec, SkillStore};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::middleware::{from_fn_with_state, Next};
use axum::routing::{get, post};
use axum::{http::Request, response::Html, response::Response, Json, Router};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::sync::Semaphore;
use tower_http::services::ServeDir;

use state::AppState;
use state::RateLimiter;

pub async fn serve(cfg: AegisConfig) -> anyhow::Result<()> {
    let tool_ctx = {
        let mut ctx = ToolContext::default_for_repo()?;
        ctx.fs_root =
            std::fs::canonicalize(&cfg.runtime.fs_root).unwrap_or(cfg.runtime.fs_root.clone());
        ctx.allow_dangerous_tools = cfg.runtime.allow_dangerous_tools;
        ctx.max_read_bytes = cfg.runtime.max_read_bytes;
        ctx.max_write_bytes = cfg.runtime.max_write_bytes;
        ctx.http_max_bytes = cfg.runtime.http_max_bytes;
        ctx.http_timeout_ms = cfg.runtime.http_timeout_ms;
        ctx.shell_timeout_ms = cfg.runtime.shell_timeout_ms;
        ctx.tool_concurrency = Arc::new(Semaphore::new(8));
        ctx
    };

    let mut tools = ToolRegistry::default();
    CoreModule.register_tools(&mut tools)?;
    let tools = Arc::new(tools);

    let store = SkillStore::new("skills");
    let skills_registry = Arc::new(RwLock::new(load_skills_registry(&store)?));

    let state = AppState {
        cfg: cfg.clone(),
        tools,
        skills: skills_registry,
        tool_ctx,
        store,
        rate_limiter: Arc::new(tokio::sync::Mutex::new(RateLimiter::new(
            Duration::from_secs(1),
            50,
        ))),
    };

    let app = Router::new()
        .layer(from_fn_with_state(state.clone(), rate_limit))
        .route("/", get(frontend_index))
        .nest_service("/assets", ServeDir::new("frontend/assets"))
        .route("/health", get(health))
        .route("/skills", get(list_skills))
        .route("/skills/{name}", get(get_skill).delete(delete_skill))
        .route("/skills/create", post(create_skill))
        .route("/skills/{name}/run", post(run_skill))
        .route("/tools", get(list_tools))
        .route("/modules", get(list_modules))
        .with_state(state);

    let addr = format!("{}:{}", cfg.server.host, cfg.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(addr = %addr, "server listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn rate_limit(
    State(state): State<AppState>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let mut limiter = state.rate_limiter.lock().await;
    if !limiter.allow() {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    Ok(next.run(req).await)
}

async fn health() -> Json<Value> {
    Json(serde_json::json!({"status":"ok"}))
}

async fn frontend_index() -> Html<&'static str> {
    Html(include_str!("../../frontend/index.html"))
}

async fn list_modules() -> Json<Value> {
    Json(serde_json::json!({
        "modules": [{
            "name": CoreModule.name(),
            "version": CoreModule.version(),
            "description": CoreModule.description()
        }]
    }))
}

async fn list_tools(State(state): State<AppState>) -> Json<Value> {
    let tools = state
        .tools
        .list()
        .into_iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.description,
                "parameters": t.parameters
            })
        })
        .collect::<Vec<_>>();
    Json(serde_json::json!({ "tools": tools }))
}

async fn list_skills(State(state): State<AppState>) -> Json<Value> {
    let skills = state.skills.read().await;
    let list = skills
        .list()
        .into_iter()
        .map(|e| serde_json::to_value(&e.spec).unwrap_or(Value::Null))
        .collect::<Vec<_>>();
    Json(serde_json::json!({ "skills": list }))
}

async fn get_skill(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let skills = state.skills.read().await;
    let entry = skills.get(&name).ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(
        serde_json::to_value(&entry.spec).unwrap_or(Value::Null),
    ))
}

async fn create_skill(
    State(state): State<AppState>,
    Json(body): Json<SkillSpec>,
) -> Result<Json<Value>, StatusCode> {
    let path = state
        .store
        .save(&body, SkillFormat::Toml)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let updated =
        load_skills_registry(&state.store).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    *state.skills.write().await = updated;
    Ok(Json(
        serde_json::json!({ "saved": path.display().to_string() }),
    ))
}

async fn delete_skill(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let deleted = state
        .store
        .delete(&name)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !deleted {
        return Err(StatusCode::NOT_FOUND);
    }
    let updated =
        load_skills_registry(&state.store).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    *state.skills.write().await = updated;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn run_skill(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(input): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let skills_snapshot = state.skills.read().await.clone();
    let runtime = AgentRuntime::new(
        state.cfg.clone(),
        state.tools.clone(),
        Arc::new(skills_snapshot),
        state.tool_ctx.clone(),
    );
    let out = runtime
        .run_skill(&name, input)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(out))
}

fn load_skills_registry(store: &SkillStore) -> anyhow::Result<SkillRegistry> {
    let mut skills = SkillRegistry::default();
    for item in store.load_all()? {
        skills.register(SkillRegistryEntry {
            spec: item.spec,
            source_path: Some(item.path),
        })?;
    }
    Ok(skills)
}
