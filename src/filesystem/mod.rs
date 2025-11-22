pub mod path_utils;
pub mod file_info;
pub mod scanner;
pub mod symlinks;
pub mod files_from;
pub mod windows_scanner;
pub mod buffer_optimizer;

#[allow(unused_imports)]
pub use file_info::{FileInfo, FileType, human_readable_size};
#[allow(unused_imports)]
pub use scanner::Scanner;
#[allow(unused_imports)]
pub use path_utils::{
    normalize_path,
    is_unc_path,
    to_long_path,
    normalize_separators,
    to_unix_separators,
    exceeds_max_path,
    paths_equal_ignore_case,
};
#[allow(unused_imports)]
pub use symlinks::{
    is_symlink,
    read_link,
    create_symlink,
    detect_symlink_loop,
    resolve_symlink,
    copy_symlink,
    copy_symlink_content,
    SymlinkInfo,
    get_symlink_info,
};
#[allow(unused_imports)]
pub use files_from::read_files_from;
