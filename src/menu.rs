/// Action executed when a sub-menu item is selected.
#[derive(Clone, Copy, PartialEq)]
pub enum MenuAction {
    Noop,
    Quit,
    ReturnToView,
    ColumnAdd,
    ColumnRemove,
    ColumnProperties,
    ColumnMove,
    ColumnWidth,
    SectionAdd,
}

pub struct SubSubItem {
    pub label:       &'static str,
    pub description: &'static str,
    pub action:      MenuAction,
}

pub struct SubItem {
    pub label:       &'static str,
    pub description: &'static str,
    pub action:      MenuAction,
    pub children:    Option<&'static [SubSubItem]>,
}

pub struct TopItem {
    pub label: &'static str,
    pub sub:   &'static [SubItem],
}

// ── View screen sub-menus ─────────────────────────────────────────────────────

static FILE_SUB: &[SubItem] = &[
    SubItem { label: "Retrieve",    description: "Retrieve a saved view from disk",        action: MenuAction::Noop, children: None },
    SubItem { label: "Save",        description: "Save current work to disk",               action: MenuAction::Noop, children: None },
    SubItem { label: "Abandon",     description: "Abandon changes since last save",         action: MenuAction::Noop, children: None },
    SubItem { label: "Properties",  description: "View or change file properties",          action: MenuAction::Noop, children: None },
    SubItem { label: "Transfer",    description: "Transfer data to or from another file",   action: MenuAction::Noop, children: None },
    SubItem { label: "Maintenance", description: "Perform file maintenance operations",     action: MenuAction::Noop, children: None },
];

static ITEM_SUB: &[SubItem] = &[
    SubItem { label: "Properties",  description: "View or change item properties",          action: MenuAction::Noop, children: None },
    SubItem { label: "MakeAssign",  description: "Assign current item to a category",       action: MenuAction::Noop, children: None },
    SubItem { label: "Discard",     description: "Discard the current item",                action: MenuAction::Noop, children: None },
    SubItem { label: "Undisc",      description: "Undiscard a previously discarded item",   action: MenuAction::Noop, children: None },
    SubItem { label: "BrkAssign",   description: "Break a category assignment",             action: MenuAction::Noop, children: None },
    SubItem { label: "Reposition",  description: "Reposition item within a section",        action: MenuAction::Noop, children: None },
    SubItem { label: "Alarm",       description: "Set or edit an alarm for this item",      action: MenuAction::Noop, children: None },
];

static CATEGORY_SUB: &[SubItem] = &[
    SubItem { label: "Properties",  description: "View or change category properties",      action: MenuAction::Noop, children: None },
    SubItem { label: "Add",         description: "Add a new category assignment",           action: MenuAction::Noop, children: None },
    SubItem { label: "Discard",     description: "Discard the current category assignment", action: MenuAction::Noop, children: None },
];

static COLUMN_SUB: &[SubSubItem] = &[
    SubSubItem { label: "Properties", description: "Edit the current column's name and width", action: MenuAction::ColumnProperties },
    SubSubItem { label: "Add",        description: "Add a new column to the view",              action: MenuAction::ColumnAdd },
    SubSubItem { label: "Remove",     description: "Remove the current column from the view",   action: MenuAction::ColumnRemove },
    SubSubItem { label: "Move",       description: "Move the current column left or right",     action: MenuAction::ColumnMove },
    SubSubItem { label: "Width",      description: "Change the width of the current column",    action: MenuAction::ColumnWidth },
];

static SECTION_SUB: &[SubSubItem] = &[
    SubSubItem { label: "Add",    description: "Add a new section to the view",    action: MenuAction::SectionAdd },
];

static VIEW_SUB: &[SubItem] = &[
    SubItem { label: "Properties",  description: "View or change view properties",          action: MenuAction::Noop, children: None },
    SubItem { label: "Add",         description: "Add a new view",                          action: MenuAction::Noop, children: None },
    SubItem { label: "Browse",      description: "Browse and select among all views",       action: MenuAction::Noop, children: None },
    SubItem { label: "Discard",     description: "Discard the current view",                action: MenuAction::Noop, children: None },
    SubItem { label: "Column",      description: "Edit view column definitions",            action: MenuAction::Noop, children: Some(COLUMN_SUB) },
    SubItem { label: "Section",     description: "Edit view section definitions",           action: MenuAction::Noop, children: Some(SECTION_SUB) },
];

