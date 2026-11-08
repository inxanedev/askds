use std::io::{BufRead, BufReader, Write};

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};

const API_URL: &str = "https://api.deepseek.com/chat/completions";

const SYSTEM_PROMPT: &str = "You are askds, a concise and practical terminal assistant. \
Answer the user's question directly in plain text that reads well in a terminal. \
When terminal history is included, use it as context: pay attention to the commands that \
were run, their output, the working directory, and any errors. Do not repeat the terminal \
history back to the user. If the history does not contain enough information to answer, say \
so briefly. Keep answers short unless the question clearly needs detail.";

pub fn ask(api_key: &str, model: &str, prompt: &str, history: Option<&str>) -> Result<()> {
    ask_at(API_URL, api_key, model, prompt, history)
}

fn ask_at(
    url: &str,
    api_key: &str,
    model: &str,
    prompt: &str,
    history: Option<&str>,
) -> Result<()> {
    let user_content = match history {
        Some(history) if !history.trim().is_empty() => format!(
            "Here is my current terminal session (oldest output first, most recent last):\n\n\
             <terminal_history>\n{history}\n</terminal_history>\n\n\
             Question: {prompt}"
        ),
        _ => prompt.to_string(),
    };

    let body = json!({
        "model": model,
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            { "role": "user", "content": user_content }
        ],
        "stream": true
    });

    let response = match ureq::post(url)
        .set("Authorization", &format!("Bearer {api_key}"))
        .set("Content-Type", "application/json")
        .send_json(body)
    {
        Ok(response) => response,
        Err(ureq::Error::Status(status, response)) => {
            let detail = response
                .into_string()
                .ok()
                .and_then(|body| extract_api_error(&body))
                .unwrap_or_else(|| "no details returned".to_string());
            bail!("DeepSeek API returned HTTP {status}: {detail}");
        }
        Err(err) => return Err(err).context("could not reach the DeepSeek API"),
    };

    let reader = BufReader::new(response.into_reader());
    let mut wrote_any = false;

    for line in reader.lines() {
        let line = line.context("failed while reading the API response")?;
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data.is_empty() {
            continue;
        }
        if data == "[DONE]" {
            break;
        }

        let Ok(event) = serde_json::from_str::<Value>(data) else {
            continue;
        };

        if let Some(message) = event
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
        {
            bail!("DeepSeek API error: {message}");
        }

        if let Some(content) = event
            .get("choices")
            .and_then(|choices| choices.get(0))
            .and_then(|choice| choice.get("delta"))
            .and_then(|delta| delta.get("content"))
            .and_then(Value::as_str)
        {
            print!("{content}");
            std::io::stdout().flush().ok();
            wrote_any = true;
        }
    }

    if wrote_any {
        println!();
    } else {
        return Err(anyhow!("DeepSeek returned an empty response"));
    }

    Ok(())
}

fn extract_api_error(body: &str) -> Option<String> {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .or_else(|| Some(body.trim().to_string()).filter(|body| !body.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_an_api_error_message() {
        let body = r#"{"error":{"message":"Authentication Fails","type":"authentication_error"}}"#;
        assert_eq!(
            extract_api_error(body).as_deref(),
            Some("Authentication Fails")
        );
    }

    #[test]
    fn falls_back_to_the_raw_body() {
        assert_eq!(
            extract_api_error("bad gateway").as_deref(),
            Some("bad gateway")
        );
    }

    #[test]
    fn streams_content_from_an_sse_response() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();

        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();

            let mut headers = Vec::new();
            let mut byte = [0_u8; 1];
            while !headers.ends_with(b"\r\n\r\n") {
                stream.read_exact(&mut byte).unwrap();
                headers.push(byte[0]);
            }

            let headers = String::from_utf8_lossy(&headers);
            let content_length = headers
                .split("\r\n")
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap();

            let mut body = vec![0_u8; content_length];
            stream.read_exact(&mut body).unwrap();

            let request: Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(request["model"], "test-model");
            assert!(request["messages"][1]["content"]
                .as_str()
                .unwrap()
                .contains("Question: hi"));

            let body = concat!(
                "data: {\"choices\":[{\"delta\":{\"content\":\"Hel\"}}]}\n\n",
                "data: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}]}\n\n",
                "data: [DONE]\n\n"
            );
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        });

        let result = ask_at(
            &format!("http://{address}/chat/completions"),
            "sk-test",
            "test-model",
            "hi",
            Some("$ ls\nfile.txt"),
        );

        assert!(result.is_ok(), "{result:?}");
        server.join().unwrap();
    }
}
