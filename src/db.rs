use crate::models::{CreateItem, Item, UpdateItem};
use dotenvy::dotenv;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("ENV var error: {0}")]
    EnvVar(#[from] std::env::VarError),
}

/// Crea y devuelve un PgPool leyendo DATABASE_URL desde .env
/// Versión simple y compatible: no usamos connect_timeout ni connect_with.
pub async fn get_db_pool() -> Result<PgPool, DbError> {
    let _ = dotenv(); // carga .env si existe
    let database_url = env::var("DATABASE_URL")?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url) // conexión directa usando URL
        .await?;

    Ok(pool)
}

// --- CRUD ---
pub async fn list_items(pool: &PgPool) -> Result<Vec<Item>, DbError> {
    let items = sqlx::query_as::<_, Item>(
        r#"
        SELECT id, name, description, quantity, price::double precision as price, created_at
        FROM items
        ORDER BY id
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(items)
}

pub async fn get_item(pool: &PgPool, id: i32) -> Result<Option<Item>, DbError> {
    let item = sqlx::query_as::<_, Item>(
        r#"
        SELECT id, name, description, quantity, price::double precision as price, created_at
        FROM items
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(item)
}

pub async fn create_item(pool: &PgPool, input: CreateItem) -> Result<Item, DbError> {
    let quantity = input.quantity.unwrap_or(0);
    let price = input.price.unwrap_or(0.0);

    let rec = sqlx::query_as::<_, Item>(
        r#"
        INSERT INTO items (name, description, quantity, price)
        VALUES ($1, $2, $3, $4)
        RETURNING id, name, description, quantity, price::double precision as price, created_at
        "#,
    )
    .bind(input.name)
    .bind(input.description)
    .bind(quantity)
    .bind(price)
    .fetch_one(pool)
    .await?;

    Ok(rec)
}

pub async fn update_item(pool: &PgPool, id: i32, input: UpdateItem) -> Result<Option<Item>, DbError> {
    if let Some(existing) = get_item(pool, id).await? {
        let new_name = input.name.unwrap_or(existing.name);
        let new_description = input.description.or(existing.description);
        let new_quantity = input.quantity.unwrap_or(existing.quantity);
        let new_price = input.price.unwrap_or(existing.price);

        let rec = sqlx::query_as::<_, Item>(
            r#"
            UPDATE items
            SET name = $1, description = $2, quantity = $3, price = $4
            WHERE id = $5
            RETURNING id, name, description, quantity, price::double precision as price, created_at
            "#,
        )
        .bind(new_name)
        .bind(new_description)
        .bind(new_quantity)
        .bind(new_price)
        .bind(id)
        .fetch_one(pool)
        .await?;

        Ok(Some(rec))
    } else {
        Ok(None)
    }
}

pub async fn delete_item(pool: &PgPool, id: i32) -> Result<bool, DbError> {
    let res = sqlx::query("DELETE FROM items WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}