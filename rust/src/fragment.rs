use crate::ref_::{self, Ref};
use crate::sha;
use crate::witnessed::{self, Witnessed};

/// A node in the possibility space.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fragment {
    /// Terminal: self-addressed, witnessed, carries data, stops.
    Shard {
        ref_: Ref,
        witnessed: Witnessed,
        data: String,
    },
    /// Self-similar: self-addressed, witnessed, carries data, contains fragments.
    Fragment {
        ref_: Ref,
        witnessed: Witnessed,
        data: String,
        fragments: Vec<Fragment>,
    },
}

impl Fragment {
    /// Create a shard. Terminal fragment.
    pub fn shard(ref_: Ref, witnessed: Witnessed, data: impl Into<String>) -> Self {
        Fragment::Shard {
            ref_,
            witnessed,
            data: data.into(),
        }
    }

    /// Create a fragment. Self-similar, contains other fragments.
    pub fn new_fragment(
        ref_: Ref,
        witnessed: Witnessed,
        data: impl Into<String>,
        fragments: Vec<Fragment>,
    ) -> Self {
        Fragment::Fragment {
            ref_,
            witnessed,
            data: data.into(),
            fragments,
        }
    }

    /// Get the ref (self-address) of a fragment.
    pub fn self_ref(&self) -> &Ref {
        match self {
            Fragment::Shard { ref_, .. } => ref_,
            Fragment::Fragment { ref_, .. } => ref_,
        }
    }

    /// Get the witness record of a fragment.
    pub fn self_witnessed(&self) -> &Witnessed {
        match self {
            Fragment::Shard { witnessed, .. } => witnessed,
            Fragment::Fragment { witnessed, .. } => witnessed,
        }
    }

    /// Get the data from a fragment.
    pub fn data(&self) -> &str {
        match self {
            Fragment::Shard { data, .. } => data,
            Fragment::Fragment { data, .. } => data,
        }
    }

    /// Get child fragments. Shards have none.
    pub fn children(&self) -> &[Fragment] {
        match self {
            Fragment::Shard { .. } => &[],
            Fragment::Fragment { fragments, .. } => fragments,
        }
    }

    /// Check if a fragment is a shard.
    pub fn is_shard(&self) -> bool {
        matches!(self, Fragment::Shard { .. })
    }

    /// Check if a fragment is a fragment (non-terminal).
    pub fn is_fragment(&self) -> bool {
        matches!(self, Fragment::Fragment { .. })
    }
}

/// Deterministic canonical serialization of a fragment.
pub fn serialize(frag: &Fragment) -> String {
    match frag {
        Fragment::Shard {
            ref_,
            witnessed,
            data,
        } => {
            format!(
                "shard\n{}\n{}\ndata:{}",
                ref_::serialize_ref(ref_),
                witnessed::serialize_witnessed(witnessed),
                data,
            )
        }
        Fragment::Fragment {
            ref_,
            witnessed,
            data,
            fragments,
        } => {
            let children_str: String = fragments
                .iter()
                .map(serialize)
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "fragment\n{}\n{}\ndata:{}\nfragments:[{}]",
                ref_::serialize_ref(ref_),
                witnessed::serialize_witnessed(witnessed),
                data,
                children_str,
            )
        }
    }
}

/// Content-address a fragment: SHA-256 of its canonical serialization.
pub fn hash_fragment(frag: &Fragment) -> String {
    let serialized = serialize(frag);
    sha::hash(&serialized).0
}
