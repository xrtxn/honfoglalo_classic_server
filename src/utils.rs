pub(crate) fn to_hex_with_length(bytes: &[u8], length: usize) -> String {
	let encoded = hex::encode(bytes);
	let trimmed = encoded.trim_start_matches('0');

	// Format the string to the desired length
	format!("{:0>width$}", trimmed, width = length).to_uppercase()
}

pub fn remove_root_tag(xml: String) -> String {
	xml.replace("<ROOT>", "").replace("</ROOT>", "")
}

pub(crate) fn modified_xml_response<T>(stru: &T) -> Result<String, anyhow::Error>
where
	T: ?Sized + serde::Serialize,
{
	Ok(remove_root_tag(quick_xml::se::to_string(stru)?))
}

pub(crate) fn split_string_n(input: &str, n: usize) -> Vec<&str> {
	let mut result = vec![];
	for i in (0..input.len()).step_by(n) {
		if i + n <= input.len() {
			result.push(&input[i..i + n]);
		}
	}
	result
}
