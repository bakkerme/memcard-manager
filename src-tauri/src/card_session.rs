use crate::card::{CardSource, CardView, Ps1Card};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedCard {
    #[serde(flatten)]
    view: CardView,
    session_id: String,
}

#[derive(Default)]
pub struct CardSessions {
    cards: HashMap<String, Ps1Card>,
    active: Option<String>,
    next_id: u64,
}

impl CardSessions {
    pub fn open(&mut self, card: Ps1Card) -> LoadedCard {
        let session_id = if card.source == CardSource::Usb {
            "usb".to_string()
        } else {
            self.next_id += 1;
            format!("file-{}", self.next_id)
        };
        let view = card.view();
        self.cards.insert(session_id.clone(), card);
        self.active = Some(session_id.clone());
        LoadedCard { view, session_id }
    }

    pub fn active(&self) -> Result<&Ps1Card, String> {
        self.active
            .as_ref()
            .and_then(|id| self.cards.get(id))
            .ok_or_else(|| "No card is open.".to_string())
    }

    pub fn activate(&mut self, session_id: &str) -> Result<(), String> {
        if !self.cards.contains_key(session_id) {
            return Err("That card is no longer open.".to_string());
        }
        self.active = Some(session_id.to_string());
        Ok(())
    }

    pub fn close(&mut self, session_id: &str, next_session_id: Option<&str>) -> Result<(), String> {
        if let Some(next) = next_session_id {
            if next == session_id || !self.cards.contains_key(next) {
                return Err("The next card is no longer open.".to_string());
            }
        }
        self.cards.remove(session_id);
        if self.active.as_deref() == Some(session_id) {
            self.active = next_session_id.map(str::to_string);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switching_and_closing_preserve_each_card_snapshot() {
        let mut sessions = CardSessions::default();
        let first = sessions.open(Ps1Card::create_formatted("first.mcr"));
        let mut card = Ps1Card::create_formatted("second.mcr");
        card.raw[8192] = 42;
        let second = sessions.open(card);
        assert_eq!(sessions.active().unwrap().raw[8192], 42);
        sessions.activate(&first.session_id).unwrap();
        assert_eq!(sessions.active().unwrap().source_name, "first.mcr");
        assert_ne!(sessions.active().unwrap().raw[8192], 42);
        sessions.close(&second.session_id, None).unwrap();
        assert_eq!(sessions.active().unwrap().source_name, "first.mcr");
        assert!(sessions.activate(&second.session_id).is_err());
        sessions.close(&first.session_id, None).unwrap();
        assert!(sessions.active().is_err());
    }

    #[test]
    fn closing_active_card_selects_the_requested_successor() {
        let mut sessions = CardSessions::default();
        let first = sessions.open(Ps1Card::create_formatted("first.mcr"));
        let second = sessions.open(Ps1Card::create_formatted("second.mcr"));
        assert!(sessions.close(&second.session_id, Some("missing")).is_err());
        assert_eq!(sessions.active().unwrap().source_name, "second.mcr");
        sessions
            .close(&second.session_id, Some(&first.session_id))
            .unwrap();
        assert_eq!(sessions.active().unwrap().source_name, "first.mcr");
    }

    #[test]
    fn adaptor_rereads_replace_only_the_physical_snapshot() {
        let mut sessions = CardSessions::default();
        let file = sessions.open(Ps1Card::create_formatted("virtual.mcr"));
        for name in ["physical", "reread"] {
            let mut card = Ps1Card::create_formatted(name);
            card.source = CardSource::Usb;
            assert_eq!(sessions.open(card).session_id, "usb");
        }
        assert_eq!(sessions.cards.len(), 2);
        assert_eq!(sessions.active().unwrap().source_name, "reread");
        sessions.activate(&file.session_id).unwrap();
        assert_eq!(sessions.active().unwrap().source_name, "virtual.mcr");
    }
}
