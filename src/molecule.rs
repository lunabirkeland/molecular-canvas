use std::collections::VecDeque;
use std::iter;

use anyhow::{Context, Result};
use atom::Direction;
use bond::draw_bond;
use iced::widget::canvas::path::lyon_path::math::Transform;
use iced::widget::canvas::{Frame, Stroke};
use iced::Point;
use iced::{Color, Vector};
use rustc_hash::{FxHashMap, FxHashSet};

mod atom;
mod atom_position;
mod bond;
mod error;
mod id;
mod molecule_position;

pub use atom::Atom;
pub use atom_position::AtomPosition;
pub use bond::{Bond, BondType};
pub use error::Error;
pub use id::{AtomId, BondId, MoleculeId};
pub use molecule_position::MoleculePosition;

use crate::bounds::Bounds;
use crate::canvas::MolCanvas;

#[derive(Debug, Clone)]
pub struct Molecule {
    atoms: FxHashMap<AtomId, Atom>,
    bonds: FxHashMap<BondId, Bond>,
    local_bounds: Bounds,
    position: MoleculePosition,
}

impl Molecule {
    pub fn new(canvas_position: Point, atom_id: AtomId, label: String) -> Result<Self> {
        let atom = Atom::new(label, AtomPosition::default(), Direction::default());

        let mut molecule = Molecule {
            atoms: FxHashMap::from_iter([(atom_id, atom)]),
            bonds: FxHashMap::default(),
            local_bounds: Bounds::default(),
            position: canvas_position.into(),
        };

        molecule.compute_bounds().context("while creating new molecule")?;
        Ok(molecule)
    }

    pub fn atoms(&self) -> impl Iterator<Item = (&AtomId, &Atom)> {
        self.atoms.iter()
    }

    // pub fn bonds(&self) -> impl Iterator<Item = (&BondId, &Bond)> {
    //     self.bonds.iter()
    // }

    fn compute_bounds(&mut self) -> Result<()> {
        let mut atoms = self.atoms.values();
        if let Some(mut bounds) = atoms.next().map(Atom::bounds) {
            for atom in self.atoms.values() {
                bounds = bounds.union(&atom.bounds());
            }

            self.local_bounds = bounds;
        } else {
            self.local_bounds = Bounds::default();
        }

        self.local_bounds.add_padding(MolCanvas::MOLECULE_PADDING);

        Ok(())
    }

    pub fn draw(
        &self,
        frame: &mut Frame,
        atom_color: &Color,
        bond_stroke: &Stroke,
        bond_color: &Color,
    ) -> Result<()> {
        let transform = self.position.into();

        for atom in self.atoms.values() {
            atom.draw(frame, &transform, atom_color)?;
            // atom.bounds().draw(frame, Stroke {
            //         style: Style::Solid(Color::WHITE),
            //         width: 1.0,
            //         ..Default::default()
            //     }, 1.0, MolCanvas::SELECT_PADDING)
        }

        for bond in self.bonds.values() {
            bond.draw(frame, &transform, &self.atoms, bond_stroke, bond_color)?;
            // bond.bounds(&self.atoms).draw(frame, Stroke {
            //         style: Style::Solid(Color::WHITE),
            //         width: 1.0,
            //         ..Default::default()
            //     }, 1.0, MolCanvas::SELECT_PADDING)
        }

        Ok(())
    }

    pub fn draw_pending_bond(
        &self,
        frame: &mut Frame,
        start: Point,
        end: Point,
        bond_type: &BondType,
        stroke: &Stroke,
        color: &Color,
    ) -> Result<()> {
        draw_bond(
            frame,
            &Transform::identity(),
            start,
            end,
            bond_type,
            stroke,
            color,
        )
    }

    pub fn extend(&mut self, mut molecule: Molecule) {
        let offset: Vector = Point::from(molecule.position) - Point::from(self.position);
        for (atom_id, mut atom) in molecule.atoms.drain() {
            atom.translate(offset);
            self.atoms.insert(atom_id, atom);
        }
        self.bonds.extend(molecule.bonds);

        let bounds = molecule.local_bounds + offset;

        self.local_bounds = self.local_bounds.union(&bounds);
    }

