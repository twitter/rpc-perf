// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

//! This crate is used to render a waterfall style plot of a heatmap

#[macro_use]
extern crate log;

use hsl::HSL;
use rustcommon_atomics::*;
use rustcommon_datastructures::*;
use rusttype::{point, Font, PositionedGlyph, Scale};

use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

const MULTIPLIER: u64 = 1_000;

/// Render and save a waterfall from a `Heatmap` to a file. You can specify
/// `labels` for the value axis. And spacing of labels on the time axis is
/// specified by the `interval` in nanoseconds.
///
/// # Examples
///
/// ```
/// use rustcommon_datastructures::*;
/// use waterfall;
///
/// use std::collections::HashMap;
///
/// // create a heatmap with appropriate configuration for your dataset
/// let heatmap = Heatmap::<AtomicU64>::new(1_000_000, 2, 1_000_000, 5_000_000_000);
///
/// // add data into the heatmap
///
/// // decide on labels and generate waterfall
/// let mut labels = HashMap::new();
/// labels.insert(0, "0".to_string());
/// labels.insert(100, "100".to_string());
/// labels.insert(1000, "1000".to_string());
/// labels.insert(10000, "10000".to_string());
/// labels.insert(100000, "100000".to_string());
/// waterfall::save_waterfall(&heatmap, "waterfall.png", labels, 1_000_000_000);
/// ```
pub fn save_waterfall<S: ::std::hash::BuildHasher, T: 'static>(
    heatmap: &Heatmap<T>,
    file: &str,
    labels: HashMap<u64, String, S>,
    interval: u64,
) where
    T: Atomic + Unsigned + SaturatingArithmetic + Default,
    <T as Atomic>::Primitive: Default + PartialEq + Copy,
    u64: From<<T as Atomic>::Primitive>,
{
    debug!("saving waterfall");
    let height = heatmap.slices();
    let width = heatmap.buckets();

    // create image buffer
    let mut buffer = ImageBuffer::<ColorRgb>::new(width, height);

    let histogram =
        Histogram::<AtomicU64>::new(heatmap.highest_count() * MULTIPLIER, 6, None, None);
    for slice in heatmap {
        for b in slice.histogram().into_iter() {
            let weight = MULTIPLIER * u64::from(b.count()) / b.width();
            if (weight) > 0 {
                histogram.increment(weight, 1);
            }
        }
    }

    if let Some(min) = histogram.percentile(0.0) {
        let mid = histogram.percentile(0.50).unwrap();
        let high = histogram.percentile(0.99).unwrap();
        let max = histogram.percentile(1.0).unwrap();
        let low = 0;

        debug!(
            "min: {} low: {} mid: {} high: {} max: {}",
            min, low, mid, high, max
        );

        let mut values: Vec<u64> = labels.keys().cloned().collect();
        values.sort();
        let mut l = 0;
        for (y, slice) in heatmap.into_iter().enumerate() {
            for (x, b) in slice.histogram().into_iter().enumerate() {
                let weight = MULTIPLIER * u64::from(b.count()) / b.width();
                let value = color_from_value(weight, low, mid, high);
                buffer.set_pixel(x, y, value);
            }
        }

        if !values.is_empty() {
            let y = 0;
            let slice = heatmap.into_iter().next().unwrap();
            for (x, bucket) in slice.histogram().into_iter().enumerate() {
                let value = bucket.max();
                if value >= values[l] {
                    if let Some(label) = labels.get(&values[l]) {
                        let overlay = string_buffer(label, 25.0);
                        buffer.overlay(&overlay, x, y);
                        buffer.vertical_line(
                            x,
                            ColorRgb {
                                r: 255,
                                g: 255,
                                b: 255,
                            },
                        );
                    }
                    l += 1;
                    if l >= values.len() {
                        break;
                    }
                }
            }
        }
    }

    let mut begin = heatmap.begin_utc();
    for (y, slice) in heatmap.into_iter().enumerate() {
        let slice_begin = slice.begin_utc();
        if slice_begin - begin >= time::Duration::nanoseconds(interval as i64) {
            let label = format!("{}", slice_begin.rfc3339());
            let overlay = string_buffer(&label, 25.0);
            buffer.overlay(&overlay, 0, y + 2);
            buffer.horizontal_line(
                y,
                ColorRgb {
                    r: 255,
                    g: 255,
                    b: 255,
                },
            );
            begin = slice_begin;
        }
    }

    let _ = buffer.write_png(file);
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ColorRgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

struct ImageBuffer<T> {
    buffer: Vec<Vec<T>>,
    height: usize,
    width: usize,
}

fn string_buffer(string: &str, size: f32) -> ImageBuffer<ColorRgb> {
    // load font
    let font = Font::try_from_bytes(dejavu::sans_mono::regular() as &[u8]).unwrap();

    // size and scaling
    let height: f32 = size;
    let pixel_height = height.ceil() as usize;
    let scale = Scale {
        x: height * 1.0,
        y: height,
    };

    let v_metrics = font.v_metrics(scale);
    let offset = point(0.0, v_metrics.ascent);

    let glyphs: Vec<PositionedGlyph> = font.layout(string, scale, offset).collect();

    let width = glyphs
        .iter()
        .map(|g| g.unpositioned().h_metrics().advance_width)
        .fold(0.0, |x, y| x + y)
        .ceil() as usize;

    let mut overlay = ImageBuffer::<ColorRgb>::new(width, pixel_height);

    for g in glyphs {
        if let Some(bb) = g.pixel_bounding_box() {
            g.draw(|x, y, v| {
                let x = (x as i32 + bb.min.x) as usize;
                let y = (y as i32 + bb.min.y) as usize;
                if v > 0.25 {
                    overlay.set_pixel(
                        x,
                        y,
                        ColorRgb {
                            r: 255,
                            g: 255,
                            b: 255,
                        },
                    );
                }
            })
        }
    }

    overlay
}

/// maps a value to a color based on a low point, mid point, and high point
/// values below low will clip to black
/// mid point is the transition between luminosity (black-blue) and hue (blue->red) ramps
/// values above high will clip to red
fn color_from_value(value: u64, low: u64, mid: u64, high: u64) -> ColorRgb {
    let hsl = if value < low {
        HSL {
            h: 250.0,
            s: 1.0,
            l: 0.0,
        }
    } else if value < mid {
        HSL {
            h: 250.0,
            s: 1.0,
            l: (value as f64 / mid as f64) * 0.5,
        }
    } else if value < high {
        HSL {
            h: 250.0 - (250.0 * (value - mid) as f64 / (high - mid) as f64),
            s: 1.0,
            l: 0.5,
        }
    } else {
        HSL {
            h: 0.0,
            s: 1.0,
            l: 0.5,
        }
    };

    let (r, g, b) = hsl.to_rgb();

    ColorRgb { r, g, b }
}

impl ImageBuffer<ColorRgb> {
    pub fn new(width: usize, height: usize) -> ImageBuffer<ColorRgb> {
        let background = ColorRgb { r: 0, g: 0, b: 0 };
        let mut row = Vec::<ColorRgb>::with_capacity(width);
        for _ in 0..width {
            row.push(background);
        }
        let mut buffer = Vec::<Vec<ColorRgb>>::with_capacity(height);
        for _ in 0..height {
            buffer.push(row.clone());
        }
        ImageBuffer {
            buffer,
            height,
            width,
        }
    }

    pub fn write_png(self, file: &str) -> Result<(), &'static str> {
        let mut buffer = Vec::<u8>::with_capacity(self.height * self.width);
        for row in 0..self.height {
            for col in 0..self.width {
                let pixel = self.buffer[row][col];
                buffer.push(pixel.r);
                buffer.push(pixel.g);
                buffer.push(pixel.b);
            }
        }
        let path = &Path::new(&file);
        if let Ok(file) = File::create(path) {
            let w = BufWriter::new(file);
            let mut encoder = png::Encoder::new(w, self.width as u32, self.height as u32);
            encoder.set_color(png::ColorType::RGB);
            encoder.set_depth(png::BitDepth::Eight);
            if let Ok(mut writer) = encoder.write_header() {
                if writer.write_image_data(&buffer).is_ok() {
                    Ok(())
                } else {
                    Err("Error writing PNG data")
                }
            } else {
                Err("Error writing PNG header")
            }
        } else {
            Err("Error creating file")
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, value: ColorRgb) {
        if x < self.width && y < self.height {
            self.buffer[y][x] = value;
        }
    }

    pub fn overlay(&mut self, other: &ImageBuffer<ColorRgb>, x: usize, y: usize) {
        let ignore = ColorRgb { r: 0, g: 0, b: 0 };
        for sx in 0..other.width {
            for sy in 0..other.height {
                if (other.buffer[sy][sx] != ignore)
                    && (((sy + y) < self.height) && ((sx + x) < self.width))
                {
                    self.buffer[(sy + y)][(sx + x)] = other.buffer[sy][sx];
                }
            }
        }
    }

    pub fn horizontal_line(&mut self, y: usize, color: ColorRgb) {
        for x in 0..self.width {
            self.buffer[y][x] = color;
        }
    }

    pub fn vertical_line(&mut self, x: usize, color: ColorRgb) {
        for y in 0..self.height {
            self.buffer[y][x] = color;
        }
    }
}
