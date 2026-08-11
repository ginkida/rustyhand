// TUI key-event handlers idiomatically use `match key.code { KeyCode::X => if ... }`
// for readability — the guard form (`KeyCode::X if cond => ...`) would scatter the
// condition across many arms and obscure the control flow.
#![allow(clippy::collapsible_match)]

pub mod agents;
pub mod audit;
pub mod channels;
pub mod chat;
pub mod dashboard;
pub mod extensions;
pub mod init_wizard;
pub mod logs;
pub mod memory;
pub mod peers;
pub mod security;
pub mod sessions;
pub mod settings;
pub mod skills;
pub mod templates;
pub mod triggers;
pub mod usage;
pub mod util;
pub mod welcome;
pub mod wizard;
pub mod workflows;
