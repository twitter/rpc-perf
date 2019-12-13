#![allow(deprecated)]

use plotters::coord::AsRangedCoord;
use plotters::prelude::*;

use superslice::Ext as _;

use atomics::AtomicU64;
use datastructures::AtomicDDSketch;

use std::error::Error;
use std::ffi::OsStr;
use std::marker::PhantomData;

mod scales;

pub use scales::*;

macro_rules! hexcolour {
    ($colour:literal) => {
        RGBColor(
            (($colour & 0xFF0000) >> 16) as u8,
            (($colour & 0x00FF00) >> 8) as u8,
            (($colour & 0x0000FF) >> 0) as u8,
        )
    };
}

const COLOURS: &[RGBColor] = &[
    hexcolour!(0xAA0000),
    hexcolour!(0x0000FF),
    hexcolour!(0x888888),
    hexcolour!(0xDDCC77),
    hexcolour!(0x999933),
    hexcolour!(0x332288),
    hexcolour!(0x117733),
    hexcolour!(0x88CCEE),
    hexcolour!(0x882255),
    hexcolour!(0x44AA99),
    hexcolour!(0xAA4499),
    hexcolour!(0xCC6677),
];

pub fn plot_linear(
    sketch1: &AtomicDDSketch<AtomicU64>,
    sketch2: &AtomicDDSketch<AtomicU64>,
) -> Result<(), Box<dyn Error>> {
    let mut quantiles: Vec<_> = (1..10000).map(|x| x as f64 / 10000.0).rev().collect();

    quantiles.sort_by(|x, y| x.partial_cmp(y).unwrap());

    let p995 = sketch1.quantile(0.995).max(sketch2.quantile(0.995)) as f64 / 1000.0;

    let root = BitMapBackend::new("linear.png", (1920, 1080)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption("Pelikan-Twemcache Latency Distribution", ("Arial", 40))
        .margin(20)
        .set_label_area_size(LabelAreaPosition::Left, 100)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .build_ranged(0.0..1.0, 0.0..p995)?;

    chart
        .configure_mesh()
        .y_desc("Latency (in µs)")
        .x_desc("Quantile")
        .x_label_style(("Arial", 20))
        .y_label_style(("Arial", 20))
        .draw()?;

    let series = quantiles.iter().map(|&x| (x, sketch1.quantile(x) as f64 / 1000.0));
    chart
        .draw_series(LineSeries::new(series, COLOURS[0].stroke_width(2)))?
        .label("C")
        .legend(move |(x, y)| Path::new(vec![(x, y), (x + 20, y)], &COLOURS[0]));

    let series = quantiles.iter().map(|&x| (x, sketch2.quantile(x) as f64 / 1000.0));
    chart
        .draw_series(LineSeries::new(series, COLOURS[1].stroke_width(2)))?
        .label("Rust")
        .legend(move |(x, y)| Path::new(vec![(x, y), (x + 20, y)], &COLOURS[1]));

    chart
        .configure_series_labels()
        .background_style(WHITE.filled())
        .draw()?;

    Ok(())
}

fn lerp(a: f64, b: f64, f: f64) -> f64 {
    a * (1.0 - f) + b * f
}

pub fn plot_log(
    sketch1: &AtomicDDSketch<AtomicU64>,
    sketch2: &AtomicDDSketch<AtomicU64>,
) -> Result<(), Box<dyn Error>> {
    let offset = 0.00009f64;
    let start = offset.ln();
    let end = (0.5f64).ln();

    let mut quantiles: Vec<_> = (1..=10000)
        .map(|x| x as f64 / 10000.0)
        .map(|x| 1.0 - lerp(start, end, x).exp())
        .rev()
        .collect();

    quantiles.sort_by(|x, y| x.partial_cmp(y).unwrap());

    let max1 = sketch1.quantile(1.0) as f64 / 1000.0;
    let max2 = sketch2.quantile(1.0) as f64 / 1000.0;
    let max = max1.max(max2);
    let p50 = sketch1.quantile(0.5).min(sketch2.quantile(0.5)) as f64 / 1000.0;

    let root = BitMapBackend::new("log.png", (1920, 1080)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption("Pelikan-Twemcache Latency Tail", ("Arial", 40))
        .margin(20)
        .set_label_area_size(LabelAreaPosition::Left, 100)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .build_ranged(InvLogCoord::new(offset), LogRange(p50 * 0.8..max * 1.5))?;

    chart
        .configure_mesh()
        .y_desc("Latency (in µs)")
        .x_desc("Quantile")
        .x_label_style(("Arial", 20))
        .y_label_style(("Arial", 20))
        .draw()?;

    let series = quantiles.iter().map(|&x| (x, sketch1.quantile(x) as f64 / 1000.0));
    chart
        .draw_series(LineSeries::new(series, COLOURS[0].stroke_width(2)))?
        .label("C")
        .legend(move |(x, y)| Path::new(vec![(x, y), (x + 20, y)], &COLOURS[0]));

    let series = quantiles.iter().map(|&x| (x, sketch2.quantile(x) as f64 / 1000.0));
    chart
        .draw_series(LineSeries::new(series, COLOURS[1].stroke_width(2)))?
        .label("Rust")
        .legend(move |(x, y)| Path::new(vec![(x, y), (x + 20, y)], &COLOURS[1]));

    chart
        .configure_series_labels()
        .background_style(WHITE.filled())
        .draw()?;

    Ok(())
}


pub trait Summary {
    fn quantile(&self, q: f64) -> f64;
}

pub trait Digest {
    fn new() -> Self;

    fn add_data<I>(&mut self, data: I)
    where
        I: Iterator<Item = f64>;

    type Summary: Summary;
    type Param;

    fn summarize(&self, param: &Self::Param) -> Self::Summary;
}

pub fn read_data(file: &str) -> Vec<f64> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = File::open(file).expect("Unable to open the file");
    let mut reader = BufReader::new(file);

    let mut res = vec![];
    let mut buf = String::new();

    let mut line = 0;
    while reader
        .read_line(&mut buf)
        .expect(&format!("Error at line {}", line))
        != 0
    {
        let val = buf
            .trim()
            .parse()
            .expect(&format!("File contained invalid number at line {}", line));

        res.push(val);
        buf.clear();
        line += 1;
    }

    res
}

fn exact_quantile(q: f64, data: &[f64]) -> f64 {
    let index = (q * data.len() as f64) as usize;
    data[index.min(data.len() - 1).max(0)]
}

fn exact_rank(val: f64, data: &[f64]) -> usize {
    data.upper_bound_by(|x| x.partial_cmp(&val).unwrap())
}
fn exact_rank_min(val: f64, data: &[f64]) -> usize {
    (data.lower_bound_by(|x| x.partial_cmp(&val).unwrap()) + 1).min(data.len())
}

fn absdiff(x: usize, y: usize) -> usize {
    let max = x.max(y);
    let min = x.min(y);

    max - min
}

fn log_interp2(pt: f64, min: f64, max: f64) -> f64 {
    min.powf(pt) * max.powf(1.0 - pt)
}

fn double_log_interp(pt: f64, min: f64, max: f64) -> f64 {
    let mid = (min + max) / 2.0;
    if pt > 0.5 {
        log_interp2((1.0 - pt) * 2.0, max, mid)
    } else {
        log_interp2(pt * 2.0, mid, min)
    }
}

pub struct PlotConfig<'data, D> {
    x_desc: String,
    y_desc: String,
    caption: String,
    data_label: String,
    size: (u32, u32),

    data: &'data [f64],

    _marker: PhantomData<D>,
}

