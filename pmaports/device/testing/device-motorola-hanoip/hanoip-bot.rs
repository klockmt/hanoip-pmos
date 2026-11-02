// Telegram bot for the phone. It answers only the owner's private chat, with fixed commands: /up (the
// phone is alive), /status (hanoip-status), /status -h (what each status line means), /status -t=MIN -g=MIN
// (hanoip-status every g minutes for t minutes), /stop (ends that) and /notes (lists these). Nothing from a message reaches a
// shell; -t and -g must be numbers.
// `hanoip-bot --alert TEXT` sends TEXT to the owner once and exits; hanoip-thermal uses it (the text comes
// from the phone's own scripts, never from a message).
// It uses long polling, so it only makes outgoing connections; the firewall stays closed to incoming ones.
// HTTPS: the standard library has no TLS, so the requests go through curl(1). The URL (which holds the token)
// and the form fields are handed to curl as a config file on its standard input, so the token never shows in
// an argument list or in ps.
// Built with rustc alone (no cargo, no crates); scripts/test-bot runs the tests at the bottom.
use std::collections::HashMap;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const MAX_TEXT: usize = 4000; // Telegram refuses messages over 4096 characters
const HELP: &str = "comandos: /up, /status, /status -h, /status -t=30 -g=1, /stop, /notes";
const NOTES: &str = "comandos do hanoip-bot:
/up - responde se o celular está de pé
/status - o estado do celular (latência, rede, CPU, GPU, RAM, disco, bateria, consumo, temperatura)
/status -h - o que cada linha do status significa e de onde vem
/status -t=MIN -g=MIN - repete o status a cada g minutos durante t minutos (g de 1 a 60, t de g a 1440; sem -t são 30, sem -g é 1)
/stop - para o envio automático
/notes - esta lista
Só o dono responde, e só neste chat.";
const USAGE: &str = "uso: /status -t=MIN -g=MIN (-g de 1 a 60, -t de g a 1440)";
const POLL_S: u64 = 50;
const STATUS_TIMEOUT: Duration = Duration::from_secs(30);
const CURL_ARGS: [&str; 2] = ["-K", "-"]; // read the config from stdin: nothing sensitive in the arguments
const MAX_DEPTH: usize = 64;

// ---- a small JSON reader, enough for what the Telegram API returns ----

#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(o) => o.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    fn as_i64(&self) -> Option<i64> {
        match self {
            Json::Int(n) => Some(*n),
            _ => None,
        }
    }
    fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }
    fn as_arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(a) => Some(a),
            _ => None,
        }
    }
}

