use crate::{
    asset_store,
    portable_store::{safe_uint, Store, MAX_SAFE_INTEGER},
    public_https,
};
use reqwest::Url;
use scraper::{Html, Selector};
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};
use time::{
    format_description::well_known::{Rfc2822, Rfc3339},
    OffsetDateTime, UtcOffset,
};

const IDS: [&str; 3] = ["github", "yahoo", "rss"];
const URLS: [&str; 2] = [
    "https://github.com/trending?since=daily",
    "https://tw.news.yahoo.com/rss",
];
const INVALID: &str = "FEED_RESPONSE_INVALID";
const SETTINGS: &str = "daily-feed-settings.json";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_persists_success_and_restart_reuses_the_same_public_cache() {
        let dir = crate::presets::tests::TestStore::new();
        let core = dir.core();
        let store = &core.store;
        let feeds = Feeds::default();
        let mut options = defaults();
        options["cards"]["github"]["enabled"] = false.into();
        options["cards"]["yahoo"]["enabled"] = false.into();
        options["rss"]["enabled"] = true.into();
        options["rss"]["url"] = "https://example.org/feed".into();
        store.atomic(SETTINGS, &options).unwrap();
        let mock = |id: &str, source: &str| {
            assert_eq!((id, source), ("rss", "https://example.org/feed"));
            Ok(vec![
                json!({"title":"Saved","url":"https://example.org/story","description":"","dailyStars":"","publishedAt":null}),
            ])
        };
        assert_eq!(
            feeds.status_with(store, Some(mock)).unwrap()["feeds"]["rss"]["loading"],
            true
        );
        for _ in 0..100 {
            let state = feeds.status_with(store, Some(mock)).unwrap();
            if state["feeds"]["rss"]["loading"] == false {
                assert_eq!(state["feeds"]["rss"]["items"][0]["title"], "Saved");
                let restarted = Feeds::default()
                    .status_with(store, Some(|_, _| panic!("fresh data should not refresh")))
                    .unwrap();
                assert_eq!(
                    restarted["feeds"]["rss"]["items"],
                    state["feeds"]["rss"]["items"]
                );
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("worker did not settle");
    }

    #[test]
    fn parses_github_rows_and_yahoo_rss_with_legacy_item_shape() {
        let github: String = (1..=5).map(|i| format!(
            r#"<article class="Box-row"><h2><a href="/team/repo-{i}"> team / repo-{i} </a></h2><p>A &amp; B</p><span class="float-sm-right">42 stars today</span></article>"#
        )).collect();
        let items = parse("github", github.as_bytes(), &Url::parse(URLS[0]).unwrap()).unwrap();
        assert_eq!(
            items[0],
            json!({"title":"team/repo-1","url":"https://github.com/team/repo-1","description":"A & B","dailyStars":"42 stars today","publishedAt":null})
        );
        let rows: String = (1..=5).map(|i| format!(
            "<item><title><![CDATA[新聞 {i}]]></title><link>https://tw.news.yahoo.com/story-{i}.html</link><pubDate>Sun, 06 Nov 1994 08:49:3{i} GMT</pubDate></item>"
        )).collect();
        let xml = format!("<rss><channel>{rows}</channel></rss>");
        let items = parse("yahoo", xml.as_bytes(), &Url::parse(URLS[1]).unwrap()).unwrap();
        assert_eq!(items[0]["title"], "新聞 5");
        assert_eq!(items[0]["publishedAt"], "1994-11-06T08:49:35.000Z");
    }

    #[test]
    fn custom_rss_supports_atom_relative_urls_and_rejects_dtd_and_private_links() {
        let atom = r#"<feed xmlns="http://www.w3.org/2005/Atom" xml:base="https://example.org/posts/">
          <entry><title>Good</title><link rel="alternate" href="a#part"/><updated>2026-11-07T12:00:00Z</updated></entry>
          <entry><title>Private</title><link href="https://127.1/page"/></entry>
          <entry><title>Duplicate</title><link href="a#part"/></entry>
        </feed>"#;
        let url = Url::parse("https://example.org/atom").unwrap();
        let items = parse("rss", atom.as_bytes(), &url).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["url"], "https://example.org/posts/a#part");
        assert_eq!(items[0]["publishedAt"], "2026-11-07T12:00:00.000Z");
        assert_eq!(
            parse(
                "rss",
                br#"<!DOCTYPE rss [<!ENTITY injected "bad">]><rss><channel/></rss>"#,
                &url
            )
            .unwrap_err(),
            INVALID
        );
        assert!(fresh(0, Some(0), 57599999, 60));
        assert!(!fresh(0, Some(0), 57600000, 60));
        assert!(!fresh(2, Some(0), 900000, 15));
    }
}

