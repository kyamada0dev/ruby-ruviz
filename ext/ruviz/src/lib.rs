// ruviz Ruby binding — native extension.
//
// Design (mirrors the ruviz Python binding): ruviz's `Plot` setters are
// *consuming* (`mut self -> Self`), which does not map onto a long-lived Ruby
// object. So the native handle keeps a plain, mutable `PlotState` and *replays*
// it onto a fresh `Plot::new()` at render time (snapshot-and-rebuild). The
// fluent chaining and keyword handling live in the Ruby facade (lib/ruviz);
// this layer stays thin: validate args, mutate state, build, render, map errors.

mod numo;

use std::cell::RefCell;

use magnus::{function, method, prelude::*, Error, Ruby, TryConvert, Value};

use ruviz::core::annotation::{ShapeStyle, TextStyle};
use ruviz::core::PlottingError;
use ruviz::prelude::{
    AxisScale, Color, HistogramConfig, IntoPlot, LegendPosition, LineStyle, MarkerStyle, Plot,
};
use ruviz::render::Theme;

const BINDING_VERSION: &str = env!("CARGO_PKG_VERSION");

// ---- error helpers ---------------------------------------------------------

/// Validation / bad-argument failures -> ArgumentError.
fn arg_err(msg: impl Into<String>) -> Error {
    Error::new(magnus::exception::arg_error(), msg.into())
}

/// ruviz render / IO failures -> RuntimeError (Ruviz::Error at the Ruby layer).
fn render_err(e: PlottingError) -> Error {
    Error::new(magnus::exception::runtime_error(), format!("ruviz: {e}"))
}

/// Extract numeric 1-D data into `Vec<f64>`.
///
/// Numo::NArray goes through the native-buffer fast path (no Ruby Array); Polars
/// Series are converted to Numo by the Ruby facade before they reach here, so
/// they take the same path. Anything else is treated as a Ruby Array.
fn extract_f64_vec(val: Value) -> Result<Vec<f64>, Error> {
    if numo::is_numo(val) {
        numo::to_f64_vec(val)
    } else {
        Vec::<f64>::try_convert(val)
    }
}

/// Extract 2-D numeric data into `Vec<Vec<f64>>` (Numo 2-D native buffer, else a
/// Ruby Array of Arrays).
fn extract_f64_matrix(val: Value) -> Result<Vec<Vec<f64>>, Error> {
    if numo::is_numo(val) {
        numo::to_f64_matrix(val)
    } else {
        Vec::<Vec<f64>>::try_convert(val)
    }
}

// ---- name -> enum parsing (tables mirror ruviz Python native_handle.rs) -----

fn parse_color(s: &str) -> Result<Color, Error> {
    Color::named(s)
        .or_else(|| Color::hex(s))
        .ok_or_else(|| arg_err(format!("unknown color: {s:?} (try a name like \"blue\" or \"#2563eb\")")))
}

fn parse_scale(name: &str, linthresh: Option<f64>) -> Result<AxisScale, Error> {
    match name.to_ascii_lowercase().as_str() {
        "linear" => Ok(AxisScale::Linear),
        "log" => Ok(AxisScale::Log),
        "symlog" => Ok(AxisScale::SymLog {
            linthresh: linthresh.unwrap_or(1.0),
        }),
        other => Err(arg_err(format!(
            "unknown scale: {other:?} (expected :linear, :log or :symlog)"
        ))),
    }
}

fn parse_legend(name: &str) -> Result<LegendPosition, Error> {
    let key = name.to_ascii_lowercase().replace('-', "_");
    let pos = match key.as_str() {
        "best" => LegendPosition::Best,
        "upper_right" => LegendPosition::UpperRight,
        "upper_left" => LegendPosition::UpperLeft,
        "lower_left" => LegendPosition::LowerLeft,
        "lower_right" => LegendPosition::LowerRight,
        "right" => LegendPosition::Right,
        "center_left" => LegendPosition::CenterLeft,
        "center_right" => LegendPosition::CenterRight,
        "lower_center" => LegendPosition::LowerCenter,
        "upper_center" => LegendPosition::UpperCenter,
        "center" => LegendPosition::Center,
        "outside_right" => LegendPosition::OutsideRight,
        "outside_left" => LegendPosition::OutsideLeft,
        "outside_upper" => LegendPosition::OutsideUpper,
        "outside_lower" => LegendPosition::OutsideLower,
        other => {
            return Err(arg_err(format!(
                "unknown legend position: {other:?} (e.g. :best, :upper_right, :outside_right)"
            )))
        }
    };
    Ok(pos)
}