struct Reader<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Reader<'a> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }
    fn eat(&mut self, c: u8) -> Result<(), String> {
        if self.b.get(self.i) == Some(&c) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("expected '{}' at {}", c as char, self.i))
        }
    }
    fn lit(&mut self, word: &str, v: Json) -> Result<Json, String> {
        if self.b[self.i..].starts_with(word.as_bytes()) {
            self.i += word.len();
            Ok(v)
        } else {
            Err(format!("bad literal at {}", self.i))
        }
    }
    fn value(&mut self, depth: usize) -> Result<Json, String> {
        if depth > MAX_DEPTH {
            return Err("nested too deep".into());
        }
        self.ws();
        match self.b.get(self.i) {
            None => Err("unexpected end".into()),
            Some(b'{') => {
                self.i += 1;
                let mut o = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(Json::Obj(o));
                }
                loop {
                    self.ws();
                    let k = self.string()?;
                    self.ws();
                    self.eat(b':')?;
                    o.push((k, self.value(depth + 1)?));
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(Json::Obj(o));
                        }
                        _ => return Err(format!("expected ',' or '}}' at {}", self.i)),
                    }
                }
            }
            Some(b'[') => {
                self.i += 1;
                let mut a = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(Json::Arr(a));
                }
                loop {
                    a.push(self.value(depth + 1)?);
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Json::Arr(a));
                        }
                        _ => return Err(format!("expected ',' or ']' at {}", self.i)),
                    }
                }
            }
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b't') => self.lit("true", Json::Bool(true)),
            Some(b'f') => self.lit("false", Json::Bool(false)),
            Some(b'n') => self.lit("null", Json::Null),
            Some(c) if *c == b'-' || c.is_ascii_digit() => self.number(),
            Some(_) => Err(format!("unexpected byte at {}", self.i)),
        }
    }
    fn number(&mut self) -> Result<Json, String> {
        let start = self.i;
        while self.i < self.b.len() && matches!(self.b[self.i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
            self.i += 1;
        }
        let s = std::str::from_utf8(&self.b[start..self.i]).map_err(|e| e.to_string())?;
        if let Ok(n) = s.parse::<i64>() {
            Ok(Json::Int(n))
        } else {
            s.parse::<f64>().map(Json::Float).map_err(|_| format!("bad number at {}", start))
        }
    }
    fn hex4(&mut self) -> Result<u32, String> {
        let h = self.b.get(self.i..self.i + 4).ok_or("short \\u escape")?;
        let s = std::str::from_utf8(h).map_err(|e| e.to_string())?;
        self.i += 4;
        u32::from_str_radix(s, 16).map_err(|_| "bad \\u escape".to_string())
    }
    fn string(&mut self) -> Result<String, String> {
        self.eat(b'"')?;
        let mut out = Vec::new();
        loop {
            let c = *self.b.get(self.i).ok_or("unterminated string")?;
            self.i += 1;
            match c {
                b'"' => return String::from_utf8(out).map_err(|e| e.to_string()),
                b'\\' => {
                    let e = *self.b.get(self.i).ok_or("unterminated escape")?;
                    self.i += 1;
                    let ch = match e {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let mut n = self.hex4()?;
                            if (0xD800..0xDC00).contains(&n) {
                                // a high surrogate must be followed by \uDC00-\uDFFF
                                if self.b.get(self.i) == Some(&b'\\') && self.b.get(self.i + 1) == Some(&b'u') {
                                    self.i += 2;
                                    let lo = self.hex4()?;
                                    if !(0xDC00..0xE000).contains(&lo) {
                                        return Err("bad surrogate pair".into());
                                    }
                                    n = 0x10000 + ((n - 0xD800) << 10) + (lo - 0xDC00);
                                } else {
                                    return Err("lone surrogate".into());
                                }
                            }
                            char::from_u32(n).ok_or("bad code point")?
                        }
                        _ => return Err("bad escape".into()),
                    };
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                _ => out.push(c),
            }
        }
    }
}

fn parse_json(s: &str) -> Result<Json, String> {
    let mut r = Reader { b: s.as_bytes(), i: 0 };
    let v = r.value(0)?;
    r.ws();
    if r.i != r.b.len() {
        return Err(format!("trailing data at {}", r.i));
    }
    Ok(v)
}

// ---- Telegram ----

/// A curl config file for one request: the URL, the limit and the form fields (curl url-encodes them).
fn curl_config(url: &str, params: &[(&str, String)], max_time: u64) -> String {
    let mut c = String::from("silent\nshow-error\nfail\n");
    c.push_str(&format!("url = {}\n", curl_quote(url)));
    c.push_str(&format!("max-time = {}\n", max_time));
    for (k, v) in params {
        c.push_str(&format!("data-urlencode = {}\n", curl_quote(&format!("{k}={v}"))));
    }
    c
}

