use clap::Parser;

/// Ask DeepSeek a question about what you are doing in the terminal.
///
/// By default the current terminal session (commands and their output) is sent
/// along with your question. Anything removed by `clear` is not included.
#[derive(Debug, Parser)]
#[command(
    name = "askds",
    version,
    about = "Ask DeepSeek a question about your current terminal session",
    after_help = "EXAMPLES:\n  \
        askds --key sk-...\n  \
        askds what file extension is most common here\n  \
        askds -s what does this project do"
)]
pub struct Cli {
    /// Save the DeepSeek API key to ~/.config/askds/config.json and exit
    #[arg(long, value_name = "API_KEY")]
    pub key: Option<String>,

    /// Ask without including the terminal history
    #[arg(short = 's', long = "no-history")]
    pub no_history: bool,

    /// Override the DeepSeek model (default: deepseek-chat)
    #[arg(long, value_name = "MODEL")]
    pub model: Option<String>,

    /// The question to ask. All remaining words are joined into one prompt.
    #[arg(
        trailing_var_arg = true,
        allow_hyphen_values = true,
        num_args = 0..,
        value_name = "PROMPT"
    )]
    pub prompt: Vec<String>,
}
