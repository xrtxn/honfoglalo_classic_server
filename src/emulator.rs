pub trait Emulator {
    fn emulate(mn: String) -> Self;
}
#[allow(dead_code)]
pub fn remove_root_tag(xml: String) -> String {
    xml.replace("<ROOT>", "").replace("</ROOT>", "")
}
