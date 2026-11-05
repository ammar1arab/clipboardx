use sha2::{Digest, Sha256};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const WINDOW: Duration = Duration::from_secs(2);

#[derive(Clone, PartialEq, Eq)]
enum Kind {
    Text(String),
    Image(String),
}

struct Recorded {
    kind: Kind,
    until: Instant,
}

#[derive(Clone, Default)]
pub struct OwnWrites {
    state: Arc<Mutex<Option<Recorded>>>,
}

fn hash_text(text: &str) -> String {
    hex::encode(Sha256::digest(text.as_bytes()))
}

impl OwnWrites {
    pub fn note_text(&self, text: &str) {
        self.record(Kind::Text(hash_text(text)), WINDOW);
    }

    pub fn note_image(&self, hash: &str) {
        self.record(Kind::Image(hash.to_string()), WINDOW);
    }

    fn record(&self, kind: Kind, window: Duration) {
        if let Ok(mut state) = self.state.lock() {
            *state = Some(Recorded {
                kind,
                until: Instant::now() + window,
            });
        }
    }

    pub fn suppress_text(&self, text: &str) -> bool {
        self.suppress(&Kind::Text(hash_text(text)))
    }

    pub fn suppress_image(&self, hash: &str) -> bool {
        self.suppress(&Kind::Image(hash.to_string()))
    }

    fn suppress(&self, kind: &Kind) -> bool {
        let Ok(state) = self.state.lock() else {
            return false;
        };
        let Some(recorded) = state.as_ref() else {
            return false;
        };
        Instant::now() < recorded.until && &recorded.kind == kind
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_matching_writes_only_inside_the_window() {
        let writes = OwnWrites::default();
        writes.record(Kind::Text(hash_text("combined")), Duration::from_secs(2));
        assert!(writes.suppress_text("combined"));
        assert!(!writes.suppress_text("a later copy"));
        writes.record(Kind::Text(hash_text("combined")), Duration::ZERO);
        assert!(!writes.suppress_text("combined"));
        writes.record(Kind::Image("abc".into()), Duration::from_secs(2));
        assert!(writes.suppress_image("abc"));
        assert!(!writes.suppress_image("def"));
        assert!(!writes.suppress_text("combined"));
    }
}
