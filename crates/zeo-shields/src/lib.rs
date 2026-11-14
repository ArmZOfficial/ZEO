//! ZEO Shields: engine-agnostic network ad/tracker blocking on top of adblock-rust.
//! Knows nothing about WebView2 or Servo; the embedder maps its request types to [`ResourceKind`].

use adblock::lists::{FilterSet, ParseOptions, RuleTypes};
use adblock::request::Request;
use adblock::Engine;

/// Request kinds the embedder can report. Names follow adblock-rust's request type strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceKind {
    Document,
    SubFrame,
    Script,
    Stylesheet,
    Image,
    Font,
    Media,
    XmlHttpRequest,
    WebSocket,
    Ping,
    Other,
}

impl ResourceKind {
    fn as_adblock_str(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::SubFrame => "sub_frame",
            Self::Script => "script",
            Self::Stylesheet => "stylesheet",
            Self::Image => "image",
            Self::Font => "font",
            Self::Media => "media",
            Self::XmlHttpRequest => "xmlhttprequest",
            Self::WebSocket => "websocket",
            Self::Ping => "ping",
            Self::Other => "other",
        }
    }
}

pub struct Shields {
    all: Engine,
    trackers: Engine,
}

/// Why a request was blocked, so the UI can count ads and trackers separately.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Ad,
    Tracker,
}

fn engine_from(lists: &[&str]) -> Engine {
    let mut set = FilterSet::new(false);
    for list in lists {
        // Network rules only: cosmetic rules aren't used yet, and dropping them saves engine memory.
        set.add_filter_list(
            (*list).to_string(),
            ParseOptions { rule_types: RuleTypes::NetworkOnly, ..Default::default() },
        );
    }
    Engine::new_with_filter_set(set)
}

impl Shields {
    /// `ad_lists` (e.g. EasyList) and `tracker_lists` (e.g. EasyPrivacy). One combined engine makes
    /// the block decision, so `@@` exceptions work across lists (adblock-rust only applies an
    /// exception when a block rule matched in the same engine). A second, tracker-only engine just
    /// labels what was blocked.
    pub fn new(ad_lists: &[&str], tracker_lists: &[&str]) -> Self {
        let all: Vec<&str> = ad_lists.iter().chain(tracker_lists).copied().collect();
        Self { all: engine_from(&all), trackers: engine_from(tracker_lists) }
    }

    /// `Some(category)` if the request should be blocked. `source_url` is the top-level page URL.
    /// Never blocks top-level documents, so a filter can't leave the user staring at a blank page.
    pub fn check(&self, url: &str, source_url: &str, kind: ResourceKind) -> Option<Category> {
        if kind == ResourceKind::Document {
            return None;
        }
        let req = Request::new(url, source_url, kind.as_adblock_str(), "GET").ok()?;
        if !self.all.check_network_request(&req).should_block() {
            return None;
        }
        let tracker = self.trackers.check_network_request(&req).filter.is_some();
        Some(if tracker { Category::Tracker } else { Category::Ad })
    }

    pub fn should_block(&self, url: &str, source_url: &str, kind: ResourceKind) -> bool {
        self.check(url, source_url, kind).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real_lists() -> Shields {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../resources/filters/");
        let el = std::fs::read_to_string(format!("{dir}easylist.txt")).unwrap();
        let ep = std::fs::read_to_string(format!("{dir}easyprivacy.txt")).unwrap();
        Shields::new(&[&el], &[&ep])
    }

    #[test]
    fn blocks_known_ads_and_trackers() {
        let s = real_lists();
        let page = "https://www.example-news.co.th/article/1";
        assert!(s.should_block(
            "https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js",
            page,
            ResourceKind::Script
        ));
        assert!(s.should_block(
            "https://securepubads.g.doubleclick.net/tag/js/gpt.js",
            page,
            ResourceKind::Script
        ));
        assert_eq!(
            s.check("https://www.google-analytics.com/analytics.js", page, ResourceKind::Script),
            Some(Category::Tracker)
        );
        assert_eq!(
            s.check("https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js", page, ResourceKind::Script),
            Some(Category::Ad)
        );
    }

    #[test]
    fn allows_normal_content() {
        let s = real_lists();
        let page = "https://en.wikipedia.org/wiki/Rust";
        assert!(!s.should_block(
            "https://upload.wikimedia.org/wikipedia/commons/a/a1/Example.jpg",
            page,
            ResourceKind::Image
        ));
        assert!(!s.should_block("https://en.wikipedia.org/w/load.php?x=1", page, ResourceKind::Script));
    }

    #[test]
    fn exception_in_one_list_unblocks_the_other() {
        let s = Shields::new(&["@@||cdn.example.com^"], &["||cdn.example.com^"]);
        assert!(!s.should_block("https://cdn.example.com/a.js", "https://a.com/", ResourceKind::Script));
    }

    #[test]
    fn never_blocks_top_level_document() {
        let s = Shields::new(&["||doubleclick.net^"], &[]);
        assert!(!s.should_block("https://doubleclick.net/", "https://doubleclick.net/", ResourceKind::Document));
        assert!(s.should_block("https://doubleclick.net/x.js", "https://a.com/", ResourceKind::Script));
    }
}