fn parse_marker(name: &str) -> Result<MarkerStyle, Error> {
    // Accept both :triangle_down and "triangle-down".
    let key = name.to_ascii_lowercase().replace('_', "-");
    let m = match key.as_str() {
        "circle" => MarkerStyle::Circle,
        "square" => MarkerStyle::Square,
        "triangle" => MarkerStyle::Triangle,
        "triangle-down" => MarkerStyle::TriangleDown,
        "diamond" => MarkerStyle::Diamond,
        "plus" => MarkerStyle::Plus,
        "cross" => MarkerStyle::Cross,
        "star" => MarkerStyle::Star,
        "circle-open" => MarkerStyle::CircleOpen,
        "square-open" => MarkerStyle::SquareOpen,
        "triangle-open" => MarkerStyle::TriangleOpen,
        "diamond-open" => MarkerStyle::DiamondOpen,
        other => {
            return Err(arg_err(format!(
                "unknown marker: {other:?} (e.g. :circle, :square, :triangle_down, :diamond_open)"
            )))
        }
    };
    Ok(m)
}

fn parse_linestyle(s: &str) -> Result<LineStyle, Error> {
    let key = s.to_ascii_lowercase().replace('_', "-");
    let ls = match key.as_str() {
        "solid" => LineStyle::Solid,
        "dashed" => LineStyle::Dashed,
        "dotted" => LineStyle::Dotted,
        "dash-dot" => LineStyle::DashDot,
        "dash-dot-dot" => LineStyle::DashDotDot,
        other => {
            return Err(arg_err(format!(
                "unknown line style: {other:?} (e.g. :solid, :dashed, :dotted, :dash_dot)"
            )))
        }
    };
    Ok(ls)
}

fn parse_theme(s: &str) -> Result<Theme, Error> {
    let t = match s.to_ascii_lowercase().as_str() {
        "light" => Theme::light(),
        "dark" => Theme::dark(),
        "publication" => Theme::publication(),
        "minimal" => Theme::minimal(),
        "seaborn" => Theme::seaborn(),
        "presentation" => Theme::presentation(),
        other => {
            return Err(arg_err(format!(
                "unknown theme: {other:?} (light, dark, publication, minimal, seaborn, presentation)"
            )))
        }
    };
    Ok(t)
}

/// Optional color string -> parsed Color.
fn opt_color(color: Option<String>) -> Result<Option<Color>, Error> {
    color.as_deref().map(parse_color).transpose()
}

/// Build the optional style tuple for a reference line: `Some(...)` when any of
/// color/width/style is given (defaults fill the rest), else `None` (ruviz's
/// default dashed-gray line).
fn line_annotation_style(
    color: Option<String>,
    width: Option<f64>,
    style: Option<String>,
) -> Result<Option<(Color, f32, LineStyle)>, Error> {
    if color.is_none() && width.is_none() && style.is_none() {
        return Ok(None);
    }
    let c = match color {
        Some(s) => parse_color(&s)?,
        None => Color::from_rgb(128, 128, 128),
    };
    let w = width.map(|w| w as f32).unwrap_or(1.0);
    let ls = match style {
        Some(s) => parse_linestyle(&s)?,
        None => LineStyle::Dashed,
    };
    Ok(Some((c, w, ls)))
}

// ---- captured state --------------------------------------------------------

