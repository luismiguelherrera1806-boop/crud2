mod db;
mod models;

use axum::{
    extract::{Extension, Form, Path},
    response::{Html, Redirect},
    routing::{get, post},
    Router,
};
use db::{create_item, delete_item, get_db_pool, get_item, list_items, update_item};
use dotenvy::dotenv;
use models::{CreateItem, UpdateItem};
use serde::Deserialize;
use std::{net::SocketAddr, sync::Arc};
use tera::{Context, Tera};
use tokio;
use anyhow::Result;
use crate::tokio::net::windows::named_pipe::PipeEnd::Server; 

#[derive(Clone)]
struct AppState {
    tera: Arc<Tera>,
    // PgPool se almacena por separado en Extension
}

#[tokio::main]
async fn main() -> Result<()> {
    // Cargar .env
    let _ = dotenv();

    // Crear pool de DB
    let pool = get_db_pool().await?;
    println!("Conectado a la DB ✅");

    // Cargar plantillas Tera
    let tera = Tera::new("templates/*")?;
    let state = AppState {
        tera: Arc::new(tera),
    };

    // Construir router
    let app = Router::new()
        .route("/", get(root_redirect))
        .route("/items", get(items_list))
        .route("/items/new", get(new_item_form))
        .route("/items", post(create_item_handler))
        .route("/items/:id/edit", get(edit_item_form))
        .route("/items/:id", post(update_item_handler))
        .route("/items/:id/delete", post(delete_item_handler))
        .layer(Extension(state))
        .layer(Extension(pool));

    // Arrancar servidor
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;

    println!("Servidor en http://{}", addr);
    Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

// Redirigir "/" -> "/items"
async fn root_redirect() -> Redirect {
    Redirect::to("/items")
}

/// Listar items y renderizar plantilla
async fn items_list(
    Extension(state): Extension<AppState>,
    Extension(pool): Extension<sqlx::PgPool>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let items = list_items(&pool).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error DB: {}", e),
        )
    })?;

    let mut ctx = Context::new();
    ctx.insert("items", &items);

    let s = state
        .tera
        .render("list.html", &ctx)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Html(s))
}

/// Form data struct
#[derive(Debug, Deserialize)]
struct ItemForm {
    name: String,
    description: Option<String>,
    quantity: Option<i32>,
    price: Option<f64>,
}

/// GET /items/new -> formulario vacío
async fn new_item_form(Extension(state): Extension<AppState>) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let mut ctx = Context::new();
    ctx.insert("action", "/items");
    ctx.insert("method", "POST");
    ctx.insert("title", "Crear item");
    ctx.insert("item", &serde_json::json!({}));
    let s = state
        .tera
        .render("form.html", &ctx)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Html(s))
}

/// POST /items -> crear item
async fn create_item_handler(
    Extension(pool): Extension<sqlx::PgPool>,
    Form(form): Form<ItemForm>,
) -> Result<Redirect, (axum::http::StatusCode, String)> {
    let create = CreateItem {
        name: form.name,
        description: form.description,
        quantity: form.quantity,
        price: form.price,
    };

    create_item(&pool, create)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)))?;

    Ok(Redirect::to("/items"))
}

/// GET /items/:id/edit -> formulario con datos actuales
async fn edit_item_form(
    Path(id): Path<i32>,
    Extension(state): Extension<AppState>,
    Extension(pool): Extension<sqlx::PgPool>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let item = get_item(&pool, id)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)))?;

    if let Some(it) = item {
        let mut ctx = Context::new();
        ctx.insert("action", &format!("/items/{}", id));
        ctx.insert("method", "POST");
        ctx.insert("title", &format!("Editar item #{}", id));
        ctx.insert("item", &it);

        let s = state
            .tera
            .render("form.html", &ctx)
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        Ok(Html(s))
    } else {
        Err((axum::http::StatusCode::NOT_FOUND, "Item no encontrado".into()))
    }
}

/// POST /items/:id -> actualizar
async fn update_item_handler(
    Path(id): Path<i32>,
    Extension(pool): Extension<sqlx::PgPool>,
    Form(form): Form<ItemForm>,
) -> Result<Redirect, (axum::http::StatusCode, String)> {
    let update = UpdateItem {
        name: Some(form.name),
        description: form.description,
        quantity: form.quantity,
        price: form.price,
    };

    update_item(&pool, id, update)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)))?;

    Ok(Redirect::to("/items"))
}

/// POST /items/:id/delete -> eliminar
async fn delete_item_handler(
    Path(id): Path<i32>,
    Extension(pool): Extension<sqlx::PgPool>,
) -> Result<Redirect, (axum::http::StatusCode, String)> {
    delete_item(&pool, id)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)))?;
    Ok(Redirect::to("/items"))
}