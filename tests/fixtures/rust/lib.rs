fn entry() {
    helper();
    nested::target();
}

fn helper() {
    target();
}

fn target() {}

mod nested {
    pub fn target() {}
}
