//! The Colin McRae Rally 2005 launcher: everything that can be tested on a Linux host lives
//! in this library; `main.rs` only wires it to Windows.
//!
//! - [`edition`]: which builds of cmr5.exe are known (SHA-256), and the
//!   refusal for any other build.
//! - [`config`]: the one option table behind `launcher.cfg`, the flags,
//!   `--help`, the template and the window rows.
//! - [`cli`]: what a command line asks for, and the `--check` report.
//! - [`form`]: the settings window's state as `launcher.cfg` text.
//! - [`source`]: an installed game, or GOG's installer to unpack.
//! - [`runtime`]: the private runtime copy of the game folder.
//! - [`saves`]: the launcher's folders and the saves folder.
//! - [`registry`]: the game's registry key and what a launch writes.
//! - [`components`]: DXVK, the close hook, innoextract, `dxvk.conf`.
//! - [`plan`]: everything one launch changes, for `main` and the tests.
//! - `palette`, `ui`: the settings window (Windows, and the host's tests).

/// The settings window's title. It differs from the game's own window
/// title ("Colin McRae Rally 2005"): a second start brings this window
/// forward (`rea_launcher_win32::instance::Window::Own`), never the game's.
pub const WINDOW_TITLE: &str = "Colin McRae Rally 2005 Launcher";

/// The single-instance name: one launcher at a time per user session (a
/// named mutex on Windows, per prefix under Wine).
pub const INSTANCE: &str = "cmr2005";

pub mod cli;
pub mod components;
pub mod config;
pub mod edition;
pub mod form;
pub mod plan;
pub mod registry;
pub mod runtime;
pub mod saves;
pub mod source;

#[cfg(any(windows, test))]
pub mod palette;
#[cfg(any(windows, test))]
pub mod ui;
