fn main() {
    println!("cargo:rerun-if-changed=resources/heroinn.rc");
    println!("cargo:rerun-if-changed=resources/heroinn.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("resources/heroinn.rc");
    }
}
