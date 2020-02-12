#![allow(dead_code)]

mod bulk;
mod create_index;
mod delete_index;

use crate::access_method::options::ZDBIndexOptions;
use crate::elasticsearch::delete_index::ElasticsearchDeleteIndexRequest;
pub use bulk::*;
pub use create_index::*;
use pgx::{pg_sys, PgBox};
use reqwest::RequestBuilder;
use serde_json::Value;
use std::io::Read;

#[derive(Clone)]
struct InternalOptions {
    url: String,
    type_name: String,
    refresh_interval: String,
    alias: String,
    uuid: String,
    index_name: String,

    optimize_after: i32,
    compression_level: i32,
    shards: i32,
    replicas: i32,
    bulk_concurrency: i32,
    batch_size: i32,
    llapi: bool,
}

#[derive(Clone)]
pub struct Elasticsearch {
    options: InternalOptions,
}

#[derive(Debug)]
pub struct ElasticsearchError(Option<reqwest::StatusCode>, String);

impl ElasticsearchError {
    pub fn status(&self) -> Option<reqwest::StatusCode> {
        self.0
    }

    pub fn message(&self) -> &str {
        &self.1
    }
}

impl Elasticsearch {
    pub fn new(
        heaprel: &PgBox<pg_sys::RelationData>,
        indexrel: &PgBox<pg_sys::RelationData>,
    ) -> Self {
        let zdboptions = ZDBIndexOptions::from(indexrel);
        Elasticsearch {
            options: InternalOptions {
                url: zdboptions.url(),
                type_name: zdboptions.type_name(),
                refresh_interval: zdboptions.refresh_interval(),
                alias: zdboptions.alias(heaprel, indexrel),
                uuid: zdboptions.uuid(heaprel, indexrel),
                index_name: zdboptions.index_name(heaprel, indexrel),

                optimize_after: zdboptions.optimize_after(),
                compression_level: zdboptions.compression_level(),
                shards: zdboptions.shards(),
                replicas: zdboptions.replicas(),
                bulk_concurrency: zdboptions.bulk_concurrency(),
                batch_size: zdboptions.batch_size(),
                llapi: zdboptions.llapi(),
            },
        }
    }

    pub fn create_index(&self, mapping: Value) -> ElasticsearchCreateIndexRequest {
        ElasticsearchCreateIndexRequest::new(self, mapping)
    }

    pub fn delete_index(&self) -> ElasticsearchDeleteIndexRequest {
        ElasticsearchDeleteIndexRequest::new(self)
    }

    pub fn start_bulk(&self) -> ElasticsearchBulkRequest {
        let concurrency = num_cpus::get().min(self.options.bulk_concurrency as usize);
        ElasticsearchBulkRequest::new(self, 10_000, concurrency)
    }

    pub fn base_url(&self) -> String {
        format!("{}{}", self.options.url, self.options.index_name)
    }

    pub fn execute_request<F, R>(
        builder: RequestBuilder,
        response_parser: F,
    ) -> std::result::Result<R, ElasticsearchError>
    where
        F: FnOnce(reqwest::StatusCode, String) -> std::result::Result<R, ElasticsearchError>,
    {
        match builder.send() {
            // the request was processed by ES, but maybe not successfully
            Ok(mut response) => {
                let code = response.status();
                let mut body_string = String::new();
                response
                    .read_to_string(&mut body_string)
                    .expect("unable to convert HTTP response to a string");

                if code.as_u16() != 200 {
                    // it wasn't a valid response code
                    Err(ElasticsearchError(Some(code), body_string))
                } else {
                    response_parser(code, body_string)
                }
            }

            // the request didn't reach ES
            Err(e) => Err(ElasticsearchError(e.status(), e.to_string())),
        }
    }
}
