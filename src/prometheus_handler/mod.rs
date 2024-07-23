// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{http::StatusCode, service::service_fn, Request, Response};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server,
};
use prometheus::{core::Collector, Encoder, TextEncoder};
use std::net::SocketAddr;

pub use prometheus::{
    self,
    core::{
        AtomicF64 as F64, AtomicI64 as I64, AtomicU64 as U64, GenericCounter as Counter,
        GenericCounterVec as CounterVec, GenericGauge as Gauge, GenericGaugeVec as GaugeVec,
    },
    exponential_buckets, Error as PrometheusError, Histogram, HistogramOpts, HistogramVec, Opts, Registry,
};

mod sourced;

pub use sourced::{MetricSource, SourcedCounter, SourcedGauge, SourcedMetric};

pub fn register<T: Clone + Collector + 'static>(metric: T, registry: &Registry) -> Result<T, PrometheusError> {
    registry.register(Box::new(metric.clone()))?;
    Ok(metric)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Hyper internal error.
    #[error(transparent)]
    Hyper(#[from] hyper::Error),

    /// Http request error.
    #[error(transparent)]
    Http(#[from] Box<dyn std::error::Error + Send + Sync>),

    /// i/o error.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Prometheus port {0} already in use.")]
    PortInUse(SocketAddr),
}

#[allow(clippy::unused_async)]
async fn request_metrics(
    req: Request<hyper::body::Incoming>,
    registry: Registry,
) -> Result<Response<Full<Bytes>>, hyper::http::Error> {
    if req.uri().path() == "/metrics" {
        let metric_families = registry.gather();
        let mut buffer = vec![];
        let encoder = TextEncoder::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();

        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", encoder.format_type())
            .body(Full::new(Bytes::from(buffer)))
    } else {
        Response::builder().status(StatusCode::NOT_FOUND).body(Full::new(Bytes::from("Not found.")))
    }
}

/// Initializes the metrics context, and starts an HTTP server
/// to serve metrics.
pub async fn init_prometheus(prometheus_addr: SocketAddr, registry: Registry) -> Result<(), Error> {
    let listener =
        tokio::net::TcpListener::bind(&prometheus_addr).await.map_err(|_| Error::PortInUse(prometheus_addr))?;

    init_prometheus_with_listener(listener, registry).await
}

/// Init prometheus using the given listener.
async fn init_prometheus_with_listener(listener: tokio::net::TcpListener, registry: Registry) -> Result<(), Error> {
    tracing::info!("〽️ Prometheus exporter started at {}", listener.local_addr().unwrap());

    loop {
        // getting the tcp stream and ignoring the remote address
        let (tcp, _) = listener.accept().await?;

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        let io = TokioIo::new(tcp);

        // making a clone of registry, as it will be used in the service_fn closure
        let registry = registry.clone();

        // Manufacturing a connection
        let conn = server::conn::auto::Builder::new(TokioExecutor::new());

        // set up the connection to use the service implemented by request_metrics fn
        // await for the res
        // and send it off
        conn.serve_connection(
            io,
            service_fn(move |req: Request<hyper::body::Incoming>| request_metrics(req, registry.clone())),
        )
        .await
        .map_err(Error::Http)?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;
    use hyper::Uri;
    use hyper_util::{
        client::legacy::{connect::HttpConnector, Client},
        rt::TokioExecutor,
    };

    #[tokio::test]
    async fn prometheus_works() {
        const METRIC_NAME: &str = "test_test_metric_name_test_test";

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("failed to create listener");

        let local_addr = listener.local_addr().expect("failed to get local addr");

        let registry = Registry::default();
        register(prometheus::Counter::new(METRIC_NAME, "yeah").expect("Creates test counter"), &registry)
            .expect("Registers the test metric");

        tokio::task::spawn(async {
            init_prometheus_with_listener(listener, registry).await.expect("failed to init prometheus");
        });

        let client: Client<HttpConnector, Full<Bytes>> = Client::builder(TokioExecutor::new()).build_http();

        let res = client
            .get(Uri::try_from(&format!("http://{local_addr}/metrics")).expect("failed to parse URI"))
            .await
            .expect("failed to request metrics");

        let buf = res.into_body().collect().await.unwrap().to_bytes();

        let body = String::from_utf8(buf.to_vec()).expect("failed to convert body to String");
        assert!(body.contains(&format!("{METRIC_NAME} 0")));
    }
}
