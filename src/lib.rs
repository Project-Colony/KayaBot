//! KayaBot renames movie and TV episode files from what their names say and
//! what online metadata sources know about them.

pub mod api_keys;
pub mod config;
pub mod export;
pub mod formatting;
pub mod i18n;
pub mod lookup;
pub mod matching;
pub mod metadata;
pub mod paths;
pub mod rename;
pub mod ui;

#[cfg(test)]
mod test_support;
