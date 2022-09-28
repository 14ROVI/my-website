use yew::Html;

#[derive(PartialEq, Eq)]
pub enum WindowState {
    Minimised,
    Open,
    Focused
}

pub struct Window {
    pub id: usize,
    pub state: WindowState,
    pub closable: bool,
    pub maximised: bool,
    pub top: i32,
    pub left: i32,
    pub width: u32,
    pub z: u32,
    pub dragging: bool,
    pub icon: String,
    pub title: String,
    pub body: Html
}