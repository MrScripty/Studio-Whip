use crate::gui_framework::scene::scene::Scene;
use std::fmt;

// Error type for group operations
#[derive(Debug)]
pub enum GroupError {
    DuplicateName,
    GroupNotFound,
}

impl fmt::Display for GroupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GroupError::DuplicateName => write!(f, "Group name already exists"),
            GroupError::GroupNotFound => write!(f, "Group not found"),
        }
    }
}

// Represents a logical group of objects
#[derive(Debug)]
pub struct Group {
    name: String,
    object_ids: Vec<usize>, // Pool indices of RenderObjects
}

impl Group {
    fn new(name: String) -> Self {
        Self {
            name,
            object_ids: Vec::new(),
        }
    }
}

// Manages all groups within a Scene
#[derive(Debug)]
pub struct GroupManager {
    groups: Vec<Group>,
}

impl GroupManager {
    pub fn new() -> Self {
        Self {
            groups: Vec::new(),
        }
    }

    pub fn add_group(&mut self, name: &str) -> Result<(), GroupError> {
        if self.groups.iter().any(|g| g.name == name) {
            return Err(GroupError::DuplicateName);
        }
        self.groups.push(Group::new(name.to_string()));
        Ok(())
    }

    pub fn delete_group(&mut self, name: &str) -> Result<(), GroupError> {
        let index = self.groups.iter().position(|g| g.name == name)
            .ok_or(GroupError::GroupNotFound)?;
        self.groups.remove(index);
        Ok(())
    }

    pub fn group<'a>(&'a mut self, name: &str, scene: &'a mut Scene) -> Result<GroupEditor<'a>, GroupError> {
        let group = self.groups.iter_mut().find(|g| g.name == name)
            .ok_or(GroupError::GroupNotFound)?;
        Ok(GroupEditor { group, scene })
    }

    pub fn get_groups_with_object(&self, object_id: usize) -> Vec<&str> {
        self.groups.iter()
            .filter(|g| g.object_ids.contains(&object_id))
            .map(|g| g.name.as_str())
            .collect()
    }
}

// Editor for modifying a specific group
pub struct GroupEditor<'a> {
    group: &'a mut Group,
    scene: &'a mut Scene,
}

impl<'a> GroupEditor<'a> {
    pub fn add_object(&mut self, object_id: usize) {
        if object_id < self.scene.pool.len() && !self.group.object_ids.contains(&object_id) {
            self.group.object_ids.push(object_id);
        }
    }

    pub fn remove_object(&mut self, object_id: usize) {
        if let Some(index) = self.group.object_ids.iter().position(|&id| id == object_id) {
            self.group.object_ids.remove(index);
        }
    }

    pub fn list_objects(&self) -> &[usize] {
        &self.group.object_ids
    }
}