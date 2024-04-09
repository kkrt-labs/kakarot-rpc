// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! RPC middleware to collect prometheus metrics on RPC calls.

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};

use crate::prometheus_handler::{
    register, CounterVec, HistogramOpts, HistogramVec, Opts, PrometheusError, Registry, U64,
};
use jsonrpsee::{server::middleware::rpc::RpcServiceT, types::Request, MethodResponse};
use pin_project_lite::pin_project;

/// Histogram time buckets in microseconds.
const HISTOGRAM_BUCKETS: [f64; 11] =
    [5.0, 25.0, 100.0, 500.0, 1_000.0, 2_500.0, 10_000.0, 25_000.0, 100_000.0, 1_000_000.0, 10_000_000.0];

/// Metrics for RPC middleware storing information about the number of requests started/completed,
/// calls started/completed and their timings.
#[derive(Debug, Clone)]
pub struct RpcMetrics {
    /// Histogram over RPC execution times.
    calls_time: HistogramVec,
    /// Number of calls started.
    calls_started: CounterVec<U64>,
    /// Number of calls completed.
    calls_finished: CounterVec<U64>,
}

impl RpcMetrics {
    /// Create an instance of metrics
    pub fn new(metrics_registry: Option<&Registry>) -> Result<Option<Self>, PrometheusError> {
        if let Some(metrics_registry) = metrics_registry {
            Ok(Some(Self {
                calls_time: register(
                    HistogramVec::new(
                        HistogramOpts::new("eth_rpc_calls_time", "Total time [μs] of processed RPC calls")
                            .buckets(HISTOGRAM_BUCKETS.to_vec()),
                        &["protocol", "method"],
                    )?,
                    metrics_registry,
                )?,
                calls_started: register(
                    CounterVec::new(
                        Opts::new("eth_rpc_calls_started", "Number of received RPC calls (unique un-batched requests)"),
                        &["protocol", "method"],
                    )?,
                    metrics_registry,
                )?,
                calls_finished: register(
                    CounterVec::new(
                        Opts::new(
                            "eth_rpc_calls_finished",
                            "Number of processed RPC calls (unique un-batched requests)",
                        ),
                        &["protocol", "method", "is_error"],
                    )?,
                    metrics_registry,
                )?,
            }))
        } else {
            Ok(None)
        }
    }
}

/// Metrics layer.
#[derive(Clone)]
pub struct MetricsLayer {
    inner: RpcMetrics,
    transport_label: &'static str,
}

impl MetricsLayer {
    /// Create a new [`MetricsLayer`].
    pub fn new(metrics: RpcMetrics, transport_label: &'static str) -> Self {
        Self { inner: metrics, transport_label }
    }
}

impl<S> tower::Layer<S> for MetricsLayer {
    type Service = Metrics<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Metrics::new(inner, self.inner.clone(), self.transport_label)
    }
}

/// Metrics middleware.
#[derive(Clone)]
pub struct Metrics<S> {
    service: S,
    metrics: RpcMetrics,
    transport_label: &'static str,
}

impl<S> Metrics<S> {
    /// Create a new metrics middleware.
    pub fn new(service: S, metrics: RpcMetrics, transport_label: &'static str) -> Metrics<S> {
        Metrics { service, metrics, transport_label }
    }
}

impl<'a, S> RpcServiceT<'a> for Metrics<S>
where
    S: Send + Sync + RpcServiceT<'a>,
{
    type Future = ResponseFuture<'a, S::Future>;

    fn call(&self, req: Request<'a>) -> Self::Future {
        let now = Instant::now();

        log::trace!(
            target: "rpc_metrics",
            "[{}] on_call name={} params={:?}",
            self.transport_label,
            req.method_name(),
            req.params(),
        );
        self.metrics.calls_started.with_label_values(&[self.transport_label, req.method_name()]).inc();

        ResponseFuture {
            fut: self.service.call(req.clone()),
            metrics: self.metrics.clone(),
            req,
            now,
            transport_label: self.transport_label,
        }
    }
}

pin_project! {
    /// Response future for metrics.
    pub struct ResponseFuture<'a, F> {
        #[pin]
        fut: F,
        metrics: RpcMetrics,
        req: Request<'a>,
        now: Instant,
        transport_label: &'static str,
    }
}

impl<'a, F> std::fmt::Debug for ResponseFuture<'a, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResponseFuture")
    }
}

impl<'a, F: Future<Output = MethodResponse>> Future for ResponseFuture<'a, F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        let res = this.fut.poll(cx);
        if let Poll::Ready(rp) = &res {
            let method_name = this.req.method_name();
            let transport_label = &this.transport_label;
            let now = this.now;
            let metrics = &this.metrics;

            log::trace!(target: "rpc_metrics", "[{transport_label}] on_response started_at={:?}", now);
            log::trace!(target: "rpc_metrics::extra", "[{transport_label}] result={:?}", rp);

            let micros = now.elapsed().as_micros();
            log::debug!(
                target: "rpc_metrics",
                "[{transport_label}] {method_name} call took {} μs",
                micros,
            );
            metrics.calls_time.with_label_values(&[transport_label, method_name]).observe(micros as _);
            metrics
                .calls_finished
                .with_label_values(&[
                    transport_label,
                    method_name,
                    // the label "is_error", so `success` should be regarded as false
                    // and vice-versa to be registrered correctly.
                    if rp.is_success() { "false" } else { "true" },
                ])
                .inc();
        }
        res
    }
}
