use uuid::Uuid;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct AtomId(Uuid);
impl AtomId {
    pub fn new() -> AtomId {
        AtomId(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct BondId(Uuid);
impl BondId {
    pub fn new() -> BondId {
        BondId(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct MoleculeId(Uuid);
impl MoleculeId {
    pub fn new() -> MoleculeId {
        MoleculeId(Uuid::new_v4())
    }
}

