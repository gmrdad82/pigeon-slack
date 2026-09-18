//! The Slack plugin for Done: one page in Plugins (the webhook URL, saved in the
//! host's own store and shown masked after the host), and a message to that webhook
//! when a Task comes back to the owner — done, stuck or capped — composed here from
//! the desk's `task-back` event. The desk itself never talks to Slack; the URL never
//! reaches a log line.
wit_bindgen::generate!({ world: "plugin", path: "wit" });
use pito::host::{kv, log, net, ui, ui_types};
use std::cell::{Cell, RefCell};

/// Where the URL lives in the plugin's store, and the field's id on the page.
const URL_KEY: &str = "webhook_url";
const FIELD: &str = "webhook";
/// The desk's event, as `on-event` receives it: the name, a colon, a JSON object.
const TASK_BACK: &str = "task-back:";
/// The body a webhook expects: a `text` field, Slack's mrkdwn inside.
const CONTENT_TYPE: &str = "application/json";

thread_local! {
    // What the owner typed since the last Save, so the field reads back what he types.
    static DRAFT: RefCell<Option<String>> = const { RefCell::new(None) };
    // The missing-URL warning is said once per activation, not once per Task.
    static WARNED_EMPTY: Cell<bool> = const { Cell::new(false) };
}

struct Slack;

impl Guest for Slack {
    fn info() -> String {
        "gmrdad82/pigeon-slack@0.1.0".into()
    }
    fn activate() {
        WARNED_EMPTY.with(|w| w.set(false));
    }
    fn deactivate() {}
    /// The one slot: the page in Plugins.
    fn render(_slot: ui::Slot, _context: String) -> ui_types::Tree {
        page()
    }
    fn on_event(event: ui_types::Event) {
        match event {
            ui_types::Event::Input(input) if input.id == FIELD => {
                DRAFT.with(|d| *d.borrow_mut() = Some(input.value));
                ui::render(ui::Slot::Settings, &page());
            }
            ui_types::Event::Submit(id) if id == FIELD => save(),
            ui_types::Event::Click(id) if id == "save" => save(),
            ui_types::Event::Click(id) if id == "test" => post(TEST_MESSAGE),
            ui_types::Event::Key(key) => {
                if let Some(json) = key.strip_prefix(TASK_BACK) {
                    match message_for(json) {
                        Some(text) => post(&text),
                        None => log::log(log::Level::Warn, "task-back event not understood"),
                    }
                }
            }
            _ => {}
        }
    }
}

/// Save writes the draft to the store (an empty draft forgets the URL) and redraws.
fn save() {
    let draft = DRAFT.with(|d| d.borrow_mut().take());
    let Some(draft) = draft else {
        return;
    };
    let draft = draft.trim();
    if draft.is_empty() {
        kv::delete(URL_KEY);
    } else {
        kv::set(URL_KEY, draft.as_bytes());
    }
    ui::render(ui::Slot::Settings, &page());
}

fn stored_url() -> Option<String> {
    kv::get(URL_KEY)
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .filter(|url| !url.is_empty())
}

/// One message to the webhook; the outcome is logged by status, never by URL.
fn post(text: &str) {
    let Some(url) = stored_url() else {
        if !WARNED_EMPTY.with(|w| w.replace(true)) {
            log::log(
                log::Level::Warn,
                "no webhook URL is configured on the Slack page; nothing was sent",
            );
        }
        return;
    };
    match net::post(&url, CONTENT_TYPE, body(text).as_bytes()) {
        Ok(status) if (200..300).contains(&status) => {
            log::log(log::Level::Info, &format!("webhook answered {status}"))
        }
        Ok(status) => log::log(
            log::Level::Warn,
            &format!("webhook answered {status}; the message was not accepted"),
        ),
        Err(why) => log::log(log::Level::Warn, &format!("webhook not reached: {why}")),
    }
}

// ---- the words ------------------------------------------------------------------

pub const TEST_MESSAGE: &str = "🕊️ Done. can reach this channel.";

/// The line for a Task that came back, from the desk's payload
/// `{ "key", "title", "project", "outcome", "by", "rounds" }`; None when the payload
/// is not the shape this plugin knows.
pub fn message_for(json: &str) -> Option<String> {
    let key = field(json, "key")?;
    let title = field(json, "title")?;
    let by = field(json, "by").unwrap_or_default();
    Some(match field(json, "outcome")?.as_str() {
        "done" => format!("🕊️ *{key} is back* — {title} · {by} handed it back done."),
        "stuck" => format!(
            "🪨 *{key} is stuck* — {title} · {by}'s session ended without handing back; it is in your hands."
        ),
        "capped" => {
            let rounds = field(json, "rounds").unwrap_or_else(|| "0".into());
            format!(
                "🛑 *{key} hit the cap* — {title} · {rounds} rounds; the loop stops here until you say so."
            )
        }
        _ => return None,
    })
}

/// The webhook body: `{"text": "<escaped>"}`.
pub fn body(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 12);
    out.push_str("{\"text\":\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push_str("\"}");
    out
}

/// One top-level field of a flat JSON object, a string (unescaped) or a number (as
/// written); enough for the desk's payload without a JSON crate in the guest.
pub fn field(json: &str, name: &str) -> Option<String> {
    let needle = format!("\"{name}\"");
    let mut rest = json;
    loop {
        let at = rest.find(&needle)?;
        let after = rest[at + needle.len()..].trim_start();
        // A value that merely contains the name is skipped: a key is followed by a colon.
        let Some(after) = after.strip_prefix(':') else {
            rest = &rest[at + needle.len()..];
            continue;
        };
        let after = after.trim_start();
        return Some(if let Some(quoted) = after.strip_prefix('"') {
            unescape(quoted)
        } else {
            after
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '-')
                .collect()
        });
    }
}

