use std::fs;
use std::path::Path;

#[allow(dead_code)]
#[path = "../svg.rs"]
mod svg;

fn main() {
    let default_dir = "plugin/io.github.chimi6.limelight.sdPlugin/images".to_string();
    let out_dir = std::env::args().nth(1).unwrap_or(default_dir);
    for (relative, content) in svg::static_icons() {
        let path = Path::new(&out_dir).join(relative);
        let parent = path.parent().expect("icon path has a parent");
        fs::create_dir_all(parent).expect("create icon directory");
        fs::write(&path, content).expect("write icon");
        println!("wrote {}", path.display());
    }
}
