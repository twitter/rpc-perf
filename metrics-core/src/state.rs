// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use std::borrow::Cow;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use once_cell::sync::Lazy;
use thread_local::CachedThreadLocal;

use crate::{
    Instant, Metadata, Metric, MetricError, MetricInstance, MetricType, MetricValue, RegisterError,
    UnregisterError,
};

#[derive(Clone)]
pub(crate) struct GlobalMetadata {
    error_fn: Arc<dyn Fn(MetricError) + Send + Sync>,
}

impl Default for GlobalMetadata {
    fn default() -> Self {
        Self {
            error_fn: Arc::new(default_error_fn),
        }
    }
}

fn default_error_fn(err: MetricError) {
    warn!("A metric error occurred: {}", err);
}

type WriteHandle = evmap::WriteHandle<Cow<'static, str>, MetricInstance, GlobalMetadata>;
type ReadHandle = evmap::ReadHandle<Cow<'static, str>, MetricInstance, GlobalMetadata>;
type ReadHandleFactory =
    evmap::ReadHandleFactory<Cow<'static, str>, MetricInstance, GlobalMetadata>;

// TODO(sean): Use alternate OnceCell implementation here so that
//             we can get rid of this variable here.
static INITIALIZED: AtomicBool = AtomicBool::new(false);
static STATE: Lazy<State> = Lazy::new(|| {
    INITIALIZED.store(true, Ordering::Relaxed);
    State::new()
});

pub(crate) struct State {
    writer: Mutex<WriteHandle>,
    factory: ReadHandleFactory,
    tls: CachedThreadLocal<ReadHandle>,
}

impl State {
    fn new() -> Self {
        let (reader, writer) = evmap::with_meta(GlobalMetadata::default());

        Self {
            writer: Mutex::new(writer),
            factory: reader.factory(),
            tls: CachedThreadLocal::new(),
        }
    }

    fn reader(&self) -> &ReadHandle {
        self.tls.get_or(|| self.factory.handle())
    }