fn exact(v: &Value, keys: &[&str]) -> bool {
    v.as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
}

fn defaults() -> Value {
    json!({"schemaVersion":1,"revision":0,"cards":{"github":{"enabled":true,"x":3,"y":58,"fontSize":16},
        "yahoo":{"enabled":true,"x":52,"y":58,"fontSize":16}},
        "rss":{"enabled":false,"url":"","title":"我的 RSS","x":52,"y":4,"fontSize":16,"intervalMinutes":60}})
}

fn card(v: &Value) -> bool {
    v["enabled"].is_boolean()
        && ["x", "y"]
            .iter()
            .all(|k| v[k].as_f64().is_some_and(|n| (0.0..=100.0).contains(&n)))
        && safe_uint(&v["fontSize"]).is_some_and(|n| (12..=28).contains(&n))
}

fn rss(v: &Value) -> bool {
    exact(
        v,
        &[
            "enabled",
            "url",
            "title",
            "x",
            "y",
            "fontSize",
            "intervalMinutes",
        ],
    ) && card(v)
        && v["title"].as_str().is_some_and(|s| {
            !s.trim().is_empty()
                && s.encode_utf16().count() <= 60
                && !s.chars().any(|c| c <= '\u{1f}')
        })
        && safe_uint(&v["intervalMinutes"]).is_some_and(|n| [15, 60, 360, 1440].contains(&n))
        && v["url"].as_str().is_some_and(|s| {
            if s.is_empty() {
                return v["enabled"] == false;
            }
            s.encode_utf16().count() <= 2048
                && s.starts_with("https://")
                && Url::parse(s).is_ok_and(|u| public_https::validate(&u).is_ok())
        })
}

fn valid_cards(v: &Value) -> bool {
    exact(v, &IDS[..2])
        && IDS[..2]
            .iter()
            .all(|id| exact(&v[id], &["enabled", "x", "y", "fontSize"]) && card(&v[id]))
}

fn settings(store: &Store) -> Result<Value, String> {
    let Some(mut v) = record(store, SETTINGS, 16384)? else {
        return Ok(defaults());
    };
    if !(exact(&v, &["schemaVersion", "revision", "cards"])
        || exact(&v, &["schemaVersion", "revision", "cards", "rss"]))
        || safe_uint(&v["schemaVersion"]) != Some(1)
        || safe_uint(&v["revision"]).is_none()
        || !valid_cards(&v["cards"])
        || v.get("rss").is_some_and(|c| !rss(c))
    {
        return Err("FEED_SETTINGS_INVALID".into());
    }
    if v.get("rss").is_none() {
        v["rss"] = defaults()["rss"].clone();
    }
    Ok(v)
}

fn record(store: &Store, name: &str, max: usize) -> Result<Option<Value>, String> {
    match asset_store::read_bytes(&store.path(name)?, max, "FEED_SETTINGS_INVALID") {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|_| "FEED_SETTINGS_INVALID".into()),
        Err(e) if e == "ENOENT" => Ok(None),
        Err(e) => Err(e),
    }
}

pub(crate) fn update(store: &Store, v: &Value) -> Result<Value, String> {
    if !(exact(v, &["schemaVersion", "expectedRevision", "cards"])
        || exact(v, &["schemaVersion", "expectedRevision", "cards", "rss"]))
        || safe_uint(&v["schemaVersion"]) != Some(1)
        || !valid_cards(&v["cards"])
        || v.get("rss").is_some_and(|c| !rss(c))
    {
        return Err("FEED_SETTINGS_INVALID".into());
    }
    let expected = safe_uint(&v["expectedRevision"]).ok_or("FEED_SETTINGS_INVALID")?;
    let _guard = store.lock()?;
    let old = settings(store)?;
    if safe_uint(&old["revision"]) != Some(expected) {
        return Err("FEED_REVISION_CONFLICT".into());
    }
    if expected >= MAX_SAFE_INTEGER {
        return Err("FEED_SETTINGS_INVALID".into());
    }
    let next = json!({"schemaVersion":1,"revision":expected+1,"cards":v["cards"],"rss":v.get("rss").unwrap_or(&old["rss"])});
    store.atomic(SETTINGS, &next)?;
    Ok(json!({"schemaVersion":1,"available":true,"settings":next}))
}

