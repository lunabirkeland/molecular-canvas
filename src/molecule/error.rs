use thiserror::Error;

use super::{AtomId, BondId, MoleculeId};

#[derive(Error, Debug)]
pub enum Error {
    #[error("atom id collision")]
    AtomCollision(AtomId),
    #[error("bond id collision")]
    BondCollision(BondId),
    #[error("molecule id collision")]
    MoleculeCollision(MoleculeId),
    #[error("atom not found")]
    AtomMissing(AtomId),
    #[error("bond not found")]
    BondMissing(BondId),
    #[error("molecule not found")]
    MoleculeMissing(MoleculeId),
}