enum Series {
    Line {
        x: Vec<f64>,
        y: Vec<f64>,
        label: Option<String>,
        color: Option<Color>,
        width: Option<f32>,
    },
    Scatter {
        x: Vec<f64>,
        y: Vec<f64>,
        label: Option<String>,
        color: Option<Color>,
        marker: Option<MarkerStyle>,
        marker_size: Option<f32>,
        alpha: Option<f32>,
    },
    Bar {
        categories: Vec<String>,
        values: Vec<f64>,
        label: Option<String>,
        color: Option<Color>,
        alpha: Option<f32>,
    },
    Histogram {
        data: Vec<f64>,
        bins: Option<usize>,
        label: Option<String>,
        color: Option<Color>,
        alpha: Option<f32>,
    },
    Area {
        x: Vec<f64>,
        y: Vec<f64>,
        baseline: f64,
        label: Option<String>,
        color: Option<Color>,
        width: Option<f32>,
        alpha: Option<f32>,
    },
    BoxPlot {
        data: Vec<f64>,
        label: Option<String>,
        color: Option<Color>,
        alpha: Option<f32>,
    },
    // kde / ecdf / violin all take a 1-D sample and the same generic styling.
    Dist {
        kind: DistKind,
        data: Vec<f64>,
        label: Option<String>,
        color: Option<Color>,
        alpha: Option<f32>,
    },
    Heatmap {
        matrix: Vec<Vec<f64>>,
    },
    Contour {
        x: Vec<f64>,
        y: Vec<f64>,
        z: Vec<f64>,
        levels: Option<usize>,
        filled: Option<bool>,
    },
}

#[derive(Clone, Copy)]
enum DistKind {
    Kde,
    Ecdf,
    Violin,
}

enum Annotation {
    HLine {
        y: f64,
        style: Option<(Color, f32, LineStyle)>,
    },
    VLine {
        x: f64,
        style: Option<(Color, f32, LineStyle)>,
    },
    Text {
        x: f64,
        y: f64,
        text: String,
        color: Option<Color>,
        size: Option<f32>,
    },
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: Option<Color>,
        line_width: Option<f32>,
    },
}

#[derive(Default)]
struct PlotState {
    width_px: Option<u32>,
    height_px: Option<u32>,
    title: Option<String>,
    xlabel: Option<String>,
    ylabel: Option<String>,
    xscale: Option<AxisScale>,
    yscale: Option<AxisScale>,
    xlim: Option<(f64, f64)>,
    ylim: Option<(f64, f64)>,
    grid: Option<bool>,
    legend: Option<LegendPosition>,
    theme: Option<Theme>,
    series: Vec<Series>,
    annotations: Vec<Annotation>,
}

#[magnus::wrap(class = "Ruviz::PlotHandle", free_immediately, size)]
struct PlotHandle(RefCell<PlotState>);

impl PlotHandle {
    fn new() -> Self {
        PlotHandle(RefCell::new(PlotState::default()))
    }

    fn size_px(&self, width: u32, height: u32) -> Result<(), Error> {
        if width == 0 || height == 0 {
            return Err(arg_err("size_px: width and height must be positive"));
        }
        let mut st = self.0.borrow_mut();
        st.width_px = Some(width);
        st.height_px = Some(height);
        Ok(())
    }

    fn title(&self, s: String) {
        self.0.borrow_mut().title = Some(s);
    }

    fn xlabel(&self, s: String) {
        self.0.borrow_mut().xlabel = Some(s);
    }

    fn ylabel(&self, s: String) {
        self.0.borrow_mut().ylabel = Some(s);
    }

    fn xscale(&self, name: String, linthresh: Option<f64>) -> Result<(), Error> {
        let scale = parse_scale(&name, linthresh)?;
        self.0.borrow_mut().xscale = Some(scale);
        Ok(())
    }

    fn yscale(&self, name: String, linthresh: Option<f64>) -> Result<(), Error> {
        let scale = parse_scale(&name, linthresh)?;
        self.0.borrow_mut().yscale = Some(scale);
        Ok(())
    }

