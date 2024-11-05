use anyhow::{Context, Result};
use iced::Vector;

use crate::{bounds::Bounds, molecule::{AtomId, BondId, MoleculeId}};

use super::state::State;

#[derive(Debug, Default, Clone, Copy)]
pub struct HoverSelection(Option<(SingleSelection, Vector)>);

impl HoverSelection {
    // pub fn new(selection: SingleSelection, offset: Vector) -> Self {
    //     Self(Some((selection, offset)))
    // }

    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    pub fn bounds(&self, state: &State) -> Result<Option<Bounds>> {
        let Some(inner) = self.0 else {
            return Ok(None)
        };
        Ok(Some(inner.0.bounds(state).context("while getting hover selection's bounds")?))
    }

    pub fn selection(&self) -> Option<SingleSelection> {
        self.0.map(|(selection, _offset)| selection)
    }
}

impl From<Option<(SingleSelection, Vector)>> for HoverSelection {
    fn from(value: Option<(SingleSelection, Vector)>) -> Self {
        Self(value)
    }
}

#[derive(Debug, Default, Clone)]
pub struct Selection(Vec<SingleSelection>);

impl Selection {
    // pub fn is_empty(&self) -> bool {
    //     self.0.is_empty()
    // }
    pub fn clear(&mut self) {
        self.0.clear()
    }

    pub fn iter(&self) -> impl Iterator<Item = &SingleSelection> {
        self.0.iter()
    }

    pub fn bounds(&self, state: &State) -> Result<Vec<Bounds>> {
        self.0.iter().map(|s| s.bounds(state).context("while getting selection's bounds")).collect::<Result<Vec<_>>>()
    }

    pub fn contains(&self, hover_selection: &HoverSelection) -> bool {
        if let Some((single_selection, _offset)) = hover_selection.0 {
            match single_selection {
                SingleSelection::Molecule(_) => self.0.contains(&single_selection),
                SingleSelection::Atom(molecule_id, _) | SingleSelection::Bond(molecule_id, _) =>
                    self.0.iter().any(|item| match item {
                        _ if *item == single_selection => true,
                        SingleSelection::Molecule(sel_molecule_id) if molecule_id == *sel_molecule_id => true,
                        _ => false
                    })
            }
        } else {
            false
        }
    }

    pub fn remove(&mut self, single_selection: SingleSelection) {
        match single_selection {
            SingleSelection::Molecule(molecule_id) => self.0.retain(|item| matches!(item, SingleSelection::Molecule(mol_id) | SingleSelection::Atom(mol_id, _) | SingleSelection::Bond(mol_id, _) if molecule_id == *mol_id)),
            _ => self.0.retain(|item| *item != single_selection)
        };
    }
}

impl From<HoverSelection> for Selection {
    fn from(val: HoverSelection) -> Self {
        Selection(match val.0 {
            Some((selection, _)) => vec![selection],
            None => vec![]
        })
    }
}

impl IntoIterator for Selection {
    type Item = SingleSelection;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<SingleSelection> for Selection {
    fn from_iter<T: IntoIterator<Item = SingleSelection>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SingleSelection {
    Molecule(MoleculeId),
    Atom(MoleculeId, AtomId),
    Bond(MoleculeId, BondId),
}

impl SingleSelection {
    pub fn bounds(&self, state: &State) -> Result<Bounds> {
        Ok(match self {
            Self::Molecule(molecule_id) => {
                let molecule = state.get_molecule(molecule_id).context("while getting single selection's bounds")?;
                molecule.bounds()
            }
            Self::Atom(molecule_id, atom_id) => {
                let molecule = state.get_molecule(molecule_id).context("while getting single selection's bounds")?;
                molecule.get_atom_bounds(atom_id).context("while getting single selection's bounds")?
            }
            Self::Bond(molecule_id, bond_id) => {
                let molecule = state.get_molecule(molecule_id).context("while getting single selection's bounds")?;
                molecule.get_bond_bounds(bond_id).context("while getting single selection's bounds")?
            }
        })
    }
}


