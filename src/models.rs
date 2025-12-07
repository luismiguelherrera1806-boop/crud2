use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Item {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub quantity: i32,
    pub price: f64, // leeremos NUMERIC(10,2) como f64
    pub created_at: DateTime<Utc>,
}

/// Estructura para recibir datos de creación desde formularios/API
#[derive(Debug, Deserialize)]
pub struct CreateItem {
    pub name: String,
    pub description: Option<String>,
    pub quantity: Option<i32>,
    pub price: Option<f64>,
}

/// Estructura para recibir datos de actualización
#[derive(Debug, Deserialize)]
pub struct UpdateItem {
    pub name: Option<String>,
    pub description: Option<String>,
    pub quantity: Option<i32>,
    pub price: Option<f64>,
}