    pub fn rename_atom(&mut self, atom_id: &AtomId, text: String) -> Result<()> {
        let atom = self.get_atom_mut(atom_id).context("while renaming atom")?;
        atom.rename(text);

        self.compute_bounds().context("while renaming atom")?;

        Ok(())
    }

    pub fn get_atom_bounds(&self, atom_id: &AtomId) -> Result<Bounds> {
        let atom = self
            .get_atom(atom_id)
            .context("while getting atom bounds")?;

        Ok(atom.bounds() + self.position.into())
    }

    pub fn get_bond_bounds(&self, bond_id: &BondId) -> Result<Bounds> {
        let bond = self
            .get_bond(bond_id)
            .context("while getting bond bounds")?;

        Ok(bond.bounds(&self.atoms)? + self.position.into())
    }

    pub fn add_atom(
        &mut self,
        atom_id: AtomId,
        label: String,
        canvas_position: Point,
    ) -> Result<()> {
        let position = AtomPosition::from(self.position, canvas_position);
        if self
            .atoms
            .insert(atom_id, Atom::new(label, position, Direction::default()))
            .is_some()
        {
            return Err(Error::AtomCollision(atom_id)).context("while adding atom");
        };

        self.compute_bounds()?;

        Ok(())
    }

    pub fn delete_atom(&mut self, atom_id: AtomId) -> Result<impl IntoIterator<Item = Molecule>> {
        self.atoms
            .remove(&atom_id)
            .ok_or(Error::AtomMissing(atom_id))
            .context("while deleting atom")?;

        let attached_bonds = self
            .attached_bonds(atom_id)
            .map(|(bond_id, _bond)| *bond_id)
            .collect::<Vec<_>>();
        let connected_atoms = self.get_directly_connected(atom_id).collect::<Vec<_>>();

        for bond_id in attached_bonds {
            self.bonds.remove(&bond_id);
        }

        for atom_id in &connected_atoms {
            self.update_atom_label_direction(atom_id)?;
        }

        self.split_fragments(connected_atoms.into_iter())
            .context("while deleting atom")
            .map(Vec::into_iter)
    }

    /// deletes bond in molecule and returns an iterator of all molecules that have become detached
    pub fn delete_bond(&mut self, bond_id: BondId) -> Result<impl Iterator<Item = Molecule>> {
        let bond = self
            .bonds
            .remove(&bond_id)
            .ok_or(Error::BondMissing(bond_id))
            .context("while deleting bond")?;
        let bond_atoms = bond.atom_ids().collect::<Vec<_>>();

        for atom_id in &bond_atoms {
            self.update_atom_label_direction(atom_id).context("while deleting bond")?;
        }

        self.split_fragments(bond_atoms.into_iter())
            .context("while deleting bond")
            .map(Vec::into_iter)
    }

    /// removes all non-connected fragments in the molecule and returns them as new molecules
    fn split_fragments(&mut self, atom_ids: impl Iterator<Item = AtomId>) -> Result<Vec<Molecule>> {
        let atom_sets = atom_ids.map(|atom| self.get_connected(atom));

        let mut unique_atom_sets = vec![];
        let mut seen_atoms = FxHashSet::default();
        'outer: for atom_set in atom_sets {
            let mut atoms = vec![];
            for atom in atom_set {
                atoms.push(atom);
                if !seen_atoms.insert(atom) {
                    // not unique molecule
                    continue 'outer;
                }
            }
            if !atoms.is_empty() {
                unique_atom_sets.push(atoms);
            };
        }

        if unique_atom_sets.len() < 2 {
            self.compute_bounds()
                .context("while exiting as no fragments")
                .context("while splitting fragments")?;
            return Ok(vec![]);
        }

        let mut molecules = vec![];
        // first unque atom set is the molecule itself
        for atom_set in &unique_atom_sets[1..] {
            let mut atoms: FxHashMap<AtomId, Atom> = FxHashMap::default();
            let mut bonds: FxHashMap<BondId, Bond> = FxHashMap::default();
            for atom_id in atom_set {
                let atom = self
                    .atoms
                    .remove(atom_id)
                    .ok_or(Error::AtomMissing(*atom_id))
                    .context("while removing atom from original fragment")
                    .context("while splitting fragments")?;
                atoms.insert(*atom_id, atom);
            }

            self.bonds.retain(|bond_id, bond| {
                if atoms.contains_key(&bond.start()) || atoms.contains_key(&bond.end()) {
                    bonds.insert(*bond_id, bond.clone());
                    return false;
                }
                true
            });

            let mut molecule = Molecule {
                atoms,
                bonds,
                local_bounds: Bounds::default(),
                position: self.position,
            };

            molecule.compute_bounds().context("while splitting fragments")?;

            molecules.push(molecule);
        }

