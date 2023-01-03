use const_format::formatcp;

pub const ABOUT: &str = "\
    A cross platform CLI for serial devices.\n\
    \n\
    Press ESC to bring up menu or Ctrl-C to exit.\
";

pub const HELP: &str = "\
    TODO: some more detailed usage information.\
";

pub const LONG_VERSION: &str = formatcp!(
    "{}
    build:  {} {} {}
    rustc:  {}
    llvm:   {}
    host:   {} 
    target: {}",
    env!("VERGEN_BUILD_SEMVER"),
    env!("VERGEN_CARGO_PROFILE"),
    env!("VERGEN_GIT_SEMVER"),
    env!("VERGEN_GIT_SHA_SHORT"),
    env!("VERGEN_RUSTC_SEMVER"),
    env!("VERGEN_RUSTC_LLVM_VERSION"),
    env!("VERGEN_RUSTC_HOST_TRIPLE"),
    env!("VERGEN_CARGO_TARGET_TRIPLE"),
);