fn trim_units(text: &str, max: usize) -> String {
    let mut total = 0;
    text.chars()
        .take_while(|c| {
            total += c.len_utf16();
            total <= max
        })
        .collect()
}

fn clean(text: &str, max: usize) -> String {
    let replaced: String = text
        .chars()
        .map(|c| {
            if c <= '\u{1f}' || c == '\u{7f}' {
                ' '
            } else {
                c
            }
        })
        .collect();
    trim_units(
        &replaced.split_whitespace().collect::<Vec<_>>().join(" "),
        max,
    )
}

fn link(id: &str, url: Url) -> Option<String> {
    let mut target = url.clone();
    if id == "rss" {
        target.set_fragment(None);
    }
    public_https::validate(&target).ok()?;
    match id {
        "github" => {
            let pieces: Vec<_> = url.path_segments()?.collect();
            if url.host_str() != Some("github.com")
                || pieces.len() != 2
                || !pieces.iter().all(|s| {
                    !s.is_empty()
                        && s.bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
                })
            {
                return None;
            }
        }
        "yahoo"
            if url.host_str() != Some("tw.news.yahoo.com") || !url.path().ends_with(".html") =>
        {
            return None
        }
        _ => {}
    }
    Some(url.into())
}

fn date(s: &str) -> Option<OffsetDateTime> {
    let s = s.trim();
    OffsetDateTime::parse(s, &Rfc3339)
        .or_else(|_| OffsetDateTime::parse(s, &Rfc2822))
        .ok()
        .or_else(|| httpdate::parse_http_date(s).ok().map(Into::into))
}

fn iso(date: OffsetDateTime) -> String {
    let d = date.to_offset(UtcOffset::UTC);
    format!(
        "{}T{:02}:{:02}:{:02}.{:03}Z",
        d.date(),
        d.hour(),
        d.minute(),
        d.second(),
        d.millisecond()
    )
}

fn xml_items(id: &str, text: &str, source: &Url) -> Result<Vec<Value>, &'static str> {
    let doc = roxmltree::Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: 30000,
            ..Default::default()
        },
    )
    .map_err(|_| INVALID)?;
    let root = doc.root_element();
    let atom = root.has_tag_name("feed") && id == "rss";
    let channel = if atom {
        root
    } else if root.has_tag_name("rss") {
        child(root, "channel").ok_or(INVALID)?
    } else {
        return Err(INVALID);
    };
    let base = if let Some(base) = root.attribute(("http://www.w3.org/XML/1998/namespace", "base"))
    {
        source.join(base).map_err(|_| INVALID)?
    } else {
        source.clone()
    };
    let mut items = Vec::new();
    for entry in channel
        .children()
        .filter(|n| n.has_tag_name(if atom { "entry" } else { "item" }))
    {
        let text = |key| {
            child(entry, key)
                .map(|n| {
                    n.descendants()
                        .filter(|n| n.is_text())
                        .filter_map(|n| n.text())
                        .collect::<String>()
                })
                .unwrap_or_default()
        };
        let title = clean(&text("title"), 160);
        let raw = if atom {
            entry
                .children()
                .find(|l| {
                    l.has_tag_name("link") && l.attribute("rel").is_none_or(|r| r == "alternate")
                })
                .and_then(|l| l.attribute("href"))
                .unwrap_or("")
                .to_owned()
        } else {
            let s = text("link");
            if id == "rss"
                && s.trim().is_empty()
                && child(entry, "guid").is_some_and(|n| n.attribute("isPermaLink") != Some("false"))
            {
                text("guid")
            } else {
                s
            }
        };
        if title.is_empty() || raw.trim().is_empty() {
            continue;
        }
        let Some(url) = (if id == "rss" {
            base.join(raw.trim())
        } else {
            Url::parse(raw.trim())
        })
        .ok()
        .and_then(|u| link(id, u)) else {
            continue;
        };
        let when = if atom {
            if child(entry, "published").is_some() {
                text("published")
            } else {
                text("updated")
            }
        } else {
            text("pubDate")
        };
        let timestamp = date(&when);
        items.push((timestamp.map_or(0, |d| d.unix_timestamp_nanos()),json!({
            "title":title,"url":url,"description":"","dailyStars":"","publishedAt":timestamp.map(iso)
        })));
    }
    items.sort_by(|a, b| b.0.cmp(&a.0));
    Ok(items.into_iter().map(|(_, v)| v).collect())
}