        self.compute_bounds().context("while splitting fragments")?;

        Ok(molecules)
    }

    fn get_connected(&self, atom_id: AtomId) -> impl Iterator<Item = AtomId> + '_ {
        let mut atoms = vec![atom_id];
        let mut atom_queue = VecDeque::from([atom_id]);

        iter::from_fn(move || {
            let curr_atom = atom_queue.pop_front()?;

            for (_bond_id, bond) in self.attached_bonds(curr_atom) {
                for atom in bond.atom_ids() {
                    if !atoms.contains(&atom) {
                        atom_queue.push_back(atom);
                        atoms.push(atom);
                    }
                }
            }

            Some(curr_atom)
        })
    }

    fn attached_bonds(&self, atom_id: AtomId) -> impl Iterator<Item = (&BondId, &Bond)> {
        self.bonds
            .iter()
            .filter(move |(_bond_id, bond)| bond.start() == atom_id || bond.end() == atom_id)
    }

    fn get_directly_connected(&self, atom_id: AtomId) -> impl Iterator<Item = AtomId> + '_ {
        self.attached_bonds(atom_id)
            .flat_map(|(_bond_id, bond)| bond.atom_ids())
            .filter(move |bond_atom_id| *bond_atom_id != atom_id)
    }

    pub fn add_bond(&mut self, start: AtomId, end: AtomId, bond_strength: BondType) -> Result<()> {
        let bond_id = BondId::new();

        if self
            .bonds
            .insert(bond_id, Bond::new(start, end, bond_strength))
            .is_some()
        {
            return Err(Error::BondCollision(bond_id)).context("while adding bond");
        };

        self.update_atom_label_direction(&start).context("while adding bond")?;
        self.update_atom_label_direction(&end).context("while adding bond")?;
        self.compute_bounds().context("while adding bond")?;

        Ok(())
    }

    pub fn atoms_at(
        &self,
        canvas_position: Point,
    ) -> impl IntoIterator<Item = (&AtomId, &Atom, Bounds)> {
        self.atoms.iter().filter_map(move |(atom_id, atom)| {
            let bounds = atom.bounds() + self.position.into();

            if bounds.contains(canvas_position) {
                Some((atom_id, atom, bounds))
            } else {
                None
            }
        })
    }

    pub fn bonds_at(
        &self,
        canvas_position: Point,
    ) -> Result<impl IntoIterator<Item = (&BondId, &Bond, Bounds)>> {
        self.bonds
            .iter()
            .filter_map(move |(bond_id, bond)| {
                let bounds = match bond.bounds(&self.atoms) {
                    Ok(val) => val,
                    Err(error) => return Some(Err(error)),
                };

                let bounds = bounds + self.position.into();

                if bounds.contains(canvas_position) {
                    Some(Ok((bond_id, bond, bounds)))
                } else {
                    None
                }
            })
            .collect::<Result<Vec<_>>>()
    }

    pub fn is_empty(&self) -> bool {
        self.atoms.is_empty()
    }

    pub fn move_molecule(&mut self, translation: Vector) {
        self.position += translation;
    }

    pub fn move_atom(&mut self, atom_id: &AtomId, translation: Vector) -> Result<()> {
        let atom = self.get_atom_mut(atom_id).context("while moving atom")?;

        atom.translate(translation);

        self.update_atom_label_direction(atom_id).context("while moving atom")?;
        for atom_id in self.get_directly_connected(*atom_id).collect::<Vec<_>>() {
            self.update_atom_label_direction(&atom_id).context("while moving atom")?;
        }

        self.compute_bounds().context("while moving atom")?;

        Ok(())
    }

    pub fn move_bond(&mut self, bond_id: &BondId, translation: Vector) -> Result<()> {
        let bond = self.get_bond_mut(bond_id).context("while moving bond")?;

        let atom_ids = bond.atom_ids();

        let mut affected_atoms = FxHashSet::default();

        for atom_id in atom_ids {
            let atom = self.get_atom_mut(&atom_id).context("while moving bond")?;
            atom.translate(translation);

            affected_atoms.insert(atom_id);
            for atom_id in self.get_directly_connected(atom_id) {
                affected_atoms.insert(atom_id);
            }
        }

        for atom_id in affected_atoms {
            self.update_atom_label_direction(&atom_id).context("while moving bond")?;
        }

        self.compute_bounds().context("while moving bond")?;
        Ok(())
    }

    pub fn update_atom_label_direction(&mut self, atom_id: &AtomId) -> Result<()> {
        let atom = self
            .get_atom(atom_id)
            .context("while updating atom label direction")?;

        let connected_atoms = self
            .get_directly_connected(*atom_id)
            .map(|atom_id| self.get_atom(&atom_id))
            .collect::<Result<Vec<_>>>()?;

        let unit_direction_vectors: Vec<Vector> = connected_atoms
            .iter()
            .map(|connected_atom| {
                let direction_vector: Vector = (connected_atom.position() - atom.position()).into();
                let magnitude = f32::sqrt(direction_vector.x.powi(2) + direction_vector.y.powi(2));
                direction_vector * magnitude.powi(-1)
            })
            .collect();

        let mut blocked_directions = FxHashSet::default();

        for unit_vector in unit_direction_vectors {
            if unit_vector.x > 0.1 {
                blocked_directions.insert(Direction::Right);
            } else if unit_vector.x < -0.1 {
                blocked_directions.insert(Direction::Left);
            }
            if unit_vector.y > 0.1 {
                blocked_directions.insert(Direction::Down);
            } else if unit_vector.y < -0.1 {
                blocked_directions.insert(Direction::Up);
            }
        }

        let direction = if !blocked_directions.contains(&Direction::Right) {
            Direction::Right
        } else if !blocked_directions.contains(&Direction::Left) {
            Direction::Left
        } else if !blocked_directions.contains(&Direction::Up) {
            Direction::Up
        } else if !blocked_directions.contains(&Direction::Down) {
            Direction::Down
        } else {
            Direction::default()
        };

        let atom = self
            .get_atom_mut(atom_id)
            .context("while updating atom label direction")?;
        atom.update_label_direction(direction);

        Ok(())
    }

    pub fn change_bond_type(&mut self, bond_id: &BondId, bond_type: BondType) {
        let Some(bond) = self.bonds.get_mut(bond_id) else {
            return;
        };

        bond.change_type(bond_type);
    }

    pub fn flip_bond(&mut self, bond_id: &BondId) {
        let Some(bond) = self.bonds.get_mut(bond_id) else {
            return;
        };

        bond.flip();
    }

    pub fn bounds(&self) -> Bounds {
        self.local_bounds + self.position.into()
    }

    pub fn get_atom(&self, atom_id: &AtomId) -> Result<&Atom> {
        self.atoms
            .get(atom_id)
            .ok_or(Error::AtomMissing(*atom_id))
            .context("while getting atom")
    }

    pub fn get_atom_mut(&mut self, atom_id: &AtomId) -> Result<&mut Atom> {
        self.atoms
            .get_mut(atom_id)
            .ok_or(Error::AtomMissing(*atom_id))
            .context("while getting atom")
    }

    pub fn get_bond(&self, bond_id: &BondId) -> Result<&Bond> {
        self.bonds
            .get(bond_id)
            .ok_or(Error::BondMissing(*bond_id))
            .context("while getting bond")
    }

    pub fn get_bond_mut(&mut self, bond_id: &BondId) -> Result<&mut Bond> {
        self.bonds
            .get_mut(bond_id)
            .ok_or(Error::BondMissing(*bond_id))
            .context("while getting bond")
    }

    pub fn position(&self) -> MoleculePosition {
        self.position
    }

    pub fn atom_position(&self, atom_id: &AtomId) -> Result<Point> {
        self.get_atom(atom_id)
            .map(|atom| atom.position() + self.position())
            .context("while getting atom's position")
    }

    pub fn bond_position(&self, bond_id: &BondId) -> Result<Point> {
        let bond = self
            .get_bond(bond_id)
            .context("while getting bond's position")?;
        let center = bond
            .center(&self.atoms)
            .context("while getting bond's position")?;

        Ok(center + self.position().into())
    }
}
