use crate::ref_::Ref;
use crate::witnessed::Witnessed;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fragment {
    Shard {
        ref_: Ref,
        witnessed: Witnessed,
        data: String,
    },
    Fragment {
        ref_: Ref,
        witnessed: Witnessed,
        data: String,
        fragments: Vec<Fragment>,
    },
}

impl Fragment {
    pub fn shard(ref_: Ref, witnessed: Witnessed, data: impl Into<String>) -> Self {
        Fragment::Shard {
            ref_,
            witnessed,
            data: data.into(),
        }
    }

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

    pub fn self_ref(&self) -> &Ref {
        match self {
            Fragment::Shard { ref_, .. } => ref_,
            Fragment::Fragment { ref_, .. } => ref_,
        }
    }

    pub fn self_witnessed(&self) -> &Witnessed {
        match self {
            Fragment::Shard { witnessed, .. } => witnessed,
            Fragment::Fragment { witnessed, .. } => witnessed,
        }
    }

    pub fn data(&self) -> &str {
        match self {
            Fragment::Shard { data, .. } => data,
            Fragment::Fragment { data, .. } => data,
        }
    }

    pub fn children(&self) -> &[Fragment] {
        match self {
            Fragment::Shard { .. } => &[],
            Fragment::Fragment { fragments, .. } => fragments,
        }
    }

    pub fn is_shard(&self) -> bool {
        matches!(self, Fragment::Shard { .. })
    }

    pub fn is_fragment(&self) -> bool {
        matches!(self, Fragment::Fragment { .. })
    }
}

pub fn serialize(_frag: &Fragment) -> String {
    todo!("implement serialize")
}

pub fn hash_fragment(_frag: &Fragment) -> String {
    todo!("implement hash_fragment")
}