    fn grid(&self, enabled: bool) {
        self.0.borrow_mut().grid = Some(enabled);
    }

    fn legend(&self, position: String) -> Result<(), Error> {
        let pos = parse_legend(&position)?;
        self.0.borrow_mut().legend = Some(pos);
        Ok(())
    }

    fn theme(&self, name: String) -> Result<(), Error> {
        let theme = parse_theme(&name)?;
        self.0.borrow_mut().theme = Some(theme);
        Ok(())
    }

    fn xlim(&self, min: f64, max: f64) -> Result<(), Error> {
        if !(min < max) {
            return Err(arg_err("xlim: min must be less than max"));
        }
        self.0.borrow_mut().xlim = Some((min, max));
        Ok(())
    }

    fn ylim(&self, min: f64, max: f64) -> Result<(), Error> {
        if !(min < max) {
            return Err(arg_err("ylim: min must be less than max"));
        }
        self.0.borrow_mut().ylim = Some((min, max));
        Ok(())
    }

    fn hline(
        &self,
        y: f64,
        color: Option<String>,
        width: Option<f64>,
        style: Option<String>,
    ) -> Result<(), Error> {
        let style = line_annotation_style(color, width, style)?;
        self.0.borrow_mut().annotations.push(Annotation::HLine { y, style });
        Ok(())
    }

    fn vline(
        &self,
        x: f64,
        color: Option<String>,
        width: Option<f64>,
        style: Option<String>,
    ) -> Result<(), Error> {
        let style = line_annotation_style(color, width, style)?;
        self.0.borrow_mut().annotations.push(Annotation::VLine { x, style });
        Ok(())
    }

