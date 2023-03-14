//! Makes an ordered list of things to be displayed in an EGUI window
//!

use egui::{ScrollArea, Ui};
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    vec::IntoIter,
};

#[derive(Debug, Clone)]
pub struct EguiList<T: Debug> {
    is_scrollable: bool,
    is_editable: bool,
    is_reorderable: bool,
    backing: Vec<T>,
}

impl<T: Debug> Default for EguiList<T> {
    fn default() -> Self {
        Self {
            is_scrollable: false,
            is_editable: false,
            is_reorderable: false,
            backing: vec![],
        }
    }
}

impl<T: Debug> EguiList<T> {
    #[must_use]
    pub const fn is_scrollable(mut self, is_scrollable: bool) -> Self {
        self.is_scrollable = is_scrollable;
        self
    }

    #[must_use]
    pub const fn is_editable(mut self, is_editable: bool) -> Self {
        self.is_editable = is_editable;
        self
    }

    #[must_use]
    pub const fn is_reorderable(mut self, is_reorderable: bool) -> Self {
        self.is_reorderable = is_reorderable;
        self
    }

    fn display_inner(&mut self, ui: &mut Ui, label: impl Fn(&T, usize) -> String) {
        if self.backing.is_empty() {
            //If we don't have any arguments, then we don't need any of this and some of the logic gets screwed
            return;
        }

        //we could have multiple (as in vecs rather than options), but immediate mode, so unlikely to affect UX but much easier to juggle for me
        let mut need_to_remove = None; //we need to remove this index
        let mut up = None; //move this index up a position
        let mut down = None; //move this index down a position

        for (i, arg) in self.backing.iter().enumerate() {
            ui.horizontal(|ui| {
                //for each of our CLI args, make a new horizontal environment (to almost mimic a table without alignment), and add buttons for remove/up/down, and if we get input then set relevant variables
                ui.label(label(arg, i));
                if self.is_editable && ui.button("Remove?").clicked() {
                    need_to_remove = Some(i);
                }
                if self.is_reorderable {
                    if ui.button("Up?").clicked() {
                        up = Some(i);
                    }
                    if ui.button("Down?").clicked() {
                        down = Some(i);
                    }
                }
            });
        }

        let len_minus_one = self.backing.len() - 1;
        if let Some(need_to_remove) = need_to_remove {
            let removed = self.backing.remove(need_to_remove);
            info!(?removed, "Removing Arg");
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

    pub fn display(&mut self, ui: &mut Ui, label: impl Fn(&T, usize) -> String) {
        if self.is_scrollable {
            ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                self.display_inner(ui, label);
            });
        } else {
            self.display_inner(ui, label);
        }
    }
}

impl<T: Debug> Deref for EguiList<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.backing
    }
}

impl<T: Debug> DerefMut for EguiList<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.backing
    }
}

impl<T: Debug> From<Vec<T>> for EguiList<T> {
    fn from(value: Vec<T>) -> Self {
        Self {
            backing: value,
            ..Default::default()
        }
    }
}

impl<T: Debug> IntoIterator for EguiList<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.backing.into_iter()
    }
}

impl<T: Debug + Clone> EguiList<T> {
    #[must_use]
    pub fn backing_vec(&self) -> Vec<T> {
        self.backing.clone()
    }
}
