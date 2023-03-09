//! Makes an ordered list of things to be displayed in an EGUI window
//! 

use std::fmt::{Display, Debug};

trait DontCareDisplay {
    fn display (&self) -> String; 
}

impl<T: Display> DontCareDisplay for T {
    fn display (&self) -> String {
        format!("{self}")
    }
}
impl<T: Debug> DontCareDisplay for T {
    fn display (&self) -> String {
        format!("{self:?}")
    }
}

pub struct EguiList<T: DontCareDisplay> {
    is_scrollable: bool,
    is_editable: bool,
    backing: Vec<T>,
    item_label: String,
}

impl<T: DontCareDisplay> EguiList<T> {
    pub fn new () -> Self {
        Self {
            is_scrollable: false,
            is_editable: true,
            backing: vec![],
            item_label: "".into()
        }
    }

    pub fn is_scrollable (mut self, is_scrollable: bool) -> Self {
        self.is_scrollable = is_scrollable;
        self
    }

    pub fn is_editable (mut self, is_editable: bool) -> Self {
        self.is_editable = is_editable;
        self
    }

    pub fn item_label (mut self, item_label: bool) -> Self {
        self.item_label = item_label;
        self
    }

    pub fn add_to_backing (mut self, item: T) -> Self {
        self.backing.push(item);
        self
    }
}