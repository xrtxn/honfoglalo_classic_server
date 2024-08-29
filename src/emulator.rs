pub trait Emulator {
	fn emulate() -> Self;
}

pub fn remove_root_tag(xml: String) -> String {
	xml.replace("<ROOT>", "").replace("</ROOT>", "")
}