impl<'d, D: Digest> PlotConfig<'d, D> {
    pub fn new(data: &'d [f64]) -> Self {
        Self {
            x_desc: "quantile".to_owned(),
            y_desc: String::new(),
            caption: String::new(),
            data_label: String::new(),
            size: (1080, 720),

            data,

            _marker: PhantomData,
        }
    }

    pub fn caption(&mut self, caption: impl AsRef<str>) -> &mut Self {
        self.caption = caption.as_ref().to_owned();
        self
    }

    pub fn data_label(&mut self, label: impl AsRef<str>) -> &mut Self {
        self.data_label = label.as_ref().to_owned();
        self
    }

    pub fn x_desc(&mut self, x_desc: impl AsRef<str>) -> &mut Self {
        self.x_desc = x_desc.as_ref().to_owned();
        self
    }

    pub fn y_desc(&mut self, y_desc: impl AsRef<str>) -> &mut Self {
        self.y_desc = y_desc.as_ref().to_owned();
        self
    }

    pub fn size(&mut self, size: (u32, u32)) -> &mut Self {
        self.size = size;
        self
    }

    fn quantiles(&self) -> Vec<f64> {
        let mut quantiles: Vec<_> = (1..10000)
            .map(|x| x as f64 / 10000.0)
            .map(|x| 1.0 - double_log_interp(x, 0.0001, 0.9999))
            .rev()
            .collect();

        quantiles.sort_by(|x, y| x.partial_cmp(y).unwrap());

        quantiles
    }

