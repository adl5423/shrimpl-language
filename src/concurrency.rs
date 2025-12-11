// src/concurrency.rs
//
// Lightweight concurrency helpers for Shrimpl, focused on concurrent
// HTTP work. This is intentionally narrow so it can be safely called
// from inside the interpreter.
//
// Example builtin usage:
//   - http_get_many(urls_json) -> JSON array of bodies
//   - http_get_json_many(urls_json) -> JSON array of parsed values

use futures::future::join_all;
use reqwest::Client;
use serde_json::Value as JsonValue;

/// Concurrently GET multiple URLs and return their bodies as strings.
///
/// `urls` is a vector of absolute URLs.
pub async fn http_get_many(urls: Vec<String>) -> Result<Vec<String>, reqwest::Error> {
    let client = Client::new();

    let futures = urls.into_iter().map(|url| {
        let client = client.clone();
        async move {
            let body = client.get(url).send().await?.text().await?;
            Ok::<String, reqwest::Error>(body)
        }
    });

    let results = join_all(futures).await;

    let mut out = Vec::new();
    for res in results {
        out.push(res?);
    }

    Ok(out)
}

/// Concurrent GET with JSON parsing.
///
/// Returns a JSON array of each response body; failures are surfaced as
/// a single error (the first encountered).
pub async fn http_get_json_many(urls: Vec<String>) -> Result<JsonValue, reqwest::Error> {
    let bodies = http_get_many(urls).await?;
    let mut arr = Vec::new();
    for body in bodies {
        let v: JsonValue = serde_json::from_str(&body).unwrap_or(JsonValue::String(body));
        arr.push(v);
    }
    Ok(JsonValue::Array(arr))
}