/// Reads a JSON string body up to its closing quote, resolving the escapes.
fn unescape(quoted: &str) -> String {
    let mut out = String::new();
    let mut chars = quoted.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => break,
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('b') => out.push('\u{8}'),
                Some('f') => out.push('\u{c}'),
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    if let Some(ch) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                        out.push(ch);
                    }
                }
                Some(other) => out.push(other),
                None => break,
            },
            c => out.push(c),
        }
    }
    out
}

/// The URL as the page shows it: the scheme and host, then `/…/[redacted]`.
pub fn masked(url: &str) -> String {
    let end = url
        .find("://")
        .map(|i| i + 3)
        .and_then(|from| url[from..].find('/').map(|i| from + i))
        .unwrap_or(url.len());
    format!("{}/…/[redacted]", &url[..end])
}

// ---- the page -------------------------------------------------------------------

/// The plugin's page in Plugins: the field, Save, the test verb, the masked value.
fn page() -> ui_types::Tree {
    let draft = DRAFT.with(|d| d.borrow().clone());
    let current = match stored_url() {
        Some(url) => format!("Webhook: {}", masked(&url)),
        None => "No webhook configured.".into(),
    };
    let mut nodes = vec![
        ui_types::Node::Column(ui_types::Container {
            children: vec![1, 2, 3, 4, 7],
            gap: 8,
        }),
        text(
            "A message to a Slack webhook when a Task comes back to you: done, stuck or capped.",
            None,
        ),
        text("Webhook URL", Some(ui_types::Tone::Dim)),
        ui_types::Node::Input(ui_types::Input {
            id: FIELD.into(),
            value: draft.unwrap_or_default(),
            placeholder: "https://hooks.slack.com/services/…".into(),
        }),
        ui_types::Node::Row(ui_types::Container {
            children: vec![5, 6],
            gap: 8,
        }),
        button("save", "Save", Some(ui_types::Tone::Accent)),
        button("test", "Send a test message", None),
    ];
    nodes.push(text(&current, Some(ui_types::Tone::Dim)));
    ui_types::Tree { root: 0, nodes }
}

fn text(words: &str, tone: Option<ui_types::Tone>) -> ui_types::Node {
    ui_types::Node::Text(vec![ui_types::Span {
        text: words.into(),
        bold: false,
        italic: false,
        code: false,
        tone,
        link: None,
    }])
}

fn button(id: &str, label: &str, tone: Option<ui_types::Tone>) -> ui_types::Node {
    ui_types::Node::Button(ui_types::Button {
        id: id.into(),
        label: label.into(),
        tone,
    })
}

export!(Slack);

#[cfg(test)]
mod tests {
    use super::*;

    const DONE: &str = r#"{"key":"lft3","title":"Wire the \"Slack\" page","project":"lft","outcome":"done","by":"Sol","rounds":2}"#;

    #[test]
    fn the_three_lines_read_as_written() {
        assert_eq!(
            message_for(DONE).unwrap(),
            "🕊️ *lft3 is back* — Wire the \"Slack\" page · Sol handed it back done."
        );
        let stuck = DONE.replace("\"done\"", "\"stuck\"");
        assert_eq!(
            message_for(&stuck).unwrap(),
            "🪨 *lft3 is stuck* — Wire the \"Slack\" page · Sol's session ended without handing back; it is in your hands."
        );
        let capped = DONE
            .replace("\"done\"", "\"capped\"")
            .replace("\"rounds\":2", "\"rounds\":3");
        assert_eq!(
            message_for(&capped).unwrap(),
            "🛑 *lft3 hit the cap* — Wire the \"Slack\" page · 3 rounds; the loop stops here until you say so."
        );
        assert_eq!(TEST_MESSAGE, "🕊️ Done. can reach this channel.");
    }

    #[test]
    fn an_unknown_outcome_or_a_missing_key_is_no_message() {
        assert!(message_for(&DONE.replace("\"done\"", "\"waiting\"")).is_none());
        assert!(message_for(r#"{"title":"x","outcome":"done"}"#).is_none());
        assert!(message_for("not json").is_none());
    }

    #[test]
    fn the_body_escapes_what_slack_must_not_see_raw() {
        assert_eq!(
            body("a \"quoted\" line\\ and\na tab\t"),
            r#"{"text":"a \"quoted\" line\\ and\na tab\t"}"#
        );
    }

    #[test]
    fn a_field_is_read_by_its_key_not_by_a_value_that_names_it() {
        let json = r#"{"title":"the key is here","key":"stg23","rounds":12,"by":"Astra"}"#;
        assert_eq!(field(json, "key").unwrap(), "stg23");
        assert_eq!(field(json, "rounds").unwrap(), "12");
        assert_eq!(field(json, "by").unwrap(), "Astra");
        assert_eq!(field(r#"{"t":"café \n"}"#, "t").unwrap(), "café \n");
        assert!(field(json, "outcome").is_none());
    }

    #[test]
    fn the_masked_url_keeps_the_host_and_hides_the_path() {
        assert_eq!(
            masked("https://hooks.slack.com/services/T0/B0/secret"),
            "https://hooks.slack.com/…/[redacted]"
        );
        assert_eq!(
            masked("http://127.0.0.1:4321/hook"),
            "http://127.0.0.1:4321/…/[redacted]"
        );
        assert_eq!(
            masked("https://hooks.slack.com"),
            "https://hooks.slack.com/…/[redacted]"
        );
    }
}