    pub fn plot_values<X, Y, XR, YR, L>(
        &mut self,
        x_axis: X,
        y_axis: Y,
        filename: impl AsRef<OsStr>,
        estimates: &[D::Param],
        mut label_fn: impl FnMut(&D::Param) -> L,
    ) -> Result<&mut Self, Box<dyn Error>>
    where
        X: FnOnce() -> XR,
        Y: FnOnce(/* min: */ f64, /* max: */ f64) -> YR,
        XR: AsRangedCoord<Value = f64>,
        YR: AsRangedCoord<Value = f64>,
        <XR as AsRangedCoord>::CoordDescType: Ranged<ValueType = f64>,
        <YR as AsRangedCoord>::CoordDescType: Ranged<ValueType = f64>,
        L: AsRef<str>,
    {
        let mut sorted = self.data.to_vec();
        sorted.sort_by(|x, y| x.partial_cmp(y).unwrap());

        let quantiles = self.quantiles();

        let root = BitMapBackend::new(filename.as_ref(), self.size).into_drawing_area();
        root.fill(&WHITE)?;

        let min = *sorted.first().unwrap();
        let max = *sorted.last().unwrap();

        let mut chart = ChartBuilder::on(&root)
            .caption(&self.caption, ("Arial", 40))
            .margin(20)
            .set_label_area_size(LabelAreaPosition::Left, 100)
            .set_label_area_size(LabelAreaPosition::Bottom, 40)
            .build_ranged(x_axis(), y_axis(min, max))?;

        chart
            .configure_mesh()
            .y_desc(&self.y_desc)
            .x_desc(&self.x_desc)
            .draw()?;

        let values = quantiles
            .iter()
            .copied()
            .map(|x| (x, exact_quantile(x, &sorted) as f64));

        let mut digest = D::new();
        digest.add_data(self.data.iter().copied());

        for (i, param) in estimates.iter().enumerate() {
            let summary = digest.summarize(param);

            let series = quantiles.iter().copied().map(|x| (x, summary.quantile(x)));

            chart
                .draw_series(LineSeries::new(series, COLOURS[i].stroke_width(2)))?
                .label(label_fn(param).as_ref())
                .legend(move |(x, y)| Path::new(vec![(x, y), (x + 20, y)], &COLOURS[i]));
        }

        chart
            .draw_series(LineSeries::new(values, BLACK.stroke_width(2)))?
            .label(&self.data_label)
            .legend(move |(x, y)| Path::new(vec![(x, y), (x + 20, y)], &BLACK));

        chart
            .configure_series_labels()
            .background_style(WHITE.filled())
            .draw()?;

        Ok(self)
    }

    pub fn plot_errors<X, Y, XR, YR>(
        &mut self,
        x_axis_fn: X,
        y_axis_fn: Y,
        filename: impl AsRef<OsStr>,
        param: &D::Param,
        mut err_fn: impl FnMut(
            /*q:*/ f64,
            /*sorted:*/ &[f64],
            /*summary:*/ &D::Summary,
        ) -> f64,
        into_percent: bool,
    ) -> Result<&mut Self, Box<dyn Error>>
    where
        X: FnOnce() -> XR,
        Y: FnOnce(/* max_error: */ f64) -> YR,
        XR: AsRangedCoord<Value = f64>,
        YR: AsRangedCoord<Value = f64>,
        <XR as AsRangedCoord>::CoordDescType: Ranged<ValueType = f64>,
        <YR as AsRangedCoord>::CoordDescType: Ranged<ValueType = f64>,
    {
        let mut sorted = self.data.to_vec();
        sorted.sort_by(|x, y| x.partial_cmp(y).unwrap());

        let root = BitMapBackend::new(filename.as_ref(), self.size).into_drawing_area();
        root.fill(&WHITE)?;

        let quantiles = self.quantiles();

        let mut digest = D::new();
        digest.add_data(self.data.iter().copied());

        let summary = digest.summarize(param);

        let mut max_err = 0.0;
        let mult = if into_percent { 100.0 } else { 1.0 };

        let values: Vec<_> = quantiles
            .iter()
            .copied()
            .map(|q| {
                let err = err_fn(q, &sorted, &summary);

                if err > max_err {
                    max_err = err;
                }

                Circle::new((q, err * mult), 3, BLACK.filled())
            })
            .collect();

        let mut chart = ChartBuilder::on(&root)
            .caption(&self.caption, ("Arial", 40))
            .margin(20)
            .set_label_area_size(LabelAreaPosition::Left, 60)
            .set_label_area_size(LabelAreaPosition::Bottom, 40)
            .build_ranged(x_axis_fn(), y_axis_fn(max_err * mult))?;

        chart
            .configure_mesh()
            .y_desc(&self.y_desc)
            .x_desc(&self.x_desc)
            .draw()?;

        chart
            .draw_series(values)?
            .label(&self.data_label)
            .legend(|(x, y)| Path::new(vec![(x, y), (x + 20, y)], &BLACK));

        chart
            .configure_series_labels()
            .background_style(WHITE.filled())
            .draw()?;
        Ok(self)
    }