static PRINT_SUB: &[SubItem] = &[
    SubItem { label: "Final",       description: "Print final output to printer",           action: MenuAction::Noop, children: None },
    SubItem { label: "Preview",     description: "Preview print output on screen",          action: MenuAction::Noop, children: None },
    SubItem { label: "Layout",      description: "Edit the print layout",                   action: MenuAction::Noop, children: None },
    SubItem { label: "Setup",       description: "Edit print setup options",                action: MenuAction::Noop, children: None },
    SubItem { label: "Named",       description: "Use a named print configuration",         action: MenuAction::Noop, children: None },
];

static UTILITIES_SUB: &[SubItem] = &[
    SubItem { label: "Customize",   description: "Customize Agenda settings",               action: MenuAction::Noop, children: None },
    SubItem { label: "Execute",     description: "Execute a macro",                         action: MenuAction::Noop, children: None },
    SubItem { label: "Show",        description: "Show system or Agenda information",       action: MenuAction::Noop, children: None },
    SubItem { label: "Questions",   description: "Answer Agenda setup questions",           action: MenuAction::Noop, children: None },
    SubItem { label: "Trash",       description: "Empty the trash",                         action: MenuAction::Noop, children: None },
    SubItem { label: "Launch",      description: "Launch another program",                  action: MenuAction::Noop, children: None },
];

static SYSTEM_SUB: &[SubItem] = &[
    SubItem { label: "Suspend",     description: "Suspend Agenda to the operating system",  action: MenuAction::Noop, children: None },
];

static QUIT_SUB: &[SubItem] = &[
    SubItem { label: "Yes",         description: "Save work and quit Agenda",               action: MenuAction::Quit, children: None },
    SubItem { label: "No",          description: "Return to Agenda without quitting",       action: MenuAction::Noop, children: None },
];

pub static VIEW_MENU: &[TopItem] = &[
    TopItem { label: "File",        sub: FILE_SUB },
    TopItem { label: "Item",        sub: ITEM_SUB },
    TopItem { label: "Category",    sub: CATEGORY_SUB },
    TopItem { label: "View",        sub: VIEW_SUB },
    TopItem { label: "Print",       sub: PRINT_SUB },
    TopItem { label: "Utilities",   sub: UTILITIES_SUB },
    TopItem { label: "System",      sub: SYSTEM_SUB },
    TopItem { label: "Quit",        sub: QUIT_SUB },
];

// ── Category Manager screen sub-menus ─────────────────────────────────────────

static CATMGR_CAT_SUB: &[SubItem] = &[
    SubItem { label: "Properties",  description: "View or change category properties",      action: MenuAction::Noop, children: None },
    SubItem { label: "Add",         description: "Add a new category",                      action: MenuAction::Noop, children: None },
    SubItem { label: "Discard",     description: "Discard the current category",            action: MenuAction::Noop, children: None },
];

static CATMGR_PRINT_SUB: &[SubItem] = &[
    SubItem { label: "Final",       description: "Print the category list",                 action: MenuAction::Noop, children: None },
    SubItem { label: "Preview",     description: "Preview the category list print",         action: MenuAction::Noop, children: None },
];

static RETURN_SUB: &[SubItem] = &[
    SubItem { label: "View",        description: "Return to the View screen",               action: MenuAction::ReturnToView, children: None },
];

pub static CATMGR_MENU: &[TopItem] = &[
    TopItem { label: "Category",    sub: CATMGR_CAT_SUB },
    TopItem { label: "Print",       sub: CATMGR_PRINT_SUB },
    TopItem { label: "Return",      sub: RETURN_SUB },
    TopItem { label: "Quit",        sub: QUIT_SUB },
];
