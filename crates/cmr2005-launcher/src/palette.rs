//! The settings window's colours and sizes, in the SDK's roles
//! (`rea_launcher_ui::theme`).
//!
//! Colin McRae Rally 2005's own menus: a white band under sepia 3D
//! backdrops, warm-black condensed headings, brown secondary text, and the
//! red of the logo's "2005", kept for the one action that starts the game.
//! The hero photo is toned sepia to match (`ui::HERO_JPEG`). The tests
//! below keep the palette within WCAG AA and the text-size rules.

use rea_launcher_ui::egui::Color32;
use rea_launcher_ui::theme::{Metrics, Palette, tint};

/// The menus' white band.
pub const BASE: Color32 = Color32::from_rgb(0xfb, 0xfa, 0xf8);
/// Fields and cards: a pale grey taken toward the backdrops' sepia.
pub const PANEL: Color32 = Color32::from_rgb(0xec, 0xe7, 0xe2);
pub const LINE: Color32 = Color32::from_rgb(0x8a, 0x7c, 0x72);
/// The headings' warm black.
pub const INK: Color32 = Color32::from_rgb(0x1c, 0x16, 0x12);
/// The menus' brown subtitles ("Play Stages or Rally.").
pub const MUTED: Color32 = Color32::from_rgb(0x66, 0x57, 0x4d);
/// The logo's red.
pub const ACCENT: Color32 = Color32::from_rgb(0xc4, 0x12, 0x2e);
/// The backdrops' dark sepia: the chosen look, like the menus' selected item.
pub const SEPIA: Color32 = Color32::from_rgb(0x3d, 0x2f, 0x26);
pub const ERROR: Color32 = Color32::from_rgb(0x9e, 0x1b, 0x14);

pub const PALETTE: Palette = Palette {
    base: BASE,
    panel: PANEL,
    line: LINE,
    ink: INK,
    muted: MUTED,
    action: ACCENT,
    action_hover: Color32::from_rgb(0xa3, 0x0d, 0x24),
    on_action: Color32::WHITE,
    select: SEPIA,
    on_select: Color32::WHITE,
    error: ERROR,
    disabled_ink: Color32::from_rgb(0x8f, 0x84, 0x7c),
    disabled_line: Color32::from_rgb(0xd6, 0xcf, 0xc8),
    focus: SEPIA,
    selection: tint(ACCENT, 64),
};

pub const METRICS: Metrics = Metrics::DEFAULT;

/// Widget fills for a light palette. The SDK theme (v0.3.3) starts from
/// egui's dark visuals and sets no widget fill, so sliders, number boxes and
/// buttons stay dark grey under this palette's dark text. Call after
/// `theme::install_with`. To be fixed in rea-launcher-sdk
/// (docs/CONTRIBUTIONS.md), then removed here.
pub fn light_widgets(ctx: &rea_launcher_ui::egui::Context) {
    let hover = Color32::from_rgb(0xe2, 0xdb, 0xd4);
    ctx.all_styles_mut(move |style| {
        let w = &mut style.visuals.widgets;
        for (widget, fill) in [
            (&mut w.noninteractive, PANEL),
            (&mut w.inactive, PANEL),
            (&mut w.hovered, hover),
            (&mut w.active, hover),
            (&mut w.open, hover),
        ] {
            widget.bg_fill = fill;
            widget.weak_bg_fill = fill;
            widget.fg_stroke.color = INK;
        }
        style.visuals.faint_bg_color = PANEL;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use rea_launcher_ui::theme;

    #[test]
    fn the_palette_meets_wcag_aa() {
        assert_eq!(
            theme::contrast_problems(&PALETTE, &METRICS),
            Vec::<String>::new()
        );
    }

    #[test]
    fn widgets_are_light_with_dark_text() {
        let ctx = rea_launcher_ui::egui::Context::default();
        theme::install_with(&ctx, &PALETTE, &METRICS);
        light_widgets(&ctx);
        let style = ctx.style_of(rea_launcher_ui::egui::Theme::Dark);
        for widget in [
            &style.visuals.widgets.inactive,
            &style.visuals.widgets.hovered,
        ] {
            assert!(
                theme::contrast(INK, widget.bg_fill) >= 4.5,
                "{:?} on {:?}",
                INK,
                widget.bg_fill
            );
            assert_eq!(widget.weak_bg_fill, widget.bg_fill);
        }
    }

    #[test]
    fn no_text_is_too_small() {
        assert_eq!(theme::text_size_problems(&METRICS), Vec::<String>::new());
    }
}
