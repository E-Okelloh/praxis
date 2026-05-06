//! SVG flame graph rendering via `inferno`.

use inferno::flamegraph::{self, Options};

use crate::{ProfileError, Sample};

/// Options forwarded to the `inferno` flame graph renderer.
#[derive(Debug, Clone)]
pub struct FlameConfig {
    /// Title shown at the top of the SVG.
    pub title: Option<String>,
    /// Width of the SVG in pixels.
    pub width: u32,
    /// Minimum width (in pixels) of a frame to render.
    pub min_width: f64,
}

impl Default for FlameConfig {
    fn default() -> Self {
        Self {
            title: None,
            width: 1200,
            min_width: 0.1,
        }
    }
}

/// Render samples as an SVG flame graph.
///
/// Each sample becomes one `inferno` folded-stack line:
/// `<program>;<label>  <cu>`.
pub(crate) fn render_svg(
    program_name: &str,
    samples: &[Sample],
    cfg: &FlameConfig,
) -> Result<Vec<u8>, ProfileError> {
    // Build inferno's "folded stacks" format: each line is `stack  count`.
    // We model: root frame = program name, child frame = instruction label.
    let lines: Vec<String> = samples
        .iter()
        .map(|s| format!("{};{}  {}", program_name, s.label, s.cu))
        .collect();

    let mut opts = Options::default();
    opts.title = cfg
        .title
        .clone()
        .unwrap_or_else(|| format!("{} — CU flame graph", program_name));
    opts.image_width = Some(cfg.width as usize);
    opts.min_width = cfg.min_width;
    // Use count-axis label that makes sense for CU data.
    opts.count_name = "CU".into();

    let mut svg_buf: Vec<u8> = Vec::new();
    flamegraph::from_lines(
        &mut opts,
        lines.iter().map(|s| s.as_str()),
        &mut svg_buf,
    )
    .map_err(|e| ProfileError::FlameGraph(e.to_string()))?;

    Ok(svg_buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_produces_svg_bytes() {
        let samples = vec![
            Sample { label: "initialize".into(), cu: 1_200 },
            Sample { label: "transfer".into(),   cu: 3_800 },
            Sample { label: "transfer".into(),   cu: 3_600 },
        ];
        let svg = render_svg("test_program", &samples, &FlameConfig::default()).unwrap();
        assert!(!svg.is_empty());
        // inferno SVG output begins with the XML/SVG declaration or svg tag.
        let text = String::from_utf8_lossy(&svg);
        assert!(text.contains("<svg"), "output should contain <svg");
    }
}
