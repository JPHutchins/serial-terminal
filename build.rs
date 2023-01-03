use vergen::{vergen, Config, ShaKind};

fn main() {
    let mut config = Config::default();
    *config.git_mut().sha_kind_mut() = ShaKind::Short;
    *config.git_mut().semver_dirty_mut() = Some("-dirty");

    vergen(config).unwrap();
}
