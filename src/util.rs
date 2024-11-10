use std::{env, path::PathBuf};

use walkdir::WalkDir;

pub fn get_path(filename: String) -> Option<PathBuf> {
    let repo_root = match env::var("KNOWLEDGES") {
        Ok(path) => PathBuf::from(path),
        Err(_) => { unreachable!(); /* 起動時に確認済み */}
    };
    
    let target = format!("{filename}.md");
    
    for entry in WalkDir::new(repo_root).into_iter().filter_map(|e|e.ok()) {
        let path = entry.path();
        if path.is_file() && path.file_name().map_or(false, |f| f == target.as_str()) {
            return Some(path.to_path_buf());
        }
    }
    None
}
