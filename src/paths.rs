use std::path::PathBuf;

pub fn app_config_dir() -> Option<PathBuf> {
    let base = if cfg!(windows) {
        dirs::data_local_dir()
    } else {
        dirs::config_dir()
    }?;

    Some(base.join("Colony").join("KayaBot"))
}