fn child<'a, 'input>(
    e: roxmltree::Node<'a, 'input>,
    key: &str,
) -> Option<roxmltree::Node<'a, 'input>> {
    e.children().find(|n| n.has_tag_name(key))
}

fn github_items(text: &str) -> Vec<Value> {
    let doc = Html::parse_document(text);
    let row = Selector::parse("article.Box-row").unwrap();
    let title = Selector::parse("h2 a").unwrap();
    let description = Selector::parse("p").unwrap();
    let stars = Selector::parse(".float-sm-right").unwrap();
    doc.select(&row).filter_map(|article| {
        let a = article.select(&title).next()?;
        let href = a.attr("href")?;
        if !href.starts_with('/') || href.starts_with("//") { return None; }
        let url = link("github",Url::parse(&format!("https://github.com{href}")).ok()?)?;
        let title = clean(&a.text().collect::<String>(),100).split('/').map(str::trim).collect::<Vec<_>>().join("/");
        if title.is_empty() { return None; }
        let field = |s,max| article.select(s).next().map_or(String::new(),|e| clean(&e.text().collect::<String>(),max));
        Some(json!({"title":title,"url":url,"description":field(&description,180),"dailyStars":field(&stars,60),"publishedAt":null}))
    }).take(1000).collect()
}

fn parse(id: &str, bytes: &[u8], source: &Url) -> Result<Vec<Value>, &'static str> {
    let text = std::str::from_utf8(bytes).map_err(|_| INVALID)?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let items = if id == "github" {
        github_items(text)
    } else {
        xml_items(id, text, source)?
    };
    let mut seen = BTreeSet::new();
    let items: Vec<_> = items
        .into_iter()
        .filter(|v| seen.insert(v["url"].as_str().unwrap().to_owned()))
        .take(5)
        .collect();
    if id != "rss" && items.len() != 5 {
        return Err(INVALID);
    }
    Ok(items)
}

fn download(id: &str, source: &str) -> Result<Vec<Value>, &'static str> {
    let url = Url::parse(source).map_err(|_| "FEED_FETCH_FAILED")?;
    let (accept, types): (_, &[_]) = match id {
        "github" => ("text/html", &["text/html"]),
        "yahoo" => (
            "application/rss+xml, application/xml, text/xml",
            &["application/rss+xml", "application/xml", "text/xml"],
        ),
        _ => (
            "application/rss+xml, application/atom+xml, application/xml, text/xml",
            &[],
        ),
    };
    let bytes = public_https::get(
        url.clone(),
        &public_https::Fetch {
            accept,
            content_types: types,
            max_bytes: if id == "rss" { 1048576 } else { 2097152 },
            redirects: if id == "rss" { 3 } else { 0 },
            seconds: 12,
        },
    )
    .map_err(|e| match e {
        public_https::Error::Blocked if id == "rss" => "ICS_URL_BLOCKED",
        public_https::Error::Size | public_https::Error::Type => INVALID,
        _ => "FEED_FETCH_FAILED",
    })?;
    parse(id, &bytes, &url)
}

fn cache_key(url: &str) -> String {
    asset_store::hash_text(url)
}

fn cache(store: &Store, index: usize, source: &str, now: u64) -> Option<Value> {
    let id = IDS[index];
    let v = record(store, &format!("daily-feed-{id}.json"), 32768)
        .ok()
        .flatten()?;
    let n = safe_uint(&v["fetchedAtMs"])?;
    if v["schemaVersion"] != 1 || n > now || (id == "rss" && v["sourceKey"] != cache_key(source)) {
        return None;
    }
    let items = v["items"].as_array()?;
    if (id == "rss" && items.len() > 5)
        || (id != "rss" && items.len() != 5)
        || !items.iter().all(|i| {
            i["url"]
                .as_str()
                .and_then(|s| Url::parse(s).ok())
                .and_then(|u| link(id, u))
                .is_some()
                && i["title"].as_str().is_some_and(|s| s == clean(s, 160))
                && [("description", 180), ("dailyStars", 60)]
                    .iter()
                    .all(|(k, n)| {
                        i[k].as_str()
                            .is_some_and(|s| s.encode_utf16().count() <= *n)
                    })
                && (i["publishedAt"].is_null()
                    || i["publishedAt"].as_str().and_then(date).is_some())
        })
    {
        return None;
    }
    Some(v)
}