    pub fn plot_rank_errors<X, Y, XR, YR>(
        &mut self,
        x_axis_fn: X,
        y_axis_fn: Y,
        filename: impl AsRef<OsStr>,
        param: &D::Param,
    ) -> Result<&mut Self, Box<dyn Error>>
    where
        X: FnOnce() -> XR,
        Y: FnOnce(/* max_error: */ f64) -> YR,
        XR: AsRangedCoord<Value = f64>,
        YR: AsRangedCoord<Value = f64>,
        <XR as AsRangedCoord>::CoordDescType: Ranged<ValueType = f64>,
        <YR as AsRangedCoord>::CoordDescType: Ranged<ValueType = f64>,
    {
        self.plot_errors(
            x_axis_fn,
            y_axis_fn,
            filename,
            param,
            |q, sorted, summary| {
                let exact = exact_quantile(q, &sorted);
                let approx = summary.quantile(q);

                let approx_rank_max = exact_rank(approx, &sorted);
                let approx_rank_min = exact_rank_min(approx, &sorted);
                let exact_rank = exact_rank(exact, &sorted);

                let rank_err =
                    absdiff(exact_rank, approx_rank_max).min(absdiff(exact_rank, approx_rank_min));

                rank_err as f64 / sorted.len() as f64
            },
            true,
        )
    }

    pub fn plot_rel_value_errors<X, Y, XR, YR>(
        &mut self,
        x_axis_fn: X,
        y_axis_fn: Y,
        filename: impl AsRef<OsStr>,
        param: &D::Param,
    ) -> Result<&mut Self, Box<dyn Error>>
    where
        X: FnOnce() -> XR,
        Y: FnOnce(/* max_error: */ f64) -> YR,
        XR: AsRangedCoord<Value = f64>,
        YR: AsRangedCoord<Value = f64>,
        <XR as AsRangedCoord>::CoordDescType: Ranged<ValueType = f64>,
        <YR as AsRangedCoord>::CoordDescType: Ranged<ValueType = f64>,
    {
        self.plot_errors(
            x_axis_fn,
            y_axis_fn,
            filename,
            param,
            |q, sorted, summary| {
                let exact = exact_quantile(q, &sorted);
                let approx = summary.quantile(q);

                let val_err = (exact - approx).abs();

                if exact == approx {
                    0.0
                } else {
                    val_err as f64 / exact as f64
                }
            },
            true,
        )
    }

    pub fn plot_abs_value_errors<X, Y, XR, YR>(
        &mut self,
        x_axis_fn: X,
        y_axis_fn: Y,
        filename: impl AsRef<OsStr>,
        param: &D::Param,
    ) -> Result<&mut Self, Box<dyn Error>>
    where
        X: FnOnce() -> XR,
        Y: FnOnce(/* max_error: */ f64) -> YR,
        XR: AsRangedCoord<Value = f64>,
        YR: AsRangedCoord<Value = f64>,
        <XR as AsRangedCoord>::CoordDescType: Ranged<ValueType = f64>,
        <YR as AsRangedCoord>::CoordDescType: Ranged<ValueType = f64>,
    {
        self.plot_errors(
            x_axis_fn,
            y_axis_fn,
            filename,
            param,
            |q, sorted, summary| {
                let exact = exact_quantile(q, &sorted);
                let approx = summary.quantile(q);

                (exact - approx).abs()
            },
            false,
        )
    }

