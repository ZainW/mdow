//! Mermaid diagrams through merman, themed to match the Electron reader's diagram palette.

use super::{Graphic, guarded, measure::FontMeasurer, rasterize_svg};
use merman::{
    Engine, MermaidConfig, OperationControl, RenderOutput, RenderRequest, Renderer, SvgRequest,
    svg::{
        CssOverridePolicy, MeasurementProfileId, ScopedCssPostprocessor, SvgPipeline,
        TextMeasurementPolicy, TextMeasurementProfile, TextMeasurementProfileIdentity,
    },
};
use resvg::usvg::fontdb;
use serde_json::{Value, json};
use std::sync::{Arc, LazyLock};

/// Diagrams are drawn in Geist Mono, the Electron reader's fallback diagram face.
pub(crate) const DIAGRAM_FONT: &[u8] = include_bytes!("../../assets/fonts/GeistMono-Variable.ttf");
const DIAGRAM_FONT_FAMILY: &str = "'Geist Mono', ui-monospace, monospace";
const DIAGRAM_FONT_SIZE: &str = "13.5px";
const DIAGRAM_RADIUS: &str = "6px";

static FONTS: LazyLock<Arc<fontdb::Database>> = LazyLock::new(|| {
    let mut database = fontdb::Database::new();
    database.load_font_data(DIAGRAM_FONT.to_vec());
    // System faces cover scripts and emoji the diagram font lacks.
    database.load_system_fonts();
    database.set_monospace_family("Geist Mono");
    Arc::new(database)
});

static MEASUREMENT: LazyLock<TextMeasurementPolicy> = LazyLock::new(|| {
    let identity = TextMeasurementProfileIdentity::new(
        MeasurementProfileId::new("mdow.geist-mono").expect("static profile id is valid"),
        "1",
    )
    .expect("static profile version is valid");
    TextMeasurementPolicy::uniform(TextMeasurementProfile::new(
        identity,
        Arc::new(FontMeasurer::new(DIAGRAM_FONT)),
    ))
});

/// The Electron reader's locked diagram palette, with the paper taken from the reader background
/// so label backgrounds blend into the page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagramPalette {
    pub dark: bool,
    pub paper: String,
    pub paper_warm: String,
    pub ink: String,
    pub muted: String,
    pub line: String,
}

impl DiagramPalette {
    pub fn new(dark: bool, paper: String) -> Self {
        let (paper_warm, ink, muted, line) = if dark {
            ("#1c1c1c", "#e4e4e4", "#a6a6a6", "#3d3d3d")
        } else {
            ("#faf9f5", "#3b3b3b", "#5e5e58", "#d8d7d0")
        };
        Self {
            dark,
            paper,
            paper_warm: paper_warm.into(),
            ink: ink.into(),
            muted: muted.into(),
            line: line.into(),
        }
    }

