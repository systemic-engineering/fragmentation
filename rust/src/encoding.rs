use crate::fragment::Fragment;
use crate::store::Store;
use crate::witnessed::Witnessed;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    UnknownLabel(String),
}

pub fn encode_char(_ch: &str, _witness: &Witnessed) -> Fragment {
    todo!("implement encode_char")
}

pub fn encode_word(_word: &str, _witness: &Witnessed) -> Fragment {
    todo!("implement encode_word")
}

pub fn encode_sentence(_text: &str, _witness: &Witnessed) -> Fragment {
    todo!("implement encode_sentence")
}

pub fn encode_paragraph(_text: &str, _witness: &Witnessed) -> Fragment {
    todo!("implement encode_paragraph")
}

pub fn encode(_text: &str, _witness: &Witnessed) -> Fragment {
    todo!("implement encode")
}

pub fn decode(_fragment: &Fragment) -> Result<String, DecodeError> {
    todo!("implement decode")
}

pub fn ingest(_text: &str, _witness: &Witnessed, _store: Store) -> (Fragment, Store) {
    todo!("implement ingest")
}
