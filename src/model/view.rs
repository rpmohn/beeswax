use super::section::Section;

pub struct View {
    pub id:       usize,
    pub name:     String,
    pub sections: Vec<Section>,
}
