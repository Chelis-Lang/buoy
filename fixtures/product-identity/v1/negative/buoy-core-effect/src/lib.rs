#![no_std]

pub fn forbidden_shell_dependency() -> usize {
    core::mem::size_of::<buoy::repository::RepositoryConfig>()
}
