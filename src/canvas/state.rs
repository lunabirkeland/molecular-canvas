use anyhow::Context;
use anyhow::Result;
use iced::Point;
use iced::Rectangle;
use iced::Vector;
use crate::bounds::Bounds;
use crate::molecule;
use crate::molecule::Bond;
use crate::molecule::BondId;
use crate::molecule::MoleculePosition;
use crate::molecule::Atom;
use crate::molecule::Molecule;
use crate::molecule::AtomId;
use crate::molecule::MoleculeId;
use rustc_hash::FxHashMap;

use super::selection::HoverSelection;
use super::selection::SingleSelection;
use super::Selection;

#[derive(Default, Debug)]
pub struct State {
    molecules: FxHashMap<MoleculeId, Molecule>,
    selection: Selection,
}

impl State {
    // pub fn add_molecule(&mut self, molecule_id: MoleculeId, position: Point) {
    //     self.molecules
    //         .insert(molecule_id, Molecule::new(position));
    // }

    pub fn add_molecule_with_atom(&mut self, molecule_id: MoleculeId, atom_id: AtomId, label: String, position: Point) -> Result<()> {
        let molecule = Molecule::new(position, atom_id, label).context("while adding molecule with atoms")?;
        if self.molecules.insert(molecule_id, molecule).is_some() {
            return Err(molecule::Error::MoleculeCollision(molecule_id)).context("while adding molecule with atoms")
        };
        Ok(())
    }

    pub fn molecules(&self) -> impl Iterator<Item = (&MoleculeId, &Molecule)> {
        self.molecules.iter()
    }

    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    pub fn new_selection(&mut self, selection: Selection) {
        self.selection = selection;
    }

    pub fn move_selection(&mut self, translation: Vector) -> Result<()> {
        for item in self.selection.clone() {
            match item {
                SingleSelection::Molecule(molecule_id) => {
                    let molecule = self.get_molecule_mut(&molecule_id).context("while moving selection")?;
                    molecule.move_molecule(translation);
                }
                SingleSelection::Atom(molecule_id, atom_id) => {
                    let molecule = self.get_molecule_mut(&molecule_id).context("while moving selection")?;
                    molecule.move_atom(&atom_id, translation).context("while moving selection")?;
                }
                SingleSelection::Bond(molecule_id, bond_id) => {
                    let molecule = self.get_molecule_mut(&molecule_id).context("while moving selection")?;
                    molecule.move_bond(&bond_id, translation).context("while moving selection")?;
                }
            }
        }

        Ok(())
    }

    pub fn molecules_at(&self, position: Point) -> impl Iterator<Item = (&MoleculeId, &Molecule, Bounds)> {
        self.molecules
            .iter()
            .filter_map(move |(molecule_id, molecule)| {
                let bounds = molecule.bounds();
                if bounds.contains(position) {
                    Some((molecule_id, molecule, bounds))
                } else {
                    None
                }
            })
    }

    pub fn get_molecule(&self, molecule_id: &MoleculeId) -> Result<&Molecule> {
        self.molecules.get(molecule_id).ok_or(molecule::Error::MoleculeMissing(*molecule_id))
            .context("while getting molecule")
    }

    pub fn get_molecule_mut(&mut self, molecule_id: &MoleculeId) -> Result<&mut Molecule> {
        self.molecules.get_mut(molecule_id).ok_or(molecule::Error::MoleculeMissing(*molecule_id))
            .context("while getting molecule mut")
    }

    pub fn get_atom(&self, molecule_id: &MoleculeId, atom_id: &AtomId) -> Result<&Atom> {
        let molecule = self.get_molecule(molecule_id).context("while getting atom")?;
        molecule.get_atom(atom_id).context("while getting atom")
    }

    pub fn get_bond(&self, molecule_id: &MoleculeId, bond_id: &BondId) -> Result<&Bond> {
        let molecule = self.get_molecule(molecule_id).context("while getting bond")?;
        molecule.get_bond(bond_id).context("while getting bond")
    }

    pub fn remove_molecule(&mut self, molecule_id: &MoleculeId) -> Result<Molecule> {
        self.selection.clear();
        self.molecules.remove(molecule_id)
            .ok_or(molecule::Error::MoleculeMissing(*molecule_id)).context("while removing molecule")
    }

    pub fn delete_atom(&mut self, molecule_id: &MoleculeId, atom_id: AtomId) -> Result<()> {
        self.selection.clear();
        let molecule = self.get_molecule_mut(molecule_id).context("while deleting atom")?;
        let detached_molecules = molecule.delete_atom(atom_id).context("while delting atom")?;

        if molecule.is_empty() {
            self.remove_molecule(molecule_id)?;
        }

        for molecule in detached_molecules {
            self.molecules.insert(MoleculeId::new(), molecule).context("while inserting detached molecules")
                .context("while deleting atom")?;
        }

        Ok(())
    }

    pub fn delete_bond(&mut self, molecule_id: &MoleculeId, bond_id: BondId) -> Result<()> {
        self.selection.clear();
        let molecule = self.get_molecule_mut(molecule_id)?;
        let detached_molecules = molecule.delete_bond(bond_id)?;

        for molecule in detached_molecules {
            self.molecules.insert(MoleculeId::new(), molecule);
        }

        Ok(())
    }

    /// prioritises closeness to center of bounding box
    pub fn get_hovered(&self, canvas_position: Point) -> Result<HoverSelection> {
        let mut selection_candidate: Option<(SingleSelection, Vector)> = None;
        let mut candidate_rating = f32::MAX;

        for (molecule_id, molecule, bounds) in self.molecules_at(canvas_position) {
            let rating = bounds.center().distance(canvas_position);
            for (atom_id, _atom, bounds) in molecule.atoms_at(canvas_position) {
                let rating = bounds.center().distance(canvas_position);

                if rating < candidate_rating {
                    candidate_rating = rating;
                    selection_candidate = Some((
                            SingleSelection::Atom(*molecule_id, *atom_id),
                            molecule.atom_position(atom_id).unwrap() - canvas_position
                    ));
                }
            }
            for (bond_id, _bond, bounds) in molecule.bonds_at(canvas_position).context("while getting hovered")? {
                let rating = bounds.center().distance(canvas_position);

                if rating < candidate_rating {
                    candidate_rating = rating;
                    selection_candidate = Some((
                            SingleSelection::Bond(*molecule_id, *bond_id),
                            molecule.bond_position(bond_id).unwrap() - canvas_position
                    ));
                }
            }

            if rating < candidate_rating && matches!(selection_candidate, Some((SingleSelection::Molecule(_), _)) | None) {
                candidate_rating = rating;
                selection_candidate = Some((
                        SingleSelection::Molecule(*molecule_id),
                        <MoleculePosition as Into<Point>>::into(molecule.position()) - canvas_position
                ));
            }
        }

        Ok(HoverSelection::from(selection_candidate))
    }

    pub fn get_selection(&self, rect: Rectangle) -> Result<Selection> {
        let mut selection = Vec::new();

        for (molecule_id, molecule) in &self.molecules {
            let bounds = molecule.bounds();
            if bounds.is_contained(&rect) {
                selection.push(SingleSelection::Molecule(*molecule_id));
            } else if bounds.intersects(&rect) {
                for (atom_id, atom) in molecule.atoms() {
                    let bounds = atom.bounds() + molecule.position().into();
                    if bounds.is_contained(&rect) {
                        selection.push(SingleSelection::Atom(*molecule_id, *atom_id));
                    }
                }
            }
        }

        Ok(Selection::from_iter(selection))
    }
}