    fn annotate_text(
        &self,
        x: f64,
        y: f64,
        text: String,
        color: Option<String>,
        size: Option<f64>,
    ) -> Result<(), Error> {
        let color = opt_color(color)?;
        self.0.borrow_mut().annotations.push(Annotation::Text {
            x,
            y,
            text,
            color,
            size: size.map(|s| s as f32),
        });
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn rect(
        &self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: Option<String>,
        line_width: Option<f64>,
    ) -> Result<(), Error> {
        let color = opt_color(color)?;
        self.0.borrow_mut().annotations.push(Annotation::Rect {
            x,
            y,
            width,
            height,
            color,
            line_width: line_width.map(|w| w as f32),
        });
        Ok(())
    }

    fn line(
        &self,
        x: Value,
        y: Value,
        label: Option<String>,
        color: Option<String>,
        width: Option<f64>,
    ) -> Result<(), Error> {
        let x = extract_f64_vec(x)?;
        let y = extract_f64_vec(y)?;
        if x.len() != y.len() {
            return Err(arg_err(format!(
                "line: x and y must have the same length (got {} and {})",
                x.len(),
                y.len()
            )));
        }
        if x.is_empty() {
            return Err(arg_err("line: data is empty"));
        }
        let color = opt_color(color)?;
        self.0.borrow_mut().series.push(Series::Line {
            x,
            y,
            label,
            color,
            width: width.map(|w| w as f32),
        });
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn scatter(
        &self,
        x: Value,
        y: Value,
        label: Option<String>,
        color: Option<String>,
        marker: Option<String>,
        marker_size: Option<f64>,
        alpha: Option<f64>,
    ) -> Result<(), Error> {
        let x = extract_f64_vec(x)?;
        let y = extract_f64_vec(y)?;
        if x.len() != y.len() {
            return Err(arg_err(format!(
                "scatter: x and y must have the same length (got {} and {})",
                x.len(),
                y.len()
            )));
        }
        if x.is_empty() {
            return Err(arg_err("scatter: data is empty"));
        }
        let color = opt_color(color)?;
        let marker = marker.as_deref().map(parse_marker).transpose()?;
        self.0.borrow_mut().series.push(Series::Scatter {
            x,
            y,
            label,
            color,
            marker,
            marker_size: marker_size.map(|s| s as f32),
            alpha: alpha.map(|a| a as f32),
        });
        Ok(())
    }

    fn bar(
        &self,
        categories: Vec<String>,
        values: Value,
        label: Option<String>,
        color: Option<String>,
        alpha: Option<f64>,
    ) -> Result<(), Error> {
        let values = extract_f64_vec(values)?;
        if categories.len() != values.len() {
            return Err(arg_err(format!(
                "bar: categories and values must have the same length (got {} and {})",
                categories.len(),
                values.len()
            )));
        }
        if categories.is_empty() {
            return Err(arg_err("bar: data is empty"));
        }
        let color = opt_color(color)?;
        self.0.borrow_mut().series.push(Series::Bar {
            categories,
            values,
            label,
            color,
            alpha: alpha.map(|a| a as f32),
        });
        Ok(())
    }

    fn histogram(
        &self,
        data: Value,
        bins: Option<usize>,
        label: Option<String>,
        color: Option<String>,
        alpha: Option<f64>,
    ) -> Result<(), Error> {
        let data = extract_f64_vec(data)?;
        if data.is_empty() {
            return Err(arg_err("histogram: data is empty"));
        }
        if bins == Some(0) {
            return Err(arg_err("histogram: bins must be positive"));
        }
        let color = opt_color(color)?;
        self.0.borrow_mut().series.push(Series::Histogram {
            data,
            bins,
            label,
            color,
            alpha: alpha.map(|a| a as f32),
        });
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn area(
        &self,
        x: Value,
        y: Value,
        baseline: f64,
        label: Option<String>,
        color: Option<String>,
        width: Option<f64>,
        alpha: Option<f64>,
    ) -> Result<(), Error> {
        let x = extract_f64_vec(x)?;
        let y = extract_f64_vec(y)?;
        if x.len() != y.len() {
            return Err(arg_err(format!(
                "area: x and y must have the same length (got {} and {})",
                x.len(),
                y.len()
            )));
        }
        if x.is_empty() {
            return Err(arg_err("area: data is empty"));
        }
        let color = opt_color(color)?;
        self.0.borrow_mut().series.push(Series::Area {
            x,
            y,
            baseline,
            label,
            color,
            width: width.map(|w| w as f32),
            alpha: alpha.map(|a| a as f32),
        });
        Ok(())
    }

    fn boxplot(
        &self,
        data: Value,
        label: Option<String>,
        color: Option<String>,
        alpha: Option<f64>,
    ) -> Result<(), Error> {
        let data = extract_f64_vec(data)?;
        if data.is_empty() {
            return Err(arg_err("boxplot: data is empty"));
        }
        let color = opt_color(color)?;
        self.0.borrow_mut().series.push(Series::BoxPlot {
            data,
            label,
            color,
            alpha: alpha.map(|a| a as f32),
        });
        Ok(())
    }

    fn push_dist(
        &self,
        kind: DistKind,
        who: &str,
        data: Value,
        label: Option<String>,
        color: Option<String>,
        alpha: Option<f64>,
    ) -> Result<(), Error> {
        let data = extract_f64_vec(data)?;
        if data.is_empty() {
            return Err(arg_err(format!("{who}: data is empty")));
        }
        let color = opt_color(color)?;
        self.0.borrow_mut().series.push(Series::Dist {
            kind,
            data,
            label,
            color,
            alpha: alpha.map(|a| a as f32),
        });
        Ok(())
    }

    fn kde(
        &self,
        data: Value,
        label: Option<String>,
        color: Option<String>,
        alpha: Option<f64>,
    ) -> Result<(), Error> {
        self.push_dist(DistKind::Kde, "kde", data, label, color, alpha)
    }

    fn ecdf(
        &self,
        data: Value,
        label: Option<String>,
        color: Option<String>,
        alpha: Option<f64>,
    ) -> Result<(), Error> {
        self.push_dist(DistKind::Ecdf, "ecdf", data, label, color, alpha)
    }

    fn violin(
        &self,
        data: Value,
        label: Option<String>,
        color: Option<String>,
        alpha: Option<f64>,
    ) -> Result<(), Error> {
        self.push_dist(DistKind::Violin, "violin", data, label, color, alpha)
    }

    fn heatmap(&self, data: Value) -> Result<(), Error> {
        let matrix = extract_f64_matrix(data)?;
        if matrix.is_empty() || matrix[0].is_empty() {
            return Err(arg_err("heatmap: data is empty"));
        }
        let cols = matrix[0].len();
        if matrix.iter().any(|r| r.len() != cols) {
            return Err(arg_err("heatmap: all rows must have the same length"));
        }
        self.0.borrow_mut().series.push(Series::Heatmap { matrix });
        Ok(())
    }

    fn contour(
        &self,
        x: Value,
        y: Value,
        z: Value,
        levels: Option<usize>,
        filled: Option<bool>,
    ) -> Result<(), Error> {
        let x = extract_f64_vec(x)?;
        let y = extract_f64_vec(y)?;
        let z = extract_f64_vec(z)?;
        if x.is_empty() || y.is_empty() {
            return Err(arg_err("contour: x and y must be non-empty"));
        }
        if z.len() != x.len() * y.len() {
            return Err(arg_err(format!(
                "contour: z length ({}) must equal x.len()*y.len() ({}*{}={})",
                z.len(),
                x.len(),
                y.len(),
                x.len() * y.len()
            )));
        }
        if levels == Some(0) {
            return Err(arg_err("contour: levels must be positive"));
        }
        self.0.borrow_mut().series.push(Series::Contour {
            x,
            y,
            z,
            levels,
            filled,
        });
        Ok(())
    }

    /// Replay the captured state onto a fresh ruviz `Plot`.
    fn build_plot(&self) -> Plot {
        let st = self.0.borrow();
        let mut plot = Plot::new();
        if let (Some(w), Some(h)) = (st.width_px, st.height_px) {
            plot = plot.size_px(w, h);
        }
        if let Some(theme) = &st.theme {
            plot = plot.theme(theme.clone());
        }
        if let Some(t) = &st.title {
            plot = plot.title(t.as_str());
        }
        if let Some(s) = &st.xlabel {
            plot = plot.xlabel(s.as_str());
        }
        if let Some(s) = &st.ylabel {
            plot = plot.ylabel(s.as_str());
        }
        if let Some(scale) = st.xscale {
            plot = plot.xscale(scale);
        }
        if let Some(scale) = st.yscale {
            plot = plot.yscale(scale);
        }
        if let Some((min, max)) = st.xlim {
            plot = plot.xlim(min, max);
        }
        if let Some((min, max)) = st.ylim {
            plot = plot.ylim(min, max);
        }
        if let Some(g) = st.grid {
            plot = plot.grid(g);
        }
        if let Some(pos) = st.legend {
            plot = plot.legend(pos);
        }
        for s in &st.series {
            plot = match s {
                Series::Line {
                    x,
                    y,
                    label,
                    color,
                    width,
                } => {
                    let mut pb = plot.line_source(x.clone(), y.clone());
                    if let Some(l) = label {
                        pb = pb.label(l.clone());
                    }
                    if let Some(c) = color {
                        pb = pb.color(*c);
                    }
                    if let Some(w) = width {
                        pb = pb.line_width(*w);
                    }
                    pb.into_plot()
                }
                Series::Scatter {
                    x,
                    y,
                    label,
                    color,
                    marker,
                    marker_size,
                    alpha,
                } => {
                    let mut pb = plot.scatter(x, y);
                    if let Some(l) = label {
                        pb = pb.label(l.clone());
                    }
                    if let Some(c) = color {
                        pb = pb.color(*c);
                    }
                    if let Some(m) = marker {
                        pb = pb.marker(*m);
                    }
                    if let Some(ms) = marker_size {
                        pb = pb.marker_size(*ms);
                    }
                    if let Some(a) = alpha {
                        pb = pb.alpha(*a);
                    }
                    pb.into_plot()
                }
                Series::Bar {
                    categories,
                    values,
                    label,
                    color,
                    alpha,
                } => {
                    let mut pb = plot.bar(categories, values);
                    if let Some(l) = label {
                        pb = pb.label(l.clone());
                    }
                    if let Some(c) = color {
                        pb = pb.color(*c);
                    }
                    if let Some(a) = alpha {
                        pb = pb.alpha(*a);
                    }
                    pb.into_plot()
                }
                Series::Histogram {
                    data,
                    bins,
                    label,
                    color,
                    alpha,
                } => {
                    let mut pb = match bins {
                        Some(n) => plot.histogram_with(
                            data,
                            HistogramConfig {
                                bins: Some(*n),
                                ..HistogramConfig::default()
                            },
                        ),
                        None => plot.histogram(data),
                    };
                    if let Some(l) = label {
                        pb = pb.label(l.clone());
                    }
                    if let Some(c) = color {
                        pb = pb.color(*c);
                    }
                    if let Some(a) = alpha {
                        pb = pb.alpha(*a);
                    }
                    pb.into_plot()
                }
                Series::Area {
                    x,
                    y,
                    baseline,
                    label,
                    color,
                    width,
                    alpha,
                } => {
                    let mut pb = plot.area(x, y, *baseline);
                    if let Some(l) = label {
                        pb = pb.label(l.clone());
                    }
                    if let Some(c) = color {
                        pb = pb.color(*c);
                    }
                    if let Some(w) = width {
                        pb = pb.line_width(*w);
                    }
                    if let Some(a) = alpha {
                        pb = pb.alpha(*a);
                    }
                    pb.into_plot()
                }
                Series::BoxPlot {
                    data,
                    label,
                    color,
                    alpha,
                } => {
                    let mut pb = plot.boxplot(data);
                    if let Some(l) = label {
                        pb = pb.label(l.clone());
                    }
                    if let Some(c) = color {
                        pb = pb.color(*c);
                    }
                    if let Some(a) = alpha {
                        pb = pb.alpha(*a);
                    }
                    pb.into_plot()
                }
                Series::Dist {
                    kind,
                    data,
                    label,
                    color,
                    alpha,
                } => {
                    // kde/ecdf/violin return different builder types, so apply the
                    // shared styling + finalize inside each arm via a local macro.
                    macro_rules! styled {
                        ($pb:expr) => {{
                            let mut pb = $pb;
                            if let Some(l) = label {
                                pb = pb.label(l.clone());
                            }
                            if let Some(c) = color {
                                pb = pb.color(*c);
                            }
                            if let Some(a) = alpha {
                                pb = pb.alpha(*a);
                            }
                            pb.into_plot()
                        }};
                    }
                    match kind {
                        DistKind::Kde => styled!(plot.kde(data)),
                        DistKind::Ecdf => styled!(plot.ecdf(data)),
                        DistKind::Violin => styled!(plot.violin(data)),
                    }
                }
                Series::Heatmap { matrix } => plot.heatmap(matrix).into_plot(),
                Series::Contour {
                    x,
                    y,
                    z,
                    levels,
                    filled,
                } => {
                    let mut pb = plot.contour(x, y, z);
                    if let Some(n) = levels {
                        pb = pb.levels(*n);
                    }
                    if let Some(f) = filled {
                        pb = pb.filled(*f);
                    }
                    pb.into_plot()
                }
            };
        }
        for a in &st.annotations {
            plot = match a {
                Annotation::HLine { y, style } => match style {
                    Some((c, w, ls)) => plot.hline_styled(*y, *c, *w, ls.clone()),
                    None => plot.hline(*y),
                },
                Annotation::VLine { x, style } => match style {
                    Some((c, w, ls)) => plot.vline_styled(*x, *c, *w, ls.clone()),
                    None => plot.vline(*x),
                },
                Annotation::Text {
                    x,
                    y,
                    text,
                    color,
                    size,
                } => {
                    if color.is_none() && size.is_none() {
                        plot.text(*x, *y, text.clone())
                    } else {
                        let mut ts = TextStyle::new();
                        if let Some(c) = color {
                            ts = ts.color(*c);
                        }
                        if let Some(s) = size {
                            ts = ts.font_size(*s);
                        }
                        plot.text_styled(*x, *y, text.clone(), ts)
                    }
                }
                Annotation::Rect {
                    x,
                    y,
                    width,
                    height,
                    color,
                    line_width,
                } => {
                    if color.is_none() && line_width.is_none() {
                        plot.rect(*x, *y, *width, *height)
                    } else {
                        let mut ss = ShapeStyle::new();
                        if let Some(c) = color {
                            ss = ss.fill(*c);
                        }
                        if let Some(w) = line_width {
                            ss = ss.edge_width(*w);
                        }
                        plot.rect_styled(*x, *y, *width, *height, ss)
                    }
                }
            };
        }
        plot
    }

    /// Render and write to `path`, dispatching by extension (png/svg/pdf).
    fn save(&self, path: String) -> Result<(), Error> {
        if self.0.borrow().series.is_empty() {
            return Err(arg_err("save: nothing to plot (add a series, e.g. #line)"));
        }
        let plot = self.build_plot();
        let lower = path.to_ascii_lowercase();
        let result = if lower.ends_with(".svg") {
            plot.export_svg(&path)
        } else if lower.ends_with(".pdf") {
            plot.save_pdf(&path)
        } else {
            plot.save(&path) // PNG (default)
        };
        result.map_err(render_err)
    }
}

fn hello() -> String {
    format!("ruviz-ruby native extension loaded (v{BINDING_VERSION})")
}

#[magnus::init(name = "ruviz")]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("Ruviz")?;
    module.define_singleton_method("_hello", function!(hello, 0))?;

    let handle = module.define_class("PlotHandle", ruby.class_object())?;
    handle.define_singleton_method("new", function!(PlotHandle::new, 0))?;
    handle.define_method("size_px", method!(PlotHandle::size_px, 2))?;
    handle.define_method("title", method!(PlotHandle::title, 1))?;
    handle.define_method("xlabel", method!(PlotHandle::xlabel, 1))?;
    handle.define_method("ylabel", method!(PlotHandle::ylabel, 1))?;
    handle.define_method("xscale", method!(PlotHandle::xscale, 2))?;
    handle.define_method("yscale", method!(PlotHandle::yscale, 2))?;
    handle.define_method("grid", method!(PlotHandle::grid, 1))?;
    handle.define_method("legend", method!(PlotHandle::legend, 1))?;
    handle.define_method("theme", method!(PlotHandle::theme, 1))?;
    handle.define_method("xlim", method!(PlotHandle::xlim, 2))?;
    handle.define_method("ylim", method!(PlotHandle::ylim, 2))?;
    handle.define_method("hline", method!(PlotHandle::hline, 4))?;
    handle.define_method("vline", method!(PlotHandle::vline, 4))?;
    handle.define_method("annotate_text", method!(PlotHandle::annotate_text, 5))?;
    handle.define_method("rect", method!(PlotHandle::rect, 6))?;
    handle.define_method("line", method!(PlotHandle::line, 5))?;
    handle.define_method("scatter", method!(PlotHandle::scatter, 7))?;
    handle.define_method("bar", method!(PlotHandle::bar, 5))?;
    handle.define_method("histogram", method!(PlotHandle::histogram, 5))?;
    handle.define_method("area", method!(PlotHandle::area, 7))?;
    handle.define_method("boxplot", method!(PlotHandle::boxplot, 4))?;
    handle.define_method("kde", method!(PlotHandle::kde, 4))?;
    handle.define_method("ecdf", method!(PlotHandle::ecdf, 4))?;
    handle.define_method("violin", method!(PlotHandle::violin, 4))?;
    handle.define_method("heatmap", method!(PlotHandle::heatmap, 1))?;
    handle.define_method("contour", method!(PlotHandle::contour, 5))?;
    handle.define_method("save", method!(PlotHandle::save, 1))?;

    Ok(())
}
