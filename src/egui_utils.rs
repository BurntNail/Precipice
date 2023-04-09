//! Makes an optionally ordered list of things to be displayed in an EGUI window

use egui::{ScrollArea, Ui};
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    vec::IntoIter,
};

///An enum to represent a change in a list item
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChangeType {
    ///An item was removed
    Removed, //TODO: also return index with change
    ///An item was reordered
    Reordered,
}

///A struct to wrap around a [`Vec`], which has utilities related to displaying it in an [`egui`] window.
#[derive(Debug, Clone)]
pub struct EguiList<T> {
    ///Whether or not the list is displayed in a vertical [`egui::ScrollArea`]. Defaults to `false`
    is_scrollable: bool,
    ///Whether or not you can remove items from the list. Defaults to `false`
    is_editable: bool,
    ///Whether or not you can reorder items in the list. Defaults to `false`
    is_reorderable: bool,
    ///A temporary variable for if we had an update
    had_list_update: Option<ChangeType>,
    ///The backing list that gets displayed.
    backing: Vec<T>,
}

impl<T> Default for EguiList<T> {
    fn default() -> Self {
        Self {
            is_scrollable: false,
            is_editable: false,
            is_reorderable: false,
            backing: vec![],
            had_list_update: None,
        }
    }
}

impl<T> EguiList<T> {
    ///This uses [`std::mem::take`] on the temporary list update variable - it gets it, and if you poll after it will be [`None`] unless something changes
    #[must_use]
    pub fn had_update(&mut self) -> Option<ChangeType> {
        std::mem::take(&mut self.had_list_update)
    }

    ///Changes whether or not we can scroll - builder pattern
    #[must_use]
    pub const fn is_scrollable(mut self, is_scrollable: bool) -> Self {
        self.is_scrollable = is_scrollable;
        self
    }

    ///Changes whether or not we can remove items - builder pattern
    #[must_use]
    pub const fn is_editable(mut self, is_editable: bool) -> Self {
        self.is_editable = is_editable;
        self
    }

    ///Changes whether or not we can reorder items - builder pattern
    #[must_use]
    pub const fn is_reorderable(mut self, is_reorderable: bool) -> Self {
        self.is_reorderable = is_reorderable;
        self
    }

    ///Inner method for displaying - this way we avoid code duplication around the scroll area.
    fn display_inner(&mut self, ui: &mut Ui, label: impl Fn(&T, usize) -> String) {
        if self.backing.is_empty() {
            //If we don't have any arguments, then we don't need any of this and some of the logic gets screwed because of 0s
            return;
        }

        //we could have multiple (as in vecs rather than options), but immediate mode, so unlikely to affect UX but much easier for me
        let mut need_to_remove = None; //we need to remove this index
        let mut up = None; //move this index up a position
        let mut down = None; //move this index down a position

        for (i, arg) in self.backing.iter().enumerate() {
            ui.horizontal(|ui| {
                //for each of our CLI args, make a new horizontal environment (to almost mimic a table without alignment), and add buttons for remove/up/down, and if we get input then set relevant variables
                ui.label(label(arg, i)); //we don't break to ensure that everything always gets drawn, but we still skip over lots of logic if we have a change

                if self.had_list_update.is_none() {
                    if self.is_editable && ui.button("Remove?").clicked() {
                        //if we need to remove, then set the index
                        need_to_remove = Some(i);
                        self.had_list_update = Some(ChangeType::Removed);
                    }
                    if self.is_reorderable {
                        //if we can redorder
                        if ui.button("Up?").clicked() {
                            //then set variables if we get clicks
                            up = Some(i);
                            self.had_list_update = Some(ChangeType::Reordered);
                        }
                        if ui.button("Down?").clicked() {
                            down = Some(i);
                            self.had_list_update = Some(ChangeType::Reordered);
                        }
                    }
                }
            });
        }

        let len_minus_one = self.backing.len() - 1;
        if let Some(need_to_remove) = need_to_remove {
            self.backing.remove(need_to_remove);
        } else if let Some(up) = up {
            //extra code with checking <> 0 for wrapping around rather than just normal swapping
            if up > 0 {
                self.backing.swap(up, up - 1);
            } else {
                self.backing.swap(0, len_minus_one);
            }
        } else if let Some(down) = down {
            if down < len_minus_one {
                self.backing.swap(down, down + 1);
            } else {
                self.backing.swap(len_minus_one, 0);
            }
        }
    }

    ///Actually displays the items, taking in a closure for how to display the items.
    pub fn display(&mut self, ui: &mut Ui, label: impl Fn(&T, usize) -> String) {
        if self.is_scrollable {
            //need to have 2 methods to allow one to be inside the vertical scroll
            ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                self.display_inner(ui, label);
            });
        } else {
            self.display_inner(ui, label);
        }
    }
}

impl<T> Deref for EguiList<T> {
    type Target = Vec<T>; //easily use vector methods without code duplication

    fn deref(&self) -> &Self::Target {
        &self.backing
    }
}

impl<T> DerefMut for EguiList<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.backing
    }
}

impl<T> From<Vec<T>> for EguiList<T> {
    fn from(value: Vec<T>) -> Self {
        //make an EguiList easily from a Vec
        Self {
            backing: value,
            ..Default::default()
        }
    }
}

impl<T> IntoIterator for EguiList<T> {
    type Item = T;
    type IntoIter = IntoIter<T>; //use the Vec into_iter

    fn into_iter(self) -> Self::IntoIter {
        self.backing.into_iter()
    }
}

impl<T: Clone> EguiList<T> {
    ///Clones the backing list
    #[must_use]
    pub fn backing_vec(&self) -> Vec<T> {
        self.backing.clone()
    }
}

impl<T> AsRef<[T]> for EguiList<T> {
    //get slice methods
    fn as_ref(&self) -> &[T] {
        &self.backing
    }
}
