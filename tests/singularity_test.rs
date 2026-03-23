use std::convert::Infallible;

use fragmentation::fragment::{self, Fractal};
use fragmentation::ref_::Ref;
use fragmentation::sha::{HashAlg, Sha};
use fragmentation::singularity::Singularity;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_shard(data: &str) -> Fractal<String> {
    let r = Ref::new(Sha(fragment::blob_oid(data)), "test");
    Fractal::shard(r, data)
}

fn make_fractal(data: &str, children: Vec<Fractal<String>>) -> Fractal<String> {
    let r = Ref::new(Sha(fragment::tree_oid(data, &children)), "test");
    Fractal::new(r, data, children)
}

fn make_lens(data: &str, targets: Vec<Sha>) -> Fractal<String> {
    let r = Ref::new(Sha(fragment::blob_oid(data)), "test");
    Fractal::lens(r, data, targets)
}

// ===========================================================================
// Collapse identity
// ===========================================================================

#[test]
fn shard_collapse_is_identity() {
    let shard = make_shard("hello");
    let collapsed = shard.collapse().unwrap();
    assert_eq!(collapsed, shard);
}

#[test]
fn fractal_collapse_is_identity() {
    let child = make_shard("leaf");
    let fractal = make_fractal("root", vec![child]);
    let collapsed = fractal.collapse().unwrap();
    assert_eq!(collapsed, fractal);
}

#[test]
fn lens_collapse_is_identity() {
    let target = Sha::hash(b"target");
    let lens = make_lens("link", vec![target]);
    let collapsed = lens.collapse().unwrap();
    assert_eq!(collapsed, lens);
}

// ===========================================================================
// Refract identity
// ===========================================================================

#[test]
fn shard_refract_is_identity() {
    let shard = make_shard("world");
    let refracted = Fractal::<String>::refract(&shard).unwrap();
    assert_eq!(refracted, shard);
}

// ===========================================================================
// Roundtrips
// ===========================================================================

#[test]
fn collapse_then_refract_roundtrip() {
    let child = make_shard("leaf");
    let fractal = make_fractal("root", vec![child]);
    let artifact = fractal.collapse().unwrap();
    let restored = Fractal::<String>::refract(&artifact).unwrap();
    assert_eq!(restored, fractal);
}

#[test]
fn refract_then_collapse_roundtrip() {
    let shard = make_shard("data");
    let restored = Fractal::<String>::refract(&shard).unwrap();
    let collapsed = restored.collapse().unwrap();
    assert_eq!(collapsed, shard);
}

// ===========================================================================
// Type-level guarantees
// ===========================================================================

#[test]
fn collapse_error_is_infallible() {
    let shard = make_shard("safe");
    let result: Result<Fractal<String>, Infallible> = shard.collapse();
    assert!(result.is_ok());
}

// ===========================================================================
// Custom impl proves the trait is useful beyond the default
// ===========================================================================

#[test]
fn custom_singularity_impl() {
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Document {
        sections: Vec<String>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Manuscript(String);

    impl Singularity for Document {
        type Artifact = Manuscript;
        type Error = Infallible;

        fn collapse(&self) -> Result<Manuscript, Infallible> {
            Ok(Manuscript(self.sections.join("\n")))
        }

        fn refract(artifact: &Manuscript) -> Result<Self, Infallible> {
            Ok(Document {
                sections: artifact.0.split('\n').map(String::from).collect(),
            })
        }
    }

    let doc = Document {
        sections: vec!["chapter one".into(), "chapter two".into()],
    };
    let manuscript = doc.collapse().unwrap();
    assert_eq!(manuscript.0, "chapter one\nchapter two");
    let restored = Document::refract(&manuscript).unwrap();
    assert_eq!(restored, doc);
}