    fn theme_variables(&self) -> Value {
        let Self {
            paper,
            paper_warm,
            ink,
            muted,
            line,
            ..
        } = self;
        let mut variables = json!({
            "background": "transparent",
            "fontFamily": DIAGRAM_FONT_FAMILY,
            "fontSize": DIAGRAM_FONT_SIZE,
            "textColor": ink,
            "lineColor": muted,
            "primaryColor": paper_warm,
            "primaryTextColor": ink,
            "primaryBorderColor": line,
            "secondaryColor": paper_warm,
            "secondaryTextColor": ink,
            "secondaryBorderColor": line,
            "tertiaryColor": paper,
            "tertiaryTextColor": ink,
            "tertiaryBorderColor": line,
            "mainBkg": paper_warm,
            "secondBkg": paper,
            "nodeBorder": line,
            "clusterBkg": "transparent",
            "clusterBorder": line,
            "titleColor": ink,
            "edgeLabelBackground": paper,
            "nodeTextColor": ink,
        });
        let more = json!({
            "actorBorder": line,
            "actorBkg": paper_warm,
            "actorTextColor": ink,
            "actorLineColor": muted,
            "signalColor": muted,
            "signalTextColor": ink,
            "labelBoxBkgColor": paper_warm,
            "labelBoxBorderColor": line,
            "labelTextColor": ink,
            "loopTextColor": ink,
            "noteBorderColor": line,
            "noteBkgColor": paper_warm,
            "noteTextColor": ink,
            "activationBorderColor": line,
            "activationBkgColor": paper,
            "sequenceNumberColor": ink,
            "sectionBkgColor": paper_warm,
            "altSectionBkgColor": paper,
            "gridColor": line,
            "pieStrokeColor": muted,
            "pieOuterStrokeColor": line,
            "pieTitleTextColor": ink,
            "pieSectionTextColor": ink,
            "pieLegendTextColor": ink,
            "errorBkgColor": paper_warm,
            "errorTextColor": ink,
            "taskBkgColor": paper_warm,
            "taskBorderColor": line,
            "taskTextColor": ink,
            "taskTextDarkColor": ink,
            "taskTextLightColor": ink,
            "taskTextOutsideColor": ink,
            "activeTaskBkgColor": paper,
            "activeTaskBorderColor": muted,
            "doneTaskBkgColor": line,
            "doneTaskBorderColor": line,
        });
        let object = variables
            .as_object_mut()
            .expect("theme variables are an object");
        object.extend(
            more.as_object()
                .expect("theme variables are an object")
                .clone(),
        );
        for index in 0..12 {
            object.insert(format!("cScale{index}"), json!(paper_warm));
            object.insert(format!("cScaleLabel{index}"), json!(ink));
        }
        // Git graphs derive branch colors from the primary color, which is near the page
        // color in both palettes; draw branches in the muted line color instead.
        for index in 0..8 {
            object.insert(format!("git{index}"), json!(muted));
            object.insert(format!("gitInv{index}"), json!(paper));
            object.insert(format!("gitBranchLabel{index}"), json!(paper));
        }
        // Journey sections and faces default to pastels that lose light text in dark mode.
        for index in 0..8 {
            object.insert(format!("fillType{index}"), json!(paper_warm));
        }
        object.insert("faceColor".into(), json!(paper_warm));
        object.insert("quadrantPointFill".into(), json!(ink));
        object.insert("quadrantPointTextFill".into(), json!(ink));
        object.insert("commitLabelColor".into(), json!(ink));
        object.insert("commitLabelBackground".into(), json!(paper_warm));
        object.insert("tagLabelColor".into(), json!(ink));
        object.insert("tagLabelBackground".into(), json!(paper_warm));
        object.insert("tagLabelBorder".into(), json!(line));
        for index in 1..=12 {
            object.insert(format!("pie{index}"), json!(paper_warm));
        }
        variables
    }

    fn config(&self) -> MermaidConfig {
        MermaidConfig::from_value(json!({
            "theme": "base",
            "darkMode": self.dark,
            "fontFamily": DIAGRAM_FONT_FAMILY,
            "htmlLabels": false,
            "flowchart": {
                "htmlLabels": false,
                "curve": "linear",
                "padding": 12,
                "wrappingWidth": 200,
                "useMaxWidth": false,
            },
            "themeVariables": self.theme_variables(),
        }))
    }