/// A double-quoted curl config string. Inside quotes curl understands \\ \" \t \n \r \v; other control
/// characters are replaced by a space.
fn curl_quote(s: &str) -> String {
    let mut q = String::from("\"");
    for ch in s.chars() {
        match ch {
            '\\' => q.push_str("\\\\"),
            '"' => q.push_str("\\\""),
            '\n' => q.push_str("\\n"),
            '\r' => q.push_str("\\r"),
            '\t' => q.push_str("\\t"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => q.push(' '),
            c => q.push(c),
        }
    }
    q.push('"');
    q
}

fn call(token: &str, method: &str, params: &[(&str, String)]) -> Result<Json, String> {
    let api = std::env::var("HANOIP_API").unwrap_or_else(|_| "https://api.telegram.org".into()); // the tests point it at a local server
    let url = format!("{api}/bot{token}/{method}");
    let curl = std::env::var("HANOIP_CURL").unwrap_or_else(|_| "curl".into());
    let mut child = Command::new(curl)
        .args(CURL_ARGS)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("{:?}", e.kind()))?;
    child
        .stdin
        .take()
        .ok_or("no stdin")?
        .write_all(curl_config(&url, params, 70).as_bytes())
        .map_err(|e| format!("{:?}", e.kind()))?;
    let out = child.wait_with_output().map_err(|e| format!("{:?}", e.kind()))?;
    if !out.status.success() {
        return Err(format!("curl exit {}", out.status.code().unwrap_or(-1))); // never the URL: it has the token
    }
    let body = parse_json(&String::from_utf8_lossy(&out.stdout))?;
    if body.get("ok") != Some(&Json::Bool(true)) {
        return Err("telegram answered not ok".into());
    }
    body.get("result").cloned().ok_or_else(|| "no result".into())
}

fn load_env(path: &str) -> std::io::Result<HashMap<String, String>> {
    let text = std::fs::read_to_string(path)?;
    let mut conf = HashMap::new();
    for line in text.lines() {
        if let Some((k, v)) = line.split_once('=') {
            conf.insert(k.to_string(), v.to_string());
        }
    }
    Ok(conf)
}

fn cut(s: &str) -> String {
    s.chars().take(MAX_TEXT).collect()
}

/// Runs the status command with fixed arguments (never taken from a message) and gives its text.
fn run_status(cmd: &str, args: &[&str]) -> String {
    let mut child = match Command::new(cmd).args(args).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
        Ok(c) => c,
        Err(e) => return format!("erro ao rodar hanoip-status: {:?}", e.kind()),
    };
    let reader = |mut s: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut b = Vec::new();
            let _ = s.read_to_end(&mut b);
            b
        })
    };
    let out_t = reader(Box::new(child.stdout.take().unwrap()));
    let err_t = reader(Box::new(child.stderr.take().unwrap()));
    let deadline = Instant::now() + STATUS_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return "erro ao rodar hanoip-status: TimedOut".into();
            }
            Err(e) => return format!("erro ao rodar hanoip-status: {:?}", e.kind()),
        }
    }
    let (out, err) = (out_t.join().unwrap_or_default(), err_t.join().unwrap_or_default());
    let (out, err) = (String::from_utf8_lossy(&out).into_owned(), String::from_utf8_lossy(&err).into_owned());
    cut(if !out.is_empty() {
        &out
    } else if !err.is_empty() {
        &err
    } else {
        "sem saída"
    })
}

/// '-t=30 -g=1' -> (30, 1), in minutes; None when anything is off. A missing -t is 30, a missing -g is 1.
fn parse_watch(args: &[&str]) -> Option<(u64, u64)> {
    let (mut t, mut g) = (None, None);
    for arg in args {
        let (key, value) = arg.split_once('=')?;
        let slot = match key {
            "-t" => &mut t,
            "-g" => &mut g,
            _ => return None,
        };
        if slot.is_some() || value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        *slot = Some(value.parse::<u64>().ok()?);
    }
    let (t, g) = (t.unwrap_or(30), g.unwrap_or(1));
    if (1..=60).contains(&g) && g <= t && t <= 1440 {
        Some((t, g))
    } else {
        None
    }
}

/// What to do with the automatic status after a message.
#[derive(Debug, PartialEq, Clone, Copy)]
enum Action {
    Nothing,
    Stop,
    Watch(u64, u64), // minutes, gap in minutes
}

struct Msg {
    from: Option<i64>,
    chat: Option<i64>,
    text: String,
}

fn msg_of(v: &Json) -> Msg {
    let id = |k: &str| v.get(k).and_then(|o| o.get("id")).and_then(Json::as_i64);
    Msg { from: id("from"), chat: id("chat"), text: v.get("text").and_then(Json::as_str).unwrap_or("").to_string() }
}