    pub fn plot_errors_and_values<X, Y, XR, YR, L>(
        &mut self,
        x_axis_fn: X,
        y_axis_fn: Y,
        filename: impl AsRef<OsStr>,
        param: &D::Param,
        mut err_fn: impl FnMut(
            /*q:*/ f64,
            /*sorted:*/ &[f64],
            /*summary:*/ &D::Summary,
        ) -> f64,
        error_label: impl AsRef<str>,
        into_percent: bool,
        mut label_fn: impl FnMut(&D::Param) -> L,
    ) -> Result<&mut Self, Box<dyn Error>>
    where
        X: FnOnce() -> XR,
        Y: FnOnce(/* max_error:*/ f64, /*max:*/ f64) -> YR,
        XR: AsRangedCoord<Value = f64>,
        YR: AsRangedCoord<Value = f64>,
        <XR as AsRangedCoord>::CoordDescType: Ranged<ValueType = f64>,
        <YR as AsRangedCoord>::CoordDescType: Ranged<ValueType = f64>,
        L: AsRef<str>,
    {
        let mut sorted = self.data.to_vec();
        sorted.sort_by(|x, y| x.partial_cmp(y).unwrap());

        let root = BitMapBackend::new(filename.as_ref(), self.size).into_drawing_area();
        root.fill(&WHITE)?;

        let quantiles = self.quantiles();

        let mut digest = D::new();
        digest.add_data(self.data.iter().copied());

        let summary = digest.summarize(param);

        let mut max_err = 0.0;
        let mult = if into_percent { 100.0 } else { 1.0 };

        let errors: Vec<_> = quantiles
            .iter()
            .copied()
            .map(|q| {
                let err = err_fn(q, &sorted, &summary);

                if err > max_err {
                    max_err = err;
                }

                (q, err * mult)
            })
            .collect();

        let values: Vec<_> = quantiles
            .iter()
            .copied()
            .map(|q| (q, exact_quantile(q, &sorted)))
            .collect();
        let estimates: Vec<_> = quantiles
            .iter()
            .copied()
            .map(|q| (q, summary.quantile(q)))
            .collect();

        let max = *sorted.last().unwrap();

        let mut chart = ChartBuilder::on(&root)
            .caption(&self.caption, ("Arial", 40))
            .margin(20)
            .set_label_area_size(LabelAreaPosition::Left, 60)
            .set_label_area_size(LabelAreaPosition::Bottom, 40)
            .build_ranged(x_axis_fn(), y_axis_fn(max_err * mult, max))?;

        chart
            .configure_mesh()
            .y_desc(&self.y_desc)
            .x_desc(&self.x_desc)
            .draw()?;

        chart.draw_series(LineSeries::new(errors.iter().copied(), RED.stroke_width(1)))?;

        chart
            .draw_series(AreaSeries::new(errors, 0.0, &RED.mix(0.2)))?
            .label(error_label.as_ref())
            .legend(|(x, y)| Path::new(vec![(x, y), (x + 20, y)], &RED));

        chart
            .draw_series(LineSeries::new(values, BLACK.stroke_width(2)))?
            .label(&self.data_label)
            .legend(|(x, y)| Path::new(vec![(x, y), (x + 20, y)], &BLACK));

        chart
            .draw_series(LineSeries::new(estimates, BLUE.stroke_width(2)))?
            .label(label_fn(&param).as_ref())
            .legend(|(x, y)| Path::new(vec![(x, y), (x + 20, y)], &BLUE));

        chart
            .configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .position(SeriesLabelPosition::UpperLeft)
            .draw()?;

        Ok(self)
    }

    pub fn plot_abs_errors_and_vals<X, Y, XR, YR, L>(
        &mut self,
        x_axis_fn: X,
        y_axis_fn: Y,
        filename: impl AsRef<OsStr>,
        param: &D::Param,
        error_label: impl AsRef<str>,
        label_fn: impl FnMut(&D::Param) -> L,
    ) -> Result<&mut Self, Box<dyn Error>>
    where
        X: FnOnce() -> XR,
        Y: FnOnce(/* max_error:*/ f64, /*max:*/ f64) -> YR,
        XR: AsRangedCoord<Value = f64>,
        YR: AsRangedCoord<Value = f64>,
        <XR as AsRangedCoord>::CoordDescType: Ranged<ValueType = f64>,
        <YR as AsRangedCoord>::CoordDescType: Ranged<ValueType = f64>,
        L: AsRef<str>,
    {
        self.plot_errors_and_values(
            x_axis_fn,
            y_axis_fn,
            filename,
            param,
            |q, sorted, summary| {
                let exact = exact_quantile(q, sorted);
                let approx = summary.quantile(q);

                (exact - approx).abs()
            },
            error_label,
            false,
            label_fn,
        )?;

        Ok(self)
    }
}
