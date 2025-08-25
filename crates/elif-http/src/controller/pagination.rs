//! Pagination utilities for controllers

use serde::{Deserialize, Serialize};

/// Query parameters for pagination and filtering
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub search: Option<String>,
    pub filter: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

impl Default for QueryParams {
    fn default() -> Self {
        Self {
            page: Some(1),
            per_page: Some(10),
            search: None,
            filter: None,
            sort: None,
            order: None,
        }
    }
}

/// Pagination metadata for responses
#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    pub current_page: u32,
    pub per_page: u32,
    pub total_items: u64,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_prev: bool,
}

impl PaginationMeta {
    pub fn new(page: u32, per_page: u32, total: u64) -> Self {
        let total_pages = if per_page > 0 {
            (total as f64 / per_page as f64).ceil() as u32
        } else {
            0
        };

        Self {
            current_page: page,
            per_page,
            total_items: total,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        }
    }
}