/// The text to send now (None ignores the message) and what to do with the automatic status.
fn reply_for(msg: &Msg, owner: i64, run: &dyn Fn(&[&str]) -> String) -> (Option<String>, Action) {
    if msg.from != Some(owner) || msg.chat != Some(owner) {
        return (None, Action::Nothing);
    }
    let words: Vec<&str> = msg.text.split_whitespace().collect();
    let command = words.first().map(|w| w.split('@').next().unwrap_or("")).unwrap_or(""); // /status@botname
    match command {
        "/up" => (Some("de pé".into()), Action::Nothing),
        "/stop" => (Some("envio automático parado".into()), Action::Stop),
        "/notes" => (Some(NOTES.into()), Action::Nothing),
        "/status" => {
            if words.len() == 1 {
                return (Some(run(&[])), Action::Nothing);
            }
            if words[1..] == ["-h"] {
                return (Some(run(&["-h"])), Action::Nothing);
            }
            match parse_watch(&words[1..]) {
                None => (Some(USAGE.into()), Action::Nothing),
                Some((t, g)) => (
                    Some(format!("enviando /status a cada {g} min por {t} min (/stop para parar)\n\n{}", run(&[]))),
                    Action::Watch(t, g),
                ),
            }
        }
        _ => (Some(HELP.into()), Action::Nothing),
    }
}

/// An automatic status as (next send, gap, end), in seconds of the monotonic clock.
type Watch = (f64, f64, f64);

fn start_watch(minutes: u64, gap_minutes: u64, now: f64) -> Watch {
    (now + gap_minutes as f64 * 60.0, gap_minutes as f64 * 60.0, now + minutes as f64 * 60.0)
}

/// Returns (send now, watch afterwards, last send). Sends missed while offline are skipped, not burst.
fn due(watch: Option<Watch>, now: f64) -> (bool, Option<Watch>, bool) {
    let Some((mut next, gap, end)) = watch else { return (false, None, false) };
    if now < next {
        return (false, watch, false);
    }
    while next <= now {
        next += gap;
    }
    if next > end {
        (true, None, true)
    } else {
        (true, Some((next, gap, end)), false)
    }
}

fn send(token: &str, owner: i64, text: &str) {
    if let Err(e) = call(token, "sendMessage", &[("chat_id", owner.to_string()), ("text", text.to_string())]) {
        eprintln!("sendMessage failed: {e}");
    }
}

fn settings() -> (String, i64) {
    let path = std::env::var("HANOIP_BOT_ENV").unwrap_or_else(|_| "/etc/hanoip/telegram.env".into());
    let conf = load_env(&path).unwrap_or_else(|e| panic!("{path}: {:?}", e.kind()));
    let token = conf.get("TOKEN").unwrap_or_else(|| panic!("{path}: no TOKEN")).clone();
    let owner = conf.get("OWNER_ID").and_then(|o| o.trim().parse().ok()).unwrap_or_else(|| panic!("{path}: no OWNER_ID"));
    (token, owner)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (token, owner) = settings();
    if args.len() == 2 && args[0] == "--alert" {
        send(&token, owner, &cut(&args[1]));
        return;
    }
    let status = std::env::var("HANOIP_STATUS_CMD").unwrap_or_else(|_| "/usr/bin/hanoip-status".into());
    let run = |a: &[&str]| run_status(&status, a);
    let t0 = Instant::now();
    let mut offset: i64 = 0;
    let mut watch: Option<Watch> = None;
    loop {
        let wait = match watch {
            None => POLL_S,
            Some((next, _, _)) => ((next - t0.elapsed().as_secs_f64()) as i64).clamp(0, POLL_S as i64) as u64,
        };
        let updates = match call(&token, "getUpdates", &[("offset", offset.to_string()), ("timeout", wait.to_string())]) {
            Ok(u) => u,
            Err(e) => {
                // the network can be down for a while
                eprintln!("getUpdates failed: {e}");
                thread::sleep(Duration::from_secs(10));
                Json::Arr(vec![])
            }
        };
        for update in updates.as_arr().unwrap_or(&[]) {
            if let Some(id) = update.get("update_id").and_then(Json::as_i64) {
                offset = id + 1;
            }
            let Some(m) = update.get("message") else { continue };
            let (text, action) = reply_for(&msg_of(m), owner, &run);
            let Some(text) = text else { continue };
            match action {
                Action::Stop => watch = None,
                Action::Watch(t, g) => watch = Some(start_watch(t, g, t0.elapsed().as_secs_f64())),
                Action::Nothing => {}
            }
            send(&token, owner, &text);
        }
        let (now_due, after, last) = due(watch, t0.elapsed().as_secs_f64());
        watch = after;
        if now_due {
            let tail = if last { "\n\nfim do envio automático" } else { "" };
            send(&token, owner, &(run(&[]) + tail));
        }
    }
}

