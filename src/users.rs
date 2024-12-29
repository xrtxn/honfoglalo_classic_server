#[derive(Clone, Debug)]
pub enum ServerCommand {
	SelectArea(u8),
	QuestionAnswer(u8),
	TipAnswer(i32),
	Ready,
}
