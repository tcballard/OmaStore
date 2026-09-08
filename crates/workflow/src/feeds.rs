//! Release feeds are generated from observed delivery events, not approval or PR state.
use crate::{Error, Result, Store};
use omastore_catalogue::Catalogue;
use rusqlite::{params, Transaction};
use serde_json::Value;

pub(crate) fn delivered(
    t: &Transaction<'_>,
    revision: &str,
    payload: &Catalogue,
    now: i64,
) -> Result<()> {
    for app in &payload.apps {
        let release = app.current_release();
        let guid = format!("urn:omastore:release:{}:{}", app.id, release.id);
        t.execute("INSERT INTO feed_entries VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(app_id,release_id) DO UPDATE SET name=excluded.name,summary=excluded.summary,version=excluded.version,maker_ids=excluded.maker_ids,latest_revision=excluded.latest_revision",params![guid,app.id,release.id,app.name,app.summary,release.version,serde_json::to_string(&app.maker_ids)?,now,revision])?;
    }
    Ok(())
}
fn xml(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if c == '\t'
                || c == '\n'
                || c == '\r'
                || (c >= '\u{20}' && c != '\u{fffe}' && c != '\u{ffff}') =>
            {
                out.push(c)
            }
            _ => {}
        }
    }
    out
}
impl Store {
    pub fn release_feed(&self, maker: Option<&str>) -> Result<String> {
        if maker.is_some_and(|m| !omastore_catalogue::token(m)) {
            return Err(Error::new(422, "invalid_maker_id"));
        }
        let c = self.connection()?;
        let mut statement=c.prepare("SELECT guid,app_id,name,summary,version,maker_ids,first_delivered_at FROM feed_entries WHERE (?1 IS NULL OR EXISTS(SELECT 1 FROM json_each(maker_ids) WHERE value=?1)) ORDER BY first_delivered_at DESC,guid LIMIT 100")?;
        let rows = statement
            .query_map([maker], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, i64>(6)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut out=String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?><rss version=\"2.0\"><channel><title>OmaStore releases</title><link>https://github.com/tcballard/OmaStore</link><description>Observed catalogue releases. Compatibility and current distribution status remain separate.</description>");
        let mut count = 0;
        for (guid, id, name, summary, version, makers, at) in rows {
            let makers: Vec<String> = serde_json::from_str(&makers)?;
            if maker.is_some_and(|m| !makers.iter().any(|id| id == m)) {
                continue;
            }
            if count >= 100 {
                break;
            }
            count += 1;
            let at = chrono::DateTime::from_timestamp(at, 0)
                .ok_or(Error::new(500, "stored_feed_invalid"))?
                .to_rfc2822();
            out.push_str(&format!("<item><title>{}</title><link>omastore://app/{}</link><guid isPermaLink=\"false\">{}</guid><pubDate>{}</pubDate><description>{}</description></item>",xml(&format!("{name} · {version}")),xml(&id),xml(&guid),xml(&at),xml(&summary)));
        }
        out.push_str("</channel></rss>");
        Ok(out)
    }
    pub fn feed_preview(&self, maker: Option<&str>) -> Result<Value> {
        Ok(
            serde_json::json!({"xml":self.release_feed(maker)?,"notice":"Only publicly observed delivery events appear in this release feed."}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn feed_escaping_and_empty_delivery_history_are_safe() {
        assert_eq!(
            xml("A & B <test> \"quoted\"\u{0}"),
            "A &amp; B &lt;test&gt; &quot;quoted&quot;"
        );
        assert!(!Store::memory()
            .unwrap()
            .release_feed(None)
            .unwrap()
            .contains("<item>"));
    }
}
