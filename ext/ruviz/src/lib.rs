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
use magnus::typed_data::Obj;

use ruviz::core::annotation::{ShapeStyle, TextStyle};
use ruviz::core::PlottingError;
use ruviz::prelude::{
    line3d, scatter3d, subplots, surface, wireframe, AxisScale, Color, HistogramConfig, IntoPlot,
    LegendPosition, LineStyle, MarkerStyle, Plot, SubplotFigure,
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
        style: Option<LineStyle>,
    },
    ErrorBars {
        x: Vec<f64>,
        y: Vec<f64>,
        x_err: Option<Vec<f64>>,
        y_err: Option<Vec<f64>>,
        label: Option<String>,
        color: Option<Color>,
    },
    PolarLine {
        r: Vec<f64>,
        theta: Vec<f64>,
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
        colormap: Option<String>,
        colorbar: bool,
        colorbar_label: Option<String>,
    },
    Contour {
        x: Vec<f64>,
        y: Vec<f64>,
        z: Vec<f64>,
        levels: Option<usize>,
        filled: Option<bool>,
    },
    Pie {
        values: Vec<f64>,
        labels: Option<Vec<String>>,
        donut: Option<f64>,
    },
    Radar {
        labels: Vec<String>,
        series: Vec<(Option<String>, Vec<f64>)>,
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
    dpi: Option<u32>,
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
    font_family: Option<String>,
    font_size: Option<f32>,
    title_size: Option<f32>,
    legend_font_size: Option<f32>,
    scale_typography: Option<f32>,
    fast: Option<bool>,
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

    // Output resolution. Keeps layout proportions (matplotlib semantics) and
    // scales the rendered pixels, so higher DPI yields crisper text/lines.
    fn dpi(&self, dpi: u32) -> Result<(), Error> {
        if dpi == 0 {
            return Err(arg_err("dpi: must be positive"));
        }
        self.0.borrow_mut().dpi = Some(dpi);
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

    fn font_family(&self, name: String) {
        self.0.borrow_mut().font_family = Some(name);
    }

    fn font_size(&self, size: f64) -> Result<(), Error> {
        if !(size > 0.0) {
            return Err(arg_err("font_size: must be positive"));
        }
        self.0.borrow_mut().font_size = Some(size as f32);
        Ok(())
    }

    fn title_size(&self, size: f64) -> Result<(), Error> {
        if !(size > 0.0) {
            return Err(arg_err("title_size: must be positive"));
        }
        self.0.borrow_mut().title_size = Some(size as f32);
        Ok(())
    }

    fn legend_font_size(&self, size: f64) -> Result<(), Error> {
        if !(size > 0.0) {
            return Err(arg_err("legend_font_size: must be positive"));
        }
        self.0.borrow_mut().legend_font_size = Some(size as f32);
        Ok(())
    }

    fn scale_typography(&self, factor: f64) -> Result<(), Error> {
        if !(factor > 0.0) {
            return Err(arg_err("scale_typography: factor must be positive"));
        }
        self.0.borrow_mut().scale_typography = Some(factor as f32);
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

    #[allow(clippy::too_many_arguments)]
    fn line(
        &self,
        x: Value,
        y: Value,
        label: Option<String>,
        color: Option<String>,
        width: Option<f64>,
        style: Option<String>,
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
        let style = style.map(|s| parse_linestyle(&s)).transpose()?;
        self.0.borrow_mut().series.push(Series::Line {
            x,
            y,
            label,
            color,
            width: width.map(|w| w as f32),
            style,
        });
        Ok(())
    }

    fn error_bars(
        &self,
        x: Value,
        y: Value,
        y_err: Value,
        x_err: Option<Value>,
        label: Option<String>,
        color: Option<String>,
    ) -> Result<(), Error> {
        let x = extract_f64_vec(x)?;
        let y = extract_f64_vec(y)?;
        let y_err = extract_f64_vec(y_err)?;
        let x_err = x_err.map(extract_f64_vec).transpose()?;
        if x.len() != y.len() || x.len() != y_err.len() {
            return Err(arg_err("error_bars: x, y, y_err must have the same length"));
        }
        if let Some(xe) = &x_err {
            if xe.len() != x.len() {
                return Err(arg_err("error_bars: x_err must match x length"));
            }
        }
        self.0.borrow_mut().series.push(Series::ErrorBars {
            x,
            y,
            x_err,
            y_err: Some(y_err),
            label,
            color: opt_color(color)?,
        });
        Ok(())
    }

    fn polar_line(
        &self,
        theta: Value,
        r: Value,
        label: Option<String>,
        color: Option<String>,
        width: Option<f64>,
    ) -> Result<(), Error> {
        let theta = extract_f64_vec(theta)?;
        let r = extract_f64_vec(r)?;
        if theta.len() != r.len() {
            return Err(arg_err("polar_line: theta and r must have the same length"));
        }
        if theta.is_empty() {
            return Err(arg_err("polar_line: data is empty"));
        }
        self.0.borrow_mut().series.push(Series::PolarLine {
            r,
            theta,
            label,
            color: opt_color(color)?,
            width: width.map(|w| w as f32),
        });
        Ok(())
    }

    fn fast(&self, enabled: bool) -> Result<(), Error> {
        self.0.borrow_mut().fast = Some(enabled);
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

    fn heatmap(
        &self,
        data: Value,
        colormap: Option<String>,
        colorbar: bool,
        colorbar_label: Option<String>,
    ) -> Result<(), Error> {
        let matrix = extract_f64_matrix(data)?;
        if matrix.is_empty() || matrix[0].is_empty() {
            return Err(arg_err("heatmap: data is empty"));
        }
        let cols = matrix[0].len();
        if matrix.iter().any(|r| r.len() != cols) {
            return Err(arg_err("heatmap: all rows must have the same length"));
        }
        self.0.borrow_mut().series.push(Series::Heatmap {
            matrix,
            colormap,
            colorbar,
            colorbar_label,
        });
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

    fn pie(
        &self,
        values: Value,
        labels: Option<Vec<String>>,
        donut: Option<f64>,
    ) -> Result<(), Error> {
        let values = extract_f64_vec(values)?;
        if values.is_empty() {
            return Err(arg_err("pie: data is empty"));
        }
        if let Some(l) = &labels {
            if l.len() != values.len() {
                return Err(arg_err(format!(
                    "pie: labels ({}) must match values ({})",
                    l.len(),
                    values.len()
                )));
            }
        }
        if let Some(d) = donut {
            if !(0.0..1.0).contains(&d) {
                return Err(arg_err("pie: donut ratio must be in [0, 1)"));
            }
        }
        self.0.borrow_mut().series.push(Series::Pie {
            values,
            labels,
            donut,
        });
        Ok(())
    }

    fn radar(
        &self,
        labels: Vec<String>,
        names: Vec<Option<String>>,
        values_list: Vec<Vec<f64>>,
    ) -> Result<(), Error> {
        if labels.is_empty() {
            return Err(arg_err("radar: labels are empty"));
        }
        if names.len() != values_list.len() {
            return Err(arg_err("radar: names and series count mismatch"));
        }
        if values_list.is_empty() {
            return Err(arg_err("radar: at least one series is required"));
        }
        for (i, vals) in values_list.iter().enumerate() {
            if vals.len() != labels.len() {
                return Err(arg_err(format!(
                    "radar: series {} has {} values but there are {} labels",
                    i,
                    vals.len(),
                    labels.len()
                )));
            }
        }
        let series = names.into_iter().zip(values_list).collect();
        self.0.borrow_mut().series.push(Series::Radar { labels, series });
        Ok(())
    }

    /// Replay the captured state onto a fresh ruviz `Plot`.
    fn build_plot(&self) -> Plot {
        let st = self.0.borrow();
        let mut plot = Plot::new();
        if let (Some(w), Some(h)) = (st.width_px, st.height_px) {
            plot = plot.size_px(w, h);
        }
        if let Some(d) = st.dpi {
            plot = plot.dpi(d);
        }
        if let Some(f) = st.fast {
            plot = plot.fast(f);
        }
        if let Some(theme) = &st.theme {
            plot = plot.theme(theme.clone());
        }
        if let Some(f) = &st.font_family {
            plot = plot.font_family(f.as_str());
        }
        if let Some(s) = st.font_size {
            plot = plot.font_size(s);
        }
        if let Some(f) = st.scale_typography {
            plot = plot.scale_typography(f);
        }
        if let Some(s) = st.title_size {
            plot = plot.title_size(s);
        }
        if let Some(s) = st.legend_font_size {
            plot = plot.legend_font_size(s);
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
                    style,
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
                    if let Some(s) = style {
                        pb = pb.line_style(s.clone());
                    }
                    pb.into_plot()
                }
                Series::ErrorBars {
                    x,
                    y,
                    x_err,
                    y_err,
                    label,
                    color,
                } => {
                    let yerr = y_err.clone().unwrap_or_else(|| vec![0.0; x.len()]);
                    let mut pb = match x_err {
                        Some(xe) => plot.error_bars_xy(x, y, xe, &yerr),
                        None => plot.error_bars(x, y, &yerr),
                    };
                    if let Some(l) = label {
                        pb = pb.label(l.clone());
                    }
                    if let Some(c) = color {
                        pb = pb.color(*c);
                    }
                    pb.into_plot()
                }
                Series::PolarLine {
                    r,
                    theta,
                    label,
                    color,
                    width,
                } => {
                    let mut pb = plot.polar_line(r, theta);
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
                Series::Heatmap {
                    matrix,
                    colormap,
                    colorbar,
                    colorbar_label,
                } => {
                    let mut cfg = ruviz::plots::heatmap::HeatmapConfig::new();
                    if let Some(name) = colormap {
                        cfg = cfg.cmap(name.clone());
                    }
                    let mut pb = plot.heatmap_with(matrix, cfg);
                    if *colorbar {
                        pb = pb.colorbar(true);
                    }
                    if let Some(lbl) = colorbar_label {
                        pb = pb.colorbar_label(lbl.clone());
                    }
                    pb.into_plot()
                }
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
                Series::Pie {
                    values,
                    labels,
                    donut,
                } => {
                    let mut pb = plot.pie(values);
                    if let Some(l) = labels {
                        pb = pb.labels(l);
                    }
                    if let Some(d) = donut {
                        pb = pb.donut(*d);
                    }
                    pb.into_plot()
                }
                Series::Radar { labels, series } => {
                    let mut pb = plot.radar(labels);
                    for (name, vals) in series {
                        pb = match name {
                            Some(n) => pb.add_series(n.clone(), vals),
                            None => pb.series(vals),
                        };
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

// ---- Subplots (grid of plots in one figure) -------------------------------

#[derive(Default)]
struct SubplotState {
    rows: usize,
    cols: usize,
    width: u32,
    height: u32,
    suptitle: Option<String>,
    suptitle_size: Option<f32>,
    // (grid index, already-built plot). Plots are built eagerly when added so
    // the Ruby-side Plot object stays usable afterwards.
    plots: Vec<(usize, Plot)>,
}

#[magnus::wrap(class = "Ruviz::SubplotHandle", free_immediately, size)]
struct SubplotHandle(RefCell<SubplotState>);

impl SubplotHandle {
    fn new(rows: usize, cols: usize, width: u32, height: u32) -> Result<Self, Error> {
        if rows == 0 || cols == 0 {
            return Err(arg_err("subplots: rows and cols must be positive"));
        }
        if width == 0 || height == 0 {
            return Err(arg_err("subplots: width and height must be positive"));
        }
        Ok(SubplotHandle(RefCell::new(SubplotState {
            rows,
            cols,
            width,
            height,
            ..Default::default()
        })))
    }

    fn suptitle(&self, text: String) -> Result<(), Error> {
        self.0.borrow_mut().suptitle = Some(text);
        Ok(())
    }

    fn suptitle_font_size(&self, size: f64) -> Result<(), Error> {
        self.0.borrow_mut().suptitle_size = Some(size as f32);
        Ok(())
    }

    fn subplot_at(&self, index: usize, plot: Obj<PlotHandle>) -> Result<(), Error> {
        let built = plot.build_plot();
        self.0.borrow_mut().plots.push((index, built));
        Ok(())
    }

    fn subplot(&self, row: usize, col: usize, plot: Obj<PlotHandle>) -> Result<(), Error> {
        let index = {
            let st = self.0.borrow();
            if row >= st.rows || col >= st.cols {
                return Err(arg_err("subplot: (row, col) out of grid range"));
            }
            row * st.cols + col
        };
        let built = plot.build_plot();
        self.0.borrow_mut().plots.push((index, built));
        Ok(())
    }

    fn save(&self, path: String) -> Result<(), Error> {
        let mut st = self.0.borrow_mut();
        if st.plots.is_empty() {
            return Err(arg_err("save: no subplots added (use #subplot or #subplot_at)"));
        }
        let mut fig: SubplotFigure =
            subplots(st.rows, st.cols, st.width, st.height).map_err(render_err)?;
        if let Some(t) = st.suptitle.take() {
            fig = fig.suptitle(t);
        }
        if let Some(s) = st.suptitle_size {
            fig = fig.suptitle_font_size(s);
        }
        for (index, plot) in std::mem::take(&mut st.plots) {
            fig = fig.subplot_at(index, plot).map_err(render_err)?;
        }
        // SubplotFigure only renders raster output (PNG).
        fig.save(&path).map_err(render_err)
    }
}

// ---- 3D plots (scatter3d / line3d / surface / wireframe) ------------------

#[derive(Clone, Copy, PartialEq)]
enum Plot3DKind {
    Scatter,
    Line,
    Surface,
    Wireframe,
}

#[derive(Default)]
struct Plot3DState {
    x: Vec<f64>,
    y: Vec<f64>,
    z1: Vec<f64>,           // scatter3d / line3d
    z2: Vec<Vec<f64>>,      // surface / wireframe
    title: Option<String>,
    xlabel: Option<String>,
    ylabel: Option<String>,
    zlabel: Option<String>,
    color: Option<Color>,
    marker: Option<MarkerStyle>,
    marker_size: Option<f32>,
    line_width: Option<f32>,
}

#[magnus::wrap(class = "Ruviz::Plot3DHandle", free_immediately, size)]
struct Plot3DHandle {
    kind: Plot3DKind,
    state: RefCell<Plot3DState>,
}

impl Plot3DHandle {
    fn new_xyz(kind: Plot3DKind, x: Value, y: Value, z: Value) -> Result<Self, Error> {
        let x = extract_f64_vec(x)?;
        let y = extract_f64_vec(y)?;
        let z1 = extract_f64_vec(z)?;
        if x.len() != y.len() || x.len() != z1.len() {
            return Err(arg_err("scatter3d/line3d: x, y, z must have the same length"));
        }
        Ok(Plot3DHandle {
            kind,
            state: RefCell::new(Plot3DState { x, y, z1, ..Default::default() }),
        })
    }

    fn new_grid(kind: Plot3DKind, x: Value, y: Value, z: Value) -> Result<Self, Error> {
        let x = extract_f64_vec(x)?;
        let y = extract_f64_vec(y)?;
        let z2 = extract_f64_matrix(z)?;
        if z2.len() != y.len() || z2.iter().any(|r| r.len() != x.len()) {
            return Err(arg_err(
                "surface/wireframe: z must be a (y.len x x.len) 2-D grid",
            ));
        }
        Ok(Plot3DHandle {
            kind,
            state: RefCell::new(Plot3DState { x, y, z2, ..Default::default() }),
        })
    }

    fn scatter3d(x: Value, y: Value, z: Value) -> Result<Self, Error> {
        Self::new_xyz(Plot3DKind::Scatter, x, y, z)
    }
    fn line3d(x: Value, y: Value, z: Value) -> Result<Self, Error> {
        Self::new_xyz(Plot3DKind::Line, x, y, z)
    }
    fn surface(x: Value, y: Value, z: Value) -> Result<Self, Error> {
        Self::new_grid(Plot3DKind::Surface, x, y, z)
    }
    fn wireframe(x: Value, y: Value, z: Value) -> Result<Self, Error> {
        Self::new_grid(Plot3DKind::Wireframe, x, y, z)
    }

    fn title(&self, s: String) -> Result<(), Error> {
        self.state.borrow_mut().title = Some(s);
        Ok(())
    }
    fn xlabel(&self, s: String) -> Result<(), Error> {
        self.state.borrow_mut().xlabel = Some(s);
        Ok(())
    }
    fn ylabel(&self, s: String) -> Result<(), Error> {
        self.state.borrow_mut().ylabel = Some(s);
        Ok(())
    }
    fn zlabel(&self, s: String) -> Result<(), Error> {
        self.state.borrow_mut().zlabel = Some(s);
        Ok(())
    }
    fn color(&self, c: String) -> Result<(), Error> {
        self.state.borrow_mut().color = opt_color(Some(c))?;
        Ok(())
    }
    fn marker(&self, m: String) -> Result<(), Error> {
        self.state.borrow_mut().marker = Some(parse_marker(&m)?);
        Ok(())
    }
    fn marker_size(&self, s: f64) -> Result<(), Error> {
        self.state.borrow_mut().marker_size = Some(s as f32);
        Ok(())
    }
    fn line_width(&self, w: f64) -> Result<(), Error> {
        self.state.borrow_mut().line_width = Some(w as f32);
        Ok(())
    }

    fn save(&self, path: String) -> Result<(), Error> {
        let st = self.state.borrow();
        macro_rules! common {
            ($b:expr) => {{
                let mut b = $b;
                if let Some(t) = &st.title { b = b.title(t.clone()); }
                if let Some(s) = &st.xlabel { b = b.xlabel(s.clone()); }
                if let Some(s) = &st.ylabel { b = b.ylabel(s.clone()); }
                if let Some(s) = &st.zlabel { b = b.zlabel(s.clone()); }
                b
            }};
        }
        let result = match self.kind {
            Plot3DKind::Scatter => {
                let mut b = common!(scatter3d(&st.x, &st.y, &st.z1));
                if let Some(c) = st.color { b = b.color(c); }
                if let Some(m) = st.marker { b = b.marker(m); }
                if let Some(s) = st.marker_size { b = b.marker_size(s); }
                b.save(&path)
            }
            Plot3DKind::Line => {
                let mut b = common!(line3d(&st.x, &st.y, &st.z1));
                if let Some(c) = st.color { b = b.color(c); }
                if let Some(w) = st.line_width { b = b.line_width(w); }
                b.save(&path)
            }
            Plot3DKind::Surface => {
                let mut b = common!(surface(&st.x, &st.y, &st.z2));
                if let Some(c) = st.color { b = b.color(c); }
                b.save(&path)
            }
            Plot3DKind::Wireframe => {
                let mut b = common!(wireframe(&st.x, &st.y, &st.z2));
                if let Some(c) = st.color { b = b.color(c); }
                if let Some(w) = st.line_width { b = b.line_width(w); }
                b.save(&path)
            }
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
    handle.define_method("dpi", method!(PlotHandle::dpi, 1))?;
    handle.define_method("title", method!(PlotHandle::title, 1))?;
    handle.define_method("xlabel", method!(PlotHandle::xlabel, 1))?;
    handle.define_method("ylabel", method!(PlotHandle::ylabel, 1))?;
    handle.define_method("xscale", method!(PlotHandle::xscale, 2))?;
    handle.define_method("yscale", method!(PlotHandle::yscale, 2))?;
    handle.define_method("grid", method!(PlotHandle::grid, 1))?;
    handle.define_method("legend", method!(PlotHandle::legend, 1))?;
    handle.define_method("theme", method!(PlotHandle::theme, 1))?;
    handle.define_method("font_family", method!(PlotHandle::font_family, 1))?;
    handle.define_method("font_size", method!(PlotHandle::font_size, 1))?;
    handle.define_method("title_size", method!(PlotHandle::title_size, 1))?;
    handle.define_method("legend_font_size", method!(PlotHandle::legend_font_size, 1))?;
    handle.define_method("scale_typography", method!(PlotHandle::scale_typography, 1))?;
    handle.define_method("xlim", method!(PlotHandle::xlim, 2))?;
    handle.define_method("ylim", method!(PlotHandle::ylim, 2))?;
    handle.define_method("hline", method!(PlotHandle::hline, 4))?;
    handle.define_method("vline", method!(PlotHandle::vline, 4))?;
    handle.define_method("annotate_text", method!(PlotHandle::annotate_text, 5))?;
    handle.define_method("rect", method!(PlotHandle::rect, 6))?;
    handle.define_method("line", method!(PlotHandle::line, 6))?;
    handle.define_method("error_bars", method!(PlotHandle::error_bars, 6))?;
    handle.define_method("polar_line", method!(PlotHandle::polar_line, 5))?;
    handle.define_method("fast", method!(PlotHandle::fast, 1))?;
    handle.define_method("scatter", method!(PlotHandle::scatter, 7))?;
    handle.define_method("bar", method!(PlotHandle::bar, 5))?;
    handle.define_method("histogram", method!(PlotHandle::histogram, 5))?;
    handle.define_method("area", method!(PlotHandle::area, 7))?;
    handle.define_method("boxplot", method!(PlotHandle::boxplot, 4))?;
    handle.define_method("kde", method!(PlotHandle::kde, 4))?;
    handle.define_method("ecdf", method!(PlotHandle::ecdf, 4))?;
    handle.define_method("violin", method!(PlotHandle::violin, 4))?;
    handle.define_method("heatmap", method!(PlotHandle::heatmap, 4))?;
    handle.define_method("contour", method!(PlotHandle::contour, 5))?;
    handle.define_method("pie", method!(PlotHandle::pie, 3))?;
    handle.define_method("radar", method!(PlotHandle::radar, 3))?;
    handle.define_method("save", method!(PlotHandle::save, 1))?;

    let sub = module.define_class("SubplotHandle", ruby.class_object())?;
    module.define_singleton_method("_subplots", function!(SubplotHandle::new, 4))?;
    sub.define_method("suptitle", method!(SubplotHandle::suptitle, 1))?;
    sub.define_method("suptitle_font_size", method!(SubplotHandle::suptitle_font_size, 1))?;
    sub.define_method("subplot", method!(SubplotHandle::subplot, 3))?;
    sub.define_method("subplot_at", method!(SubplotHandle::subplot_at, 2))?;
    sub.define_method("save", method!(SubplotHandle::save, 1))?;

    let p3 = module.define_class("Plot3DHandle", ruby.class_object())?;
    module.define_singleton_method("_scatter3d", function!(Plot3DHandle::scatter3d, 3))?;
    module.define_singleton_method("_line3d", function!(Plot3DHandle::line3d, 3))?;
    module.define_singleton_method("_surface", function!(Plot3DHandle::surface, 3))?;
    module.define_singleton_method("_wireframe", function!(Plot3DHandle::wireframe, 3))?;
    p3.define_method("title", method!(Plot3DHandle::title, 1))?;
    p3.define_method("xlabel", method!(Plot3DHandle::xlabel, 1))?;
    p3.define_method("ylabel", method!(Plot3DHandle::ylabel, 1))?;
    p3.define_method("zlabel", method!(Plot3DHandle::zlabel, 1))?;
    p3.define_method("color", method!(Plot3DHandle::color, 1))?;
    p3.define_method("marker", method!(Plot3DHandle::marker, 1))?;
    p3.define_method("marker_size", method!(Plot3DHandle::marker_size, 1))?;
    p3.define_method("line_width", method!(Plot3DHandle::line_width, 1))?;
    p3.define_method("save", method!(Plot3DHandle::save, 1))?;

    Ok(())
}
