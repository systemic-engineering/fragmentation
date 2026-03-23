use std::convert::Infallible;

use crate::fragment::Fractal;
use crate::sha::HashAlg;

/// The point where a tree of possibilities collapses into a single artifact.
/// `collapse` resolves. `refract` reconstructs.
pub trait Singularity: Sized {
    type Artifact;
    type Error;

    fn collapse(&self) -> Result<Self::Artifact, Self::Error>;
    fn refract(artifact: &Self::Artifact) -> Result<Self, Self::Error>;
}

impl<E: Clone, H: HashAlg> Singularity for Fractal<E, H> {
    type Artifact = Self;
    type Error = Infallible;

    fn collapse(&self) -> Result<Self, Infallible> {
        Ok(self.clone())
    }

    fn refract(artifact: &Self) -> Result<Self, Infallible> {
        Ok(artifact.clone())
    }
}
