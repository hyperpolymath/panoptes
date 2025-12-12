// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

//! Web UI for Panoptes dashboard

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::db::{Database, FileRecord, Tag};
use crate::config::AppConfig;

/// Shared application state
pub struct AppState {
    pub db: Database,
    pub config: AppConfig,
}

/// Create the web application router
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Pages
        .route("/", get(index_page))
        .route("/files", get(files_page))
        .route("/tags", get(tags_page))
        .route("/settings", get(settings_page))
        // API endpoints
        .route("/api/files", get(api_get_files))
        .route("/api/files/search", get(api_search_files))
        .route("/api/tags", get(api_get_tags))
        .route("/api/stats", get(api_get_stats))
        .route("/api/categories", get(api_get_categories))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

// === Page Handlers ===

async fn index_page(State(state): State<Arc<AppState>>) -> Html<String> {
    let recent_files = state.db.get_recent_files(10).unwrap_or_default();
    let stats = state.db.get_category_stats().unwrap_or_default();
    let file_count = state.db.get_file_count().unwrap_or(0);

    Html(render_index(&recent_files, &stats, file_count))
}

async fn files_page(State(state): State<Arc<AppState>>) -> Html<String> {
    let files = state.db.get_recent_files(100).unwrap_or_default();
    Html(render_files_page(&files))
}

async fn tags_page(State(state): State<Arc<AppState>>) -> Html<String> {
    let tags = state.db.get_all_tags().unwrap_or_default();
    Html(render_tags_page(&tags))
}

async fn settings_page(State(state): State<Arc<AppState>>) -> Html<String> {
    Html(render_settings_page(&state.config))
}

// === API Handlers ===

#[derive(Deserialize)]
struct FilesQuery {
    limit: Option<usize>,
    category: Option<String>,
}

async fn api_get_files(
    State(state): State<Arc<AppState>>,
    Query(query): Query<FilesQuery>,
) -> Json<Vec<FileRecord>> {
    let limit = query.limit.unwrap_or(50);
    let files = if let Some(category) = query.category {
        state.db.get_files_by_category(&category, limit).unwrap_or_default()
    } else {
        state.db.get_recent_files(limit).unwrap_or_default()
    };
    Json(files)
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    limit: Option<usize>,
}

async fn api_search_files(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> Json<Vec<FileRecord>> {
    let limit = query.limit.unwrap_or(50);
    let files = state.db.search_files(&query.q, limit).unwrap_or_default();
    Json(files)
}

async fn api_get_tags(State(state): State<Arc<AppState>>) -> Json<Vec<Tag>> {
    let tags = state.db.get_all_tags().unwrap_or_default();
    Json(tags)
}

#[derive(Serialize)]
struct StatsResponse {
    total_files: i64,
    categories: Vec<(String, i64)>,
}

async fn api_get_stats(State(state): State<Arc<AppState>>) -> Json<StatsResponse> {
    let total_files = state.db.get_file_count().unwrap_or(0);
    let categories = state.db.get_category_stats().unwrap_or_default();
    Json(StatsResponse { total_files, categories })
}

async fn api_get_categories(State(state): State<Arc<AppState>>) -> Json<Vec<(String, i64)>> {
    let stats = state.db.get_category_stats().unwrap_or_default();
    Json(stats)
}

// === Template Rendering ===

fn base_template(title: &str, content: &str) -> String {
    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - Panoptes</title>
    <style>
        :root {{
            --bg-primary: #1a1a2e;
            --bg-secondary: #16213e;
            --bg-card: #0f3460;
            --text-primary: #e8e8e8;
            --text-secondary: #a0a0a0;
            --accent: #e94560;
            --accent-hover: #ff6b6b;
            --success: #00d9a5;
            --border: #2a2a4a;
        }}
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: var(--bg-primary);
            color: var(--text-primary);
            line-height: 1.6;
        }}
        .container {{ max-width: 1400px; margin: 0 auto; padding: 20px; }}
        nav {{
            background: var(--bg-secondary);
            padding: 15px 20px;
            display: flex;
            align-items: center;
            gap: 30px;
            border-bottom: 1px solid var(--border);
        }}
        nav .logo {{
            font-size: 1.5em;
            font-weight: bold;
            color: var(--accent);
            text-decoration: none;
        }}
        nav a {{
            color: var(--text-secondary);
            text-decoration: none;
            transition: color 0.2s;
        }}
        nav a:hover {{ color: var(--text-primary); }}
        .card {{
            background: var(--bg-card);
            border-radius: 12px;
            padding: 20px;
            margin-bottom: 20px;
        }}
        .card h2 {{
            margin-bottom: 15px;
            color: var(--accent);
        }}
        .stats-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }}
        .stat-card {{
            background: var(--bg-card);
            border-radius: 12px;
            padding: 20px;
            text-align: center;
        }}
        .stat-card .number {{
            font-size: 2.5em;
            font-weight: bold;
            color: var(--accent);
        }}
        .stat-card .label {{
            color: var(--text-secondary);
            font-size: 0.9em;
        }}
        table {{
            width: 100%;
            border-collapse: collapse;
        }}
        th, td {{
            padding: 12px;
            text-align: left;
            border-bottom: 1px solid var(--border);
        }}
        th {{ color: var(--text-secondary); font-weight: 500; }}
        tr:hover {{ background: rgba(255,255,255,0.05); }}
        .tag {{
            display: inline-block;
            background: var(--accent);
            color: white;
            padding: 2px 8px;
            border-radius: 12px;
            font-size: 0.8em;
            margin: 2px;
        }}
        .category-badge {{
            display: inline-block;
            background: var(--bg-secondary);
            border: 1px solid var(--border);
            padding: 4px 10px;
            border-radius: 6px;
            font-size: 0.85em;
        }}
        .confidence {{
            display: inline-block;
            width: 60px;
            height: 8px;
            background: var(--bg-secondary);
            border-radius: 4px;
            overflow: hidden;
        }}
        .confidence-fill {{
            height: 100%;
            background: var(--success);
            border-radius: 4px;
        }}
    </style>
