//  Copyright 2019 Twitter, Inc
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

use hsl::HSL;
use png::HasParameters;
use rusttype::{point, FontCollection, PositionedGlyph, Scale};

use datastructures::histogram::{Histogram, Latched};
use datastructures::Heatmap;
use logger::*;

use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

pub fn save_waterfall(
    heatmap: &Heatmap,
    file: &str,
    labels: HashMap<usize, String>,
    interval: usize,
) {
    debug!("saving waterfall");
    let height = heatmap.slices();
    let width = heatmap.buckets();

    // create image buffer
    let mut buffer = ImageBuffer::<ColorRgb>::new(width, height);

    let mut y = 0;
    let histogram = Latched::new(0, heatmap.highest_count(), 3);
    for slice in heatmap {
        for b in slice.histogram() {
            let magnitude = (b.count() as f64 / b.width() as f64).ceil() as usize;
            if magnitude > 0 {
                histogram.incr(magnitude, 1);
            }
        }
    }

    let min = histogram.percentile(0.0).unwrap();
    let low = histogram.percentile(0.01).unwrap();
    let mid = histogram.percentile(0.50).unwrap();
    let high = histogram.percentile(0.99).unwrap();
    let max = histogram.percentile(1.0).unwrap();

    debug!(
        "min: {} low: {} mid: {} high: {} max: {}",
        min, low, mid, high, max
    );

    let mut values: Vec<usize> = labels.keys().map(|v| *v).collect();
    values.sort();
    let mut l = 0;
    for slice in heatmap {
        let mut x = 0;
        for bucket in slice.histogram() {
            let value = color_from_value(bucket.count() / bucket.width(), low, mid, high);
            buffer.set_pixel(x, y, value);
            x += 1;
        }
        y += 1;
    }

    if !values.is_empty() {
        let mut x = 0;
        let y = 0;
        let slice = heatmap.into_iter().next().unwrap();
        for bucket in slice.histogram() {
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
            x += 1;
        }
    }

    let mut y = 0;
    let mut begin = heatmap.begin_utc();
    for slice in heatmap {
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
        y += 1;
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
    let font_data = dejavu::sans_mono::regular();
    let collection = FontCollection::from_bytes(font_data as &[u8]).unwrap();
    let font = collection.into_font().unwrap();

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
fn color_from_value(value: usize, low: usize, mid: usize, high: usize) -> ColorRgb {
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
            h: 250.0 - (250.0 * (value - mid) as f64 / high as f64),
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

    ColorRgb { r: r, g: g, b: b }
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
            buffer: buffer,
            height: height,
            width: width,
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
            encoder.set(png::ColorType::RGB).set(png::BitDepth::Eight);
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