fn fresh(index: usize, at: Option<u64>, now: u64, minutes: u64) -> bool {
    let Some(at) = at.filter(|at| *at <= now) else {
        return false;
    };
    if index == 2 {
        now - at < minutes * 60000
    } else {
        (now + 28800000) / 86400000 == (at + 28800000) / 86400000
    }
}

#[derive(Default)]
struct Job {
    source: String,
    pending: bool,
    attempt: Option<u64>,
    error: Option<&'static str>,
}

#[derive(Default)]
pub(crate) struct Feeds(Arc<Mutex<[Job; 3]>>);

impl Feeds {
    pub(crate) fn status(&self, store: &Store, network: bool) -> Result<Value, String> {
        self.status_with(store, network.then_some(download))
    }

    fn status_with(
        &self,
        store: &Store,
        download: Option<fn(&str, &str) -> Result<Vec<Value>, &'static str>>,
    ) -> Result<Value, String> {
        let s = settings(store)?;
        let now = crate::journal::now_ms();
        let mut jobs = self.0.lock().map_err(|_| "FEED_FETCH_FAILED")?;
        let mut result = json!({});
        for (i, id) in IDS.into_iter().enumerate() {
            let (config, source) = if i == 2 {
                (&s["rss"], s["rss"]["url"].as_str().unwrap())
            } else {
                (&s["cards"][id], URLS[i])
            };
            let old = cache(store, i, source, now);
            let at = old.as_ref().and_then(|v| safe_uint(&v["fetchedAtMs"]));
            let fresh = fresh(
                i,
                at,
                now,
                safe_uint(&config["intervalMinutes"]).unwrap_or(60),
            );
            let job = &mut jobs[i];
            if job.source != source && !job.pending {
                *job = Job {
                    source: source.to_owned(),
                    ..Job::default()
                };
            }
            let enabled = config["enabled"] == true;
            if let Some(download) = download.filter(|_| {
                enabled
                    && !job.pending
                    && !fresh
                    && job
                        .attempt
                        .is_none_or(|at| now.checked_sub(at).is_none_or(|age| age >= 1800000))
            }) {
                job.attempt = Some(now);
                job.pending = true;
                let state = self.0.clone();
                let owner = Store {
                    root: store.root.clone(),
                };
                let source = source.to_owned();
                if std::thread::Builder::new().spawn(move || {
                    let downloaded = download(id,&source);
                    let current = || settings(&owner).is_ok_and(|s| i!=2 || s["rss"]["url"]==source);
                    let outcome = (|| {
                        let items = downloaded?;
                        let _guard=owner.lock().map_err(|_|"FEED_FETCH_FAILED")?;
                        if !current() {return Ok(());}
                        let mut v=json!({"schemaVersion":1,"items":items,"fetchedAtMs":crate::journal::now_ms()});
                        if i==2 {v["sourceKey"]=cache_key(&source).into();}
                        owner.atomic(&format!("daily-feed-{id}.json"),&v).map_err(|_|"FEED_FETCH_FAILED")
                    })();
                    if let Ok(mut jobs)=state.lock() {
                        if current() {jobs[i].error=outcome.err();}
                        jobs[i].pending=false;
                    }
                }).is_err() { job.pending=false; job.error=Some("FEED_FETCH_FAILED"); }
            }
            let same = job.source == source;
            result[id] = json!({"source":source,"enabled":enabled,"loading":same&&job.pending,
                "error":if same {job.error.map(|code|json!({"code":code}))}else{None},
                "stale":old.is_some()&&(!fresh||(same&&job.error.is_some())),"fetchedAtMs":at,
                "items":old.map_or(json!([]),|c|c["items"].clone())});
        }
        Ok(json!({"schemaVersion":1,"available":true,"settings":s,"feeds":result}))
    }
}
