use cssparser_color::Color;
use lsp_types::{
    ColorInformation, ColorPresentation, ColorPresentationParams, DocumentColorParams,
};

use crate::{context::FileLang, utils::location_to_lsp_range, wxss::StyleSheet, ServerContext};

pub(crate) async fn color_presentation(
    ctx: ServerContext,
    params: ColorPresentationParams,
) -> anyhow::Result<Vec<ColorPresentation>> {
    let ret = ctx
        .clone()
        .project_thread_task(
            &params.text_document.uri,
            move |_project, _abs_path, _| -> anyhow::Result<Vec<ColorPresentation>> {
                let mut ret = vec![];
                let rgba = convert_lsp_color_u8(&params.color);
                let rgba_str = if rgba.3 == 1. {
                    format!("rgb({}, {}, {})", rgba.0, rgba.1, rgba.2)
                } else {
                    format!("rgba({}, {}, {}, {})", rgba.0, rgba.1, rgba.2, rgba.3)
                };
                ret.push(ColorPresentation {
                    label: rgba_str,
                    text_edit: None,
                    additional_text_edits: None,
                });
                Ok(ret)
            },
        )
        .await??;
    Ok(ret)
}

pub(crate) async fn color(
    ctx: ServerContext,
    params: DocumentColorParams,
) -> anyhow::Result<Vec<ColorInformation>> {
    let ret = ctx
        .clone()
        .project_thread_task(
            &params.text_document.uri,
            move |project, abs_path, file_lang| -> anyhow::Result<Vec<ColorInformation>> {
                let ranges = match file_lang {
                    FileLang::Wxml => {
                        vec![]
                    }
                    FileLang::Wxss => {
                        let sheet = project.get_style_sheet(&abs_path)?;
                        collect_wxss_colors(sheet)
                    }
                    _ => vec![],
                };
                Ok(ranges)
            },
        )
        .await??;
    Ok(ret)
}

fn convert_lsp_color_u8(color: &lsp_types::Color) -> (u8, u8, u8, f32) {
    let r = (color.red * 255.).round() as u8;
    let g = (color.green * 255.).round() as u8;
    let b = (color.blue * 255.).round() as u8;
    (r, g, b, color.alpha)
}

fn convert_css_color(color: &Color) -> Option<(f32, f32, f32, f32)> {
    match color {
        Color::Rgba(x) => {
            let r = x.red as f32 / 255.;
            let g = x.green as f32 / 255.;
            let b = x.blue as f32 / 255.;
            Some((r, g, b, x.alpha))
        }
        Color::Hsl(x) => {
            let (r, g, b) = cssparser_color::hsl_to_rgb(x.hue? / 360., x.saturation?, x.lightness?);
            Some((r, g, b, 1.))
        }
        Color::Hwb(x) => {
            let (r, g, b) = cssparser_color::hwb_to_rgb(x.hue? / 360., x.whiteness?, x.blackness?);
            Some((r, g, b, 1.))
        }
        _ => None,
    }
}

fn collect_wxss_colors(sheet: &StyleSheet) -> Vec<ColorInformation> {
    let mut ret = vec![];
    for (color, loc) in sheet.special_locations.colors.iter() {
        let Some(rgba) = convert_css_color(color) else {
            continue;
        };
        let lsp_color = lsp_types::Color {
            red: rgba.0,
            green: rgba.1,
            blue: rgba.2,
            alpha: rgba.3,
        };
        ret.push(ColorInformation {
            range: location_to_lsp_range(loc),
            color: lsp_color,
        });
    }
    ret
}
