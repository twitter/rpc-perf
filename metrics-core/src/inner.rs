// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use std::any::Any;

use evmap::ShallowCopy;

use crate::{Counter, DynCow, Gauge, Metadata, MetricCommon, Summary};

/// The type of a metric.
///
/// This is provided for convenience but usually you'll want to match on
/// [`Metric`][metric].
///
/// [metric]: crate::Metric
#[allow(missing_docs)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum MetricType {
    Counter,
    Gauge,
    Summary,
}

impl std::fmt::Display for MetricType {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Counter => write!(fmt, "counter"),
            Self::Gauge => write!(fmt, "gauge"),
            Self::Summary => write!(fmt, "summary"),
        }
    }
}

/// A stored metric. Used for introspection.
#[allow(missing_docs)]
#[derive(Eq, PartialEq)]
pub enum Metric {
    Counter(DynCow<'static, dyn Counter>),
    Gauge(DynCow<'static, dyn Gauge>),
    Summary(DynCow<'static, dyn Summary>),
}

impl Metric {
    /// The type of the metric.
    pub fn ty(&self) -> MetricType {
        match self {
            Self::Counter(_) => MetricType::Counter,
            Self::Gauge(_) => MetricType::Gauge,
            Self::Summary(_) => MetricType::Summary,
        }
    }
}

/// A metric and its metadata.
#[derive(Eq, PartialEq)]
pub struct MetricInstance {
    pub(crate) inner: Metric,
    pub(crate) metadata: Metadata,
}

impl MetricInstance {
    pub(crate) fn new(metric: Metric, metadata: Metadata) -> Self {
        Self {
            inner: metric,
            metadata,
        }
    }

    /// The type of this metric.
    pub fn ty(&self) -> MetricType {
        self.inner.ty()
    }

    /// The metadata associated with this metric.
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// The current metric as a MetricCommon instance.
    pub fn as_metric(&self) -> &dyn MetricCommon {
        self.metric()
    }

    /// The inner type containing the actual metrics.
    pub fn metric(&self) -> &Metric {
        &self.inner
    }

    /// If this type is a counter, get a reference to that counter.
    pub fn as_counter(&self) -> Option<&dyn Counter> {
        match self.metric() {
            Metric::Counter(c) => Some(&**c),
            _ => None,
        }
    }

    /// If this type is a gauge, get a reference to that gauge.
    pub fn as_gauge(&self) -> Option<&dyn Gauge> {
        match self.metric() {
            Metric::Gauge(g) => Some(&**g),
            _ => None,
        }
    }

    /// If this type is a summary, get a reference to that summary.
    pub fn as_summary(&self) -> Option<&dyn Summary> {
        match self.metric() {
            Metric::Summary(h) => Some(&**h),
            _ => None,
        }
    }
}

impl MetricCommon for Metric {
    fn as_any(&self) -> Option<&dyn Any> {
        match self {
            Self::Counter(c) => c.as_any(),
            Self::Gauge(g) => g.as_any(),
            Self::Summary(h) => h.as_any(),
        }
    }
}

impl ShallowCopy for MetricInstance {
    unsafe fn shallow_copy(&mut self) -> Self {
        Self {
            inner: self.inner.shallow_copy(),
            metadata: self.metadata,
        }
    }
}

impl ShallowCopy for Metric {
    unsafe fn shallow_copy(&mut self) -> Self {
        match self {
            Self::Counter(x) => Self::Counter(x.shallow_copy()),
            Self::Gauge(x) => Self::Gauge(x.shallow_copy()),
            Self::Summary(x) => Self::Summary(x.shallow_copy()),
        }
    }
}
