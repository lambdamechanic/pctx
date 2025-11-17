pub(crate) mod jsonl_logger;
pub(crate) mod logger;
pub(crate) mod prompts;
pub(crate) mod spinner;
pub(crate) mod styles;

pub(crate) static LOGO: &str = include_str!("./ascii-logo.txt");
pub(crate) static CHECK: &str = "✔";
pub(crate) static MARK: &str = "✘";
