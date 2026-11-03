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

    /// The theme fills egui's widgets from this light palette (SDK v0.4.0;
    /// v0.3.3 left them egui's dark grey under dark ink, about 1.5:1).
    /// Checks every style the context holds, in every widget state.
    #[test]
    fn widgets_are_light_with_dark_text() {
        let ctx = rea_launcher_ui::egui::Context::default();
        theme::install_with(&ctx, &PALETTE, &METRICS);
        for which in [
            rea_launcher_ui::egui::Theme::Dark,
            rea_launcher_ui::egui::Theme::Light,
        ] {
            let style = ctx.style_of(which);
            let w = &style.visuals.widgets;
            for (state, widget) in [
                ("noninteractive", &w.noninteractive),
                ("inactive", &w.inactive),
                ("hovered", &w.hovered),
                ("active", &w.active),
                ("open", &w.open),
            ] {
                for fill in [widget.bg_fill, widget.weak_bg_fill] {
                    assert!(
                        theme::contrast(INK, fill) >= 4.5,
                        "{which:?} {state}: ink on {fill:?}"
                    );
                }
            }
        }
    }

    /// The check above fails for egui's dark widget fills.
    #[test]
    fn dark_widget_fills_fail_the_check() {
        let dark = rea_launcher_ui::egui::Visuals::dark();
        assert!(theme::contrast(INK, dark.widgets.inactive.bg_fill) < 4.5);
        assert!(theme::contrast(INK, dark.widgets.inactive.weak_bg_fill) < 4.5);
    }

    #[test]
    fn no_text_is_too_small() {
        assert_eq!(theme::text_size_problems(&METRICS), Vec::<String>::new());
    }
}