// The checks the Python bot had, plus the JSON reader and the curl config.
#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: i64 = 1234;

    fn msg(text: &str, from: i64, chat: i64) -> Msg {
        Msg { from: Some(from), chat: Some(chat), text: text.to_string() }
    }
    fn owner_msg(text: &str) -> Msg {
        msg(text, OWNER, OWNER)
    }
    /// A status runner that counts its calls and tells a plain run from -h.
    fn fake(calls: &std::cell::Cell<u32>) -> impl Fn(&[&str]) -> String + '_ {
        move |a: &[&str]| {
            calls.set(calls.get() + 1);
            if a == ["-h"] { "help ok".into() } else { "status ok".into() }
        }
    }

    #[test]
    fn who_is_answered() {
        let calls = std::cell::Cell::new(0);
        let run = fake(&calls);
        assert_eq!(reply_for(&owner_msg("/up"), OWNER, &run), (Some("de pé".into()), Action::Nothing));
        assert_eq!(reply_for(&owner_msg("/status"), OWNER, &run), (Some("status ok".into()), Action::Nothing));
        assert_eq!(reply_for(&owner_msg("/status@hanoip_bot"), OWNER, &run), (Some("status ok".into()), Action::Nothing), "accepts /status@botname");
        assert_eq!(reply_for(&msg("/status", 99, OWNER), OWNER, &run), (None, Action::Nothing), "stranger ignored");
        assert_eq!(reply_for(&msg("/status", OWNER, -100), OWNER, &run), (None, Action::Nothing), "group chat ignored");
        assert_eq!(reply_for(&msg("/status -t=30 -g=1", 99, OWNER), OWNER, &run), (None, Action::Nothing), "a stranger cannot start a watch");
        assert_eq!(reply_for(&Msg { from: None, chat: Some(OWNER), text: "/up".into() }, OWNER, &run), (None, Action::Nothing), "no sender");
        assert_eq!(reply_for(&owner_msg("; rm -rf /"), OWNER, &run), (Some(HELP.into()), Action::Nothing), "unknown text gets the help");
        assert_eq!(reply_for(&owner_msg(""), OWNER, &run), (Some(HELP.into()), Action::Nothing), "empty text gets the help");
        assert_eq!(calls.get(), 2, "only the two /status ran the command");
    }

    // /notes lists the commands, so the owner does not have to remember them.
    #[test]
    fn notes_lists_every_command() {
        let calls = std::cell::Cell::new(0);
        let run = fake(&calls);
        let (text, action) = reply_for(&owner_msg("/notes"), OWNER, &run);
        assert_eq!(action, Action::Nothing);
        let text = text.unwrap();
        for cmd in ["/up", "/status", "/status -h", "/status -t=", "-g=", "/stop", "/notes"] {
            assert!(text.contains(cmd), "{cmd} is missing from /notes: {text}");
        }
        assert!(text.chars().count() < MAX_TEXT, "fits in one message");
        assert_eq!(reply_for(&owner_msg("/notes@hanoip_bot"), OWNER, &run).0.as_deref(), Some(text.as_str()), "accepts /notes@botname");
        assert_eq!(reply_for(&msg("/notes", 99, OWNER), OWNER, &run), (None, Action::Nothing), "a stranger gets nothing");
        assert_eq!(calls.get(), 0, "/notes does not run the status");
        assert!(HELP.contains("/notes"), "the answer to unknown text points at /notes");
    }

    #[test]
    fn watch_commands() {
        let calls = std::cell::Cell::new(0);
        let run = fake(&calls);
        let (text, action) = reply_for(&owner_msg("/status -t=30 -g=1"), OWNER, &run);
        assert_eq!(action, Action::Watch(30, 1));
        let text = text.unwrap();
        assert!(text.ends_with("status ok") && text.contains("a cada 1 min por 30 min"));
        assert_eq!(reply_for(&owner_msg("/stop"), OWNER, &run), (Some("envio automático parado".into()), Action::Stop));
        assert_eq!(reply_for(&owner_msg("/status -h"), OWNER, &run), (Some("help ok".into()), Action::Nothing));
        assert_eq!(reply_for(&owner_msg("/status -h x"), OWNER, &run), (Some(USAGE.into()), Action::Nothing));
        assert_eq!(reply_for(&owner_msg("/status -t=abc"), OWNER, &run), (Some(USAGE.into()), Action::Nothing));
    }

    #[test]
    fn parse_the_watch_options() {
        assert_eq!(parse_watch(&["-g=5", "-t=60"]), Some((60, 5)));
        assert_eq!(parse_watch(&["-t=10"]), Some((10, 1)), "default gap");
        assert_eq!(parse_watch(&["-g=2"]), Some((30, 2)), "default time");
        for bad in [
            vec!["-t=0"], vec!["-g=0"], vec!["-g=61"], vec!["-t=1441"], vec!["-t=5", "-g=10"], vec!["-t=-1"],
            vec!["-t=1.5"], vec!["-t"], vec!["-t=30", "-t=30"], vec!["-x=1"], vec!["-t=²"], vec!["-t=30;reboot"],
            vec!["-t="], vec!["-t=99999999999999999999999"],
        ] {
            assert_eq!(parse_watch(&bad), None, "{bad:?}");
        }
    }

    // /status -t=3 -g=1 at t=0 sends now (in the reply), then at 1, 2 and 3 minutes, the last marked.
    #[test]
    fn schedule() {
        let mut w = Some(start_watch(3, 1, 0.0));
        assert_eq!(due(w, 59.0), (false, w, false), "not due before the gap");
        let mut sent = vec![];
        for now in (0..400).step_by(10) {
            let (go, after, last) = due(w, now as f64);
            w = after;
            if go {
                sent.push((now, last));
            }
        }
        assert_eq!(sent, vec![(60, false), (120, false), (180, true)]);
        assert_eq!(w, None, "nothing after the end");
        let (go, after, last) = due(Some(start_watch(10, 1, 0.0)), 300.0); // offline for 5 minutes: one send, no burst
        assert_eq!((go, after.map(|a| a.0), last), (true, Some(360.0), false));
        assert_eq!(due(None, 5.0), (false, None, false));
    }

    fn script(body: &str) -> String {
        use std::os::unix::fs::PermissionsExt;
        let p = std::env::temp_dir().join(format!("hanoip-bot-test-{}-{}", std::process::id(), body.len()));
        std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        p.to_string_lossy().into_owned()
    }

    #[test]
    fn the_status_runner() {
        let long = script("head -c 5000 /dev/zero | tr '\\0' a");
        assert_eq!(run_status(&long, &[]).chars().count(), MAX_TEXT, "long output is cut");
        let echo = script("echo \"$@\"");
        assert_eq!(run_status(&echo, &["-h"]), "-h\n", "-h reaches the command as a fixed argument");
        let err_only = script("echo oops >&2");
        assert_eq!(run_status(&err_only, &[]), "oops\n", "stderr when stdout is empty");
        let silent = script("true");
        assert_eq!(run_status(&silent, &[]), "sem saída");
        assert_eq!(run_status("/nonexistent/hanoip-status", &[]), "erro ao rodar hanoip-status: NotFound", "a missing command is text, not a crash");
        let accents = script("printf 'ação é ✓\\n'");
        assert_eq!(run_status(&accents, &[]), "ação é ✓\n");
        for p in [long, echo, err_only, silent, accents] {
            let _ = std::fs::remove_file(p);
        }
    }

    #[test]
    fn the_cut_is_by_characters() {
        assert_eq!(cut(&"é".repeat(5000)).chars().count(), MAX_TEXT);
        assert_eq!(cut("curto"), "curto");
    }

    #[test]
    fn json_reader() {
        let v = parse_json(r#"{"ok":true,"result":[{"update_id":123456789012,"message":{"from":{"id":1234},"chat":{"id":-100},"text":"/status -h"}}]}"#).unwrap();
        let u = &v.get("result").unwrap().as_arr().unwrap()[0];
        assert_eq!(u.get("update_id").and_then(Json::as_i64), Some(123456789012), "ids pass 32 bits");
        let m = msg_of(u.get("message").unwrap());
        assert_eq!((m.from, m.chat, m.text.as_str()), (Some(1234), Some(-100), "/status -h"));
        assert_eq!(parse_json(r#""a\u00e9\ud83d\ude00\n\"\\\/""#).unwrap(), Json::Str("aé😀\n\"\\/".into()));
        assert_eq!(parse_json(" [1, -2.5, null, true, false, \"x\"] ").unwrap(),
                   Json::Arr(vec![Json::Int(1), Json::Float(-2.5), Json::Null, Json::Bool(true), Json::Bool(false), Json::Str("x".into())]));
        assert_eq!(parse_json("{}").unwrap(), Json::Obj(vec![]));
        assert_eq!(parse_json("[]").unwrap(), Json::Arr(vec![]));
        let nomsg = parse_json(r#"{"update_id":5,"edited_message":{}}"#).unwrap();
        assert!(nomsg.get("message").is_none());
    }

    #[test]
    fn json_reader_refuses_bad_input() {
        for bad in ["", "{", "[1,]", "{\"a\"}", "{\"a\":1,}", "[1] x", "nul", "\"abc", "\"\\q\"", "\"\\ud83d\"", "\"\\ud83dx\"", "\"\\u12\"", "-", "{'a':1}"] {
            assert!(parse_json(bad).is_err(), "{bad:?}");
        }
        let deep = "[".repeat(10_000) + &"]".repeat(10_000);
        assert!(parse_json(&deep).is_err(), "deep nesting is refused, not a stack overflow");
    }

    #[test]
    fn curl_gets_its_config_on_stdin() {
        // the token is in the URL, so the arguments must not hold anything but "read the config from stdin"
        assert_eq!(CURL_ARGS, ["-K", "-"]);
        let c = curl_config("https://api.telegram.org/bot123:SECRET/sendMessage", &[("chat_id", "1234".into()), ("text", "olá \"a\"\\b\nc\t".into())], 70);
        assert!(c.contains("url = \"https://api.telegram.org/bot123:SECRET/sendMessage\"\n"));
        assert!(c.contains("max-time = 70\n"));
        assert!(c.contains("data-urlencode = \"chat_id=1234\"\n"));
        assert!(c.contains("data-urlencode = \"text=olá \\\"a\\\"\\\\b\\nc\\t\"\n"), "quotes, backslash, newline and tab are escaped: {c}");
        assert!(c.starts_with("silent\nshow-error\nfail\n"));
        assert_eq!(curl_quote("a\u{1}b\u{7f}"), "\"a b \"", "other control characters become spaces");
        assert_eq!(c.lines().filter(|l| l.starts_with("url")).count(), 1, "a newline in the text cannot add a config line");
    }

    #[test]
    fn a_text_cannot_add_curl_options() {
        // a text trying to start a new config line (to read a file or change the URL) stays inside its string
        let c = curl_config("https://x/bot1:T/m", &[("text", "x\"\nurl = \"https://evil/\nfile".into())], 70);
        assert_eq!(c.lines().filter(|l| l.starts_with("url")).count(), 1);
        assert_eq!(c.lines().count(), 3 + 1 + 1 + 1, "the config has the fixed lines plus url, max-time and one field");
    }
}
