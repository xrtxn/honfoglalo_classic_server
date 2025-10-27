#[derive(Clone, Debug)]
pub enum ServerCommand {
	SelectArea(u8),
	QuestionAnswer(u8),
	TipAnswer(i32),
	Ready,
}

impl ServerCommand {
	// this looks nicer
	#[allow(clippy::match_like_matches_macro)]
	/// Checks if two commands are of the same variant, while ignoring the inner data.
	pub(crate) fn variant_eq(&self, other: &Self) -> bool {
		match (self, other) {
			(ServerCommand::SelectArea(_), ServerCommand::SelectArea(_)) => true,
			(ServerCommand::QuestionAnswer(_), ServerCommand::QuestionAnswer(_)) => true,
			(ServerCommand::TipAnswer(_), ServerCommand::TipAnswer(_)) => true,
			(ServerCommand::Ready, ServerCommand::Ready) => true,
			_ => false,
		}
	}
}