    /// The Electron reader's `themeCSS`: flat fills, hairline strokes, one diagram font.
    fn css(&self) -> String {
        let Self {
            paper,
            paper_warm,
            ink,
            muted,
            line,
            ..
        } = self;
        // Mindmap edges take their section's fill color, which is the node paper here.
        let mindmap_edges = (-1..12)
            .map(|index| format!(".section-edge-{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "svg {{ background: transparent !important; }}
            .node rect, .node polygon, .node circle, .node ellipse, .node path,
            rect.actor, rect.actor-box, .classGroup rect, .labelBox, .note, .note rect,
            .statediagram-state rect, .er.entityBox, .requirement, .quoted {{
              fill: {paper_warm} !important; stroke: {line} !important;
              stroke-width: 1px !important; filter: none !important;
            }}
            .node rect, rect.actor, .classGroup rect, .labelBox, .note rect,
            .statediagram-state rect, .er.entityBox {{
              rx: {DIAGRAM_RADIUS} !important; ry: {DIAGRAM_RADIUS} !important;
            }}
            .edgePath .path, .flowchart-link, .messageLine0, .messageLine1,
            .transition, .relation, .relationLine, .loopLine {{
              stroke: {muted} !important; stroke-width: 1px !important; fill: none !important;
            }}
            marker path, .arrowheadPath, .marker {{
              fill: {muted} !important; stroke: {muted} !important;
            }}
            .label, .nodeLabel, .edgeLabel, text.actor, text.actor > tspan, .messageText,
            .loopText, .noteText, .titleText, text {{
              font-family: {DIAGRAM_FONT_FAMILY} !important; font-size: {DIAGRAM_FONT_SIZE} !important;
              fill: {ink} !important; color: {ink} !important;
            }}
            .edgeLabel rect, .labelBkg {{ fill: {paper} !important; stroke: none !important; }}
            .cluster rect {{ fill: transparent !important; stroke: {line} !important; }}
            {mindmap_edges} {{ stroke: {line} !important; }}"
        )
    }
}

/// Renders Mermaid `source` to SVG with the reader's palette.
pub fn render_mermaid_svg(source: &str, palette: &DiagramPalette) -> Result<String, String> {
    guarded(|| {
        let renderer =
            Renderer::new().with_engine(Engine::new().with_site_config(palette.config()));
        let pipeline = SvgPipeline::resvg_safe().with_postprocessor(
            ScopedCssPostprocessor::new(palette.css())
                .with_override_policy(CssOverridePolicy::StripExistingImportant),
        );
        let mut request = SvgRequest {
            pipeline: Some(pipeline),
            ..SvgRequest::default()
        };
        request.environment = request
            .environment
            .with_text_measurement_policy(MEASUREMENT.clone());
        match renderer.render(RenderRequest::svg(source, OperationControl::new(), request)) {
            Ok(RenderOutput::Svg(Some(svg))) => Ok(transparent_root(svg.svg())),
            Ok(_) => Err("the diagram produced no output".into()),
            Err(error) => Err(error.to_string()),
        }
    })
}

/// Drops the `background-color` merman puts on the root `<svg>` (white, whatever the theme):
/// resvg paints it, but the reader wants diagrams to sit on the page like the Electron reader's
/// `svg { background: transparent }`.
fn transparent_root(svg: &str) -> String {
    let Some(tag_end) = svg.find('>') else {
        return svg.to_owned();
    };
    let (root, rest) = svg.split_at(tag_end);
    let Some(start) = root.find("background-color:") else {
        return svg.to_owned();
    };
    let end = root[start..]
        .find([';', '"'])
        .map_or(root.len(), |offset| start + offset);
    let end = if root[end..].starts_with(';') {
        end + 1
    } else {
        end
    };
    format!("{}{}{rest}", &root[..start], &root[end..])
}

/// Renders Mermaid `source` to a raster at `scale` times its natural size.
pub fn render_mermaid(
    source: &str,
    palette: &DiagramPalette,
    scale: f32,
) -> Result<Graphic, String> {
    let svg = render_mermaid_svg(source, palette)?;
    guarded(|| rasterize_svg(&svg, Some(FONTS.clone()), None, scale))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{DocumentBlock, parse_document};
    use std::path::Path;

    fn light() -> DiagramPalette {
        DiagramPalette::new(false, "#fbf8f5".into())
    }

    #[test]
    fn renders_every_fixture_diagram_and_reports_the_invalid_one() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/diagrams.md");
        let parsed = parse_document(fixture.clone(), std::fs::read_to_string(&fixture).unwrap());
        let sources = parsed
            .blocks
            .iter()
            .filter_map(|block| match block {
                DocumentBlock::MermaidCard { source } => Some(source.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();

        let (invalid, valid) = sources.split_last().unwrap();
        for source in valid {
            let svg = render_mermaid_svg(source, &light())
                .unwrap_or_else(|error| panic!("{error}\n{source}"));
            assert!(svg.contains("<svg"), "{source}");
        }
        assert!(render_mermaid_svg(invalid, &light()).is_err());
    }

    #[test]
    fn root_background_is_removed_so_diagrams_sit_on_the_page() {
        assert_eq!(
            transparent_root(
                r#"<svg style="max-width:9px;background-color:white" a="b"><g style="background-color:red"/></svg>"#
            ),
            r#"<svg style="max-width:9px;" a="b"><g style="background-color:red"/></svg>"#
        );
        let svg = render_mermaid_svg("pie\n  \"A\" : 1\n", &light()).unwrap();
        let root = &svg[..svg.find('>').unwrap()];
        assert!(!root.contains("background-color"), "{root}");
    }

    #[test]
    fn diagrams_use_the_palette_and_the_diagram_font() {
        let svg = render_mermaid_svg("flowchart TD\n  A[Start] --> B[End]\n", &light()).unwrap();

        assert!(svg.contains("#faf9f5"), "paper-warm node fill");
        assert!(svg.contains("Geist Mono"), "diagram font");
        let dark = render_mermaid_svg(
            "flowchart TD\n  A[Start] --> B[End]\n",
            &DiagramPalette::new(true, "#090909".into()),
        )
        .unwrap();
        assert!(dark.contains("#1c1c1c"));
    }

    #[test]
    fn rasterizes_a_flowchart_at_its_natural_size() {
        let graphic =
            render_mermaid("flowchart LR\n  A[Open] --> B[Read]\n", &light(), 1.0).unwrap();

        assert!(
            graphic.width > 100.0 && graphic.height > 20.0,
            "{graphic:?}"
        );
        let size = graphic.image.size(0);
        assert_eq!(size.width.0 as f32, (graphic.width * 2.0).ceil());
    }
}
