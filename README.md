# askds

Ask [DeepSeek](https://www.deepseek.com/) a question about what you are doing in the terminal.

`askds` reads your current terminal scrollback — commands **and their output** — and sends it
to DeepSeek together with your question. The answer is streamed directly to the terminal.

Anything removed by `clear` is excluded automatically, because `askds` reads the terminal's
live scrollback rather than a permanent log.

```
$ echo "Hello, world!"
$ clear
$ ls
Cargo.toml  README.md  src  target
$ askds what file extension is most common here
```

In the example above, DeepSeek only sees the `ls` output, since the `echo` output was cleared.

## Installation

Requires Rust 1.75+.

```sh
cargo install --path .
# or
cargo build --release
install -m 755 target/release/askds ~/.local/bin/askds
```

## Setup

Save your DeepSeek API key:

```sh
askds --key sk-your-key-here
```

This writes `~/.config/askds/config.json` (or `$XDG_CONFIG_HOME/askds/config.json`) with
owner-only permissions (`0600`).

You can also set the `DEEPSEEK_API_KEY` environment variable instead; it takes precedence
over the config file.

## Usage

```sh
# Ask using the current terminal session for context
askds what file extension is most common here

# Ask without sending any terminal history
askds -s what does this project do

# Override the model (default: deepseek-chat)
askds --model deepseek-reasoner why is this build failing

# Save a new API key
askds --key sk-your-key-here
```

All remaining words are joined into a single prompt, so quoting is optional:

```sh
askds why did that last command fail
```

## Terminal support

`askds` reads scrollback through the terminal itself, so cleared content is naturally
excluded. Supported terminals:

| Terminal | Mechanism |
| --- | --- |
| Konsole | D-Bus `getAllDisplayedText` (requires `qdbus6` or `qdbus`) |
| tmux | `tmux capture-pane` |
| kitty | `kitty @ get-text` (requires remote control enabled) |
| WezTerm | `wezterm cli get-text` |

On an unsupported terminal, `askds` prints a warning to stderr and asks the question without
history. Use `-s` to skip history silently.

The most recent 120,000 characters of history are sent, keeping the newest output when the
scrollback is larger than that.

## Configuration

`~/.config/askds/config.json`:

```json
{
  "api_key": "sk-...",
  "model": "deepseek-chat"
}
```

Both fields are optional. The model can always be overridden with `--model`.

## How it works

1. Reads the visible terminal scrollback (minus the `askds` invocation line itself).
2. Builds a request to `https://api.deepseek.com/chat/completions`.
3. Streams the assistant's response to stdout.

## Development

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```

## License

The [askds License 1.0](LICENSE) — source-available. You may use, fork, and
modify askds, including for commercial and internal work. You may not sell it or
bundle it as a standalone paid product, and any use must include a visible link
to the original repository:

> https://github.com/inxanedev/askds

This is not an OSI-approved open source license.
