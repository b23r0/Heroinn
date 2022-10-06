pub fn _warn(title: &String, desc: &String) {
    let mut msg = rfd::MessageDialog::new();
    msg = msg.set_level(rfd::MessageLevel::Warning);
    msg = msg.set_title(title);
    msg = msg.set_description(desc);
    msg = msg.set_buttons(rfd::MessageButtons::Ok);
    msg.show();
}

pub fn error(title: &String, desc: &String) {
    let mut msg = rfd::MessageDialog::new();
    msg = msg.set_level(rfd::MessageLevel::Error);
    msg = msg.set_title(title);
    msg = msg.set_description(desc);
    msg = msg.set_buttons(rfd::MessageButtons::Ok);
    msg.show();
}

pub fn _info(title: &String, desc: &String) {
    let mut msg = rfd::MessageDialog::new();
    msg = msg.set_level(rfd::MessageLevel::Info);
    msg = msg.set_title(title);
    msg = msg.set_description(desc);
    msg = msg.set_buttons(rfd::MessageButtons::Ok);
    msg.show();
}