</head>
<body>
    <nav>
        <a href="/" class="logo">Panoptes</a>
        <a href="/">Dashboard</a>
        <a href="/files">Files</a>
        <a href="/tags">Tags</a>
        <a href="/settings">Settings</a>
    </nav>
    <main class="container">
        {}
    </main>
</body>
</html>"#, title, content)
}

fn render_index(files: &[FileRecord], stats: &[(String, i64)], file_count: i64) -> String {
    let category_count = stats.len();

    let stats_html = format!(r#"
        <div class="stats-grid">
            <div class="stat-card">
                <div class="number">{}</div>
                <div class="label">Total Files</div>
            </div>
            <div class="stat-card">
                <div class="number">{}</div>
                <div class="label">Categories</div>
            </div>
        </div>
    "#, file_count, category_count);

    let files_html = render_files_table(files);

    let categories_html: String = stats.iter()
        .map(|(cat, count)| format!(r#"<tr><td>{}</td><td>{}</td></tr>"#, cat, count))
        .collect();

    let content = format!(r#"
        <h1>Dashboard</h1>
        {}
        <div style="display: grid; grid-template-columns: 2fr 1fr; gap: 20px;">
            <div class="card">
                <h2>Recent Files</h2>
                {}
            </div>
            <div class="card">
                <h2>Categories</h2>
                <table>
                    <tr><th>Category</th><th>Count</th></tr>
                    {}
                </table>
            </div>
        </div>
    "#, stats_html, files_html, categories_html);

    base_template("Dashboard", &content)
}

fn render_files_table(files: &[FileRecord]) -> String {
    let rows: String = files.iter()
        .map(|f| {
            let confidence_pct = (f.confidence * 100.0) as u32;
            format!(r#"
                <tr>
                    <td>{}</td>
                    <td><span class="category-badge">{}</span></td>
                    <td>
                        <div class="confidence">
                            <div class="confidence-fill" style="width: {}%"></div>
                        </div>
                    </td>
                    <td>{}</td>
                </tr>
            "#,
            f.suggested_name,
            f.category.as_deref().unwrap_or("Uncategorized"),
            confidence_pct,
            f.created_at.format("%Y-%m-%d %H:%M")
            )
        })
        .collect();

    format!(r#"
        <table>
            <tr>
                <th>Name</th>
                <th>Category</th>
                <th>Confidence</th>
                <th>Date</th>
            </tr>
            {}
        </table>
    "#, rows)
}

fn render_files_page(files: &[FileRecord]) -> String {
    let content = format!(r#"
        <h1>Files</h1>
        <div class="card">
            {}
        </div>
    "#, render_files_table(files));

    base_template("Files", &content)
}

fn render_tags_page(tags: &[Tag]) -> String {
    let tags_html: String = tags.iter()
        .map(|t| format!(r#"<span class="tag">{}</span>"#, t.name))
        .collect();

    let content = format!(r#"
        <h1>Tags</h1>
        <div class="card">
            <p>All tags in the database:</p>
            <div style="margin-top: 20px;">
                {}
            </div>
        </div>
    "#, if tags_html.is_empty() { "No tags yet".to_string() } else { tags_html });

    base_template("Tags", &content)
}

fn render_settings_page(config: &AppConfig) -> String {
    let watch_paths: String = config.watch_paths.iter()
        .map(|p| format!("<li>{}</li>", p))
        .collect();

    let content = format!(r#"
        <h1>Settings</h1>
        <div class="card">
            <h2>Watch Directories</h2>
            <ul>{}</ul>
        </div>
        <div class="card">
            <h2>AI Configuration</h2>
            <table>
                <tr><td>Vision Model</td><td>{}</td></tr>
                <tr><td>Text Model</td><td>{}</td></tr>
                <tr><td>Code Model</td><td>{}</td></tr>
                <tr><td>API URL</td><td>{}</td></tr>
            </table>
        </div>
        <div class="card">
            <h2>Rules</h2>
            <table>
                <tr><td>Date Prefix</td><td>{}</td></tr>
                <tr><td>Max Length</td><td>{}</td></tr>
                <tr><td>Auto Categorize</td><td>{}</td></tr>
            </table>
        </div>
    "#,
        watch_paths,
        config.ai_engine.models.vision,
        config.ai_engine.models.text,
        config.ai_engine.models.code,
        config.ai_engine.url,
        config.rules.date_prefix,
        config.rules.max_length,
        config.rules.auto_categorize,
    );

    base_template("Settings", &content)
}

/// Start the web server with config and database
pub async fn start_server(config: AppConfig, db: Database) -> crate::Result<()> {
    let state = Arc::new(AppState {
        db,
        config: config.clone(),
    });

    let addr = format!("{}:{}", config.web.host, config.web.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("Web UI available at https://{}", addr);

    let router = create_router(state);
    axum::serve(listener, router).await
        .map_err(|e| crate::PanoptesError::Config(format!("Server error: {}", e)))?;

    Ok(())
}