    /// Get the state if it has been initialized otherwise just return None.
    ///
    /// This is useful for cases where we wouldn't do anything with an empty
    /// state anyways. (specifically, recording a value to a metric. If the
    /// state hasn't been set up then the metric definitely doesn't exist.)
    #[inline]
    pub(crate) fn get() -> Option<&'static Self> {
        if INITIALIZED.load(Ordering::Relaxed) {
            Some(&*STATE)
        } else {
            None
        }
    }

    /// If the value hasn't been initializes then it creates it as well
    #[inline]
    pub(crate) fn get_force() -> &'static Self {
        &*STATE
    }

    /// Register a new metric, returns whether the metric was registered
    /// successfully
    pub(crate) fn register_metric(
        &self,
        name: Cow<'static, str>,
        metric: Metric,
        metadata: Metadata,
    ) -> Result<(), RegisterError> {
        let mut writer = self.writer.lock().unwrap();
        let instance = MetricInstance::new(metric, metadata);

        if writer.is_destroyed() {
            return Err(RegisterError::LibraryShutdown);
        }

        if writer.contains_key(&name) {
            return Err(RegisterError::MetricAlreadyExists);
        }

        writer.update(name, instance);

        writer.refresh();

        Ok(())
    }

    /// Unregister an existing metric, if the metric doesn't exist then this
    /// method does nothing.
    ///
    // TODO: I'd like to somehow return the existing entry in the hash table.
    //       Unfortunately, evmap doesn't offer an API to get the removed value
    //       or even to tell if we removed a value.
    pub(crate) fn unregister_metric(&self, name: &str) -> Result<(), UnregisterError> {
        let mut writer = self.writer.lock().unwrap();

        if writer.is_destroyed() {
            return Err(UnregisterError::LibraryShutdown);
        }

        if !writer.contains_key(name) {
            return Err(UnregisterError::NoSuchMetric);
        }

        // Hack to get a Cow with a static lifetime
        // this is safe since we immediately call writer.refresh
        writer.empty(Cow::Borrowed(unsafe { &*(name as *const str) }));
        writer.refresh();

        Ok(())
    }

    pub(crate) fn set_error_fn(&self, err_fn: Arc<dyn Fn(MetricError) + Send + Sync>) {
        let mut writer = self.writer.lock().unwrap();

        let mut meta = writer.meta().unwrap();
        meta.error_fn = err_fn;
        writer.set_meta(meta);

        writer.refresh();
    }

    #[cold]
    fn error(&self, err: MetricError) {
        let reader = self.reader();

        if let Some(meta) = reader.meta() {
            (*meta.error_fn)(err);
        }
    }

    pub(crate) fn record_value(&self, name: &str, value: MetricValue, count: u64, time: Instant) {
        let reader = self.reader();

        reader.get_and(name, |val| match val[0].metric() {
            Metric::Counter(counter) => match value.as_u64() {
                Some(val) => counter.store(time, val),
                _ => self.error(MetricError::invalid_unsigned(
                    name,
                    value.as_i64_unchecked(),
                )),
            },
            Metric::Gauge(gauge) => match value.as_i64() {
                Some(val) => gauge.store(time, val),
                _ => self.error(MetricError::invalid_signed(name, value.as_u64_unchecked())),
            },
            Metric::Summary(histogram) => match value.as_u64() {
                Some(val) => histogram.record(time, val, count),
                _ => self.error(MetricError::invalid_unsigned(
                    name,
                    value.as_i64_unchecked(),
                )),
            },
        });
    }

    pub(crate) fn record_increment(&self, name: &str, value: MetricValue, time: Instant) {
        let reader = self.reader();

        reader.get_and(name, |val| match val[0].metric() {
            Metric::Counter(counter) => match value.as_u64() {
                Some(val) => counter.add(time, val),
                None => self.error(MetricError::invalid_unsigned(
                    name,
                    value.as_i64_unchecked(),
                )),
            },
            Metric::Gauge(gauge) => match value.as_i64() {
                Some(val) => gauge.add(time, val),
                None => self.error(MetricError::invalid_signed(name, value.as_u64_unchecked())),
            },
            Metric::Summary(_) => {
                self.error(MetricError::invalid_increment(name, MetricType::Summary))
            }
        });
    }

    pub(crate) fn record_decrement(&self, name: &str, value: MetricValue, time: Instant) {
        let reader = self.reader();

        reader.get_and(name, |val| match val[0].metric() {
            Metric::Gauge(gauge) => match value.as_i64() {
                Some(val) => gauge.sub(time, val),
                None => self.error(MetricError::invalid_signed(name, value.as_u64_unchecked())),
            },
            metric => self.error(MetricError::invalid_decrement(name, metric.ty())),
        });
    }

    pub(crate) fn record_counter_value(&self, name: &str, value: u64, time: Instant) {
        let reader = self.reader();

        reader.get_and(name, |val| match val[0].metric() {
            Metric::Counter(counter) => counter.store(time, value),
            metric => self.error(MetricError::wrong_type(
                name,
                MetricType::Counter,
                metric.ty(),
            )),
        });
    }

    pub(crate) fn record_gauge_value(&self, name: &str, value: i64, time: Instant) {
        let reader = self.reader();

        reader.get_and(name, |val| match val[0].metric() {
            Metric::Gauge(gauge) => gauge.store(time, value),
            metric => self.error(MetricError::wrong_type(
                name,
                MetricType::Gauge,
                metric.ty(),
            )),
        });
    }

    pub(crate) fn for_each_metric<F, R, C>(&self, mut func: F) -> C
    where
        C: std::iter::FromIterator<R>,
        F: FnMut(&str, &MetricInstance) -> R,
    {
        let reader = self.reader();

        reader.map_into(move |key, vals| func(&*key, &vals[0]))
    }
}
