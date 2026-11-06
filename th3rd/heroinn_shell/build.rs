fn main() {
    #[cfg(windows)]
    embed_resource::compile("./windows/windows.rc");
}
