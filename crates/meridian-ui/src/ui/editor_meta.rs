use meridian_core::render::RendererKind;

#[derive(Debug, Clone, Copy)]
pub enum UiControlKind {
    Toggle,
    Slider { min: f32, max: f32 },
    Choice,
    Text,
}

#[derive(Debug, Clone, Copy)]
pub struct UiFieldMeta {
    pub section: &'static str,
    pub key: &'static str,
    pub label: &'static str,
    pub control: UiControlKind,
    pub default_hint: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct ProjectorEditorMeta {
    pub renderer: RendererKind,
    pub label: &'static str,
    pub fields: &'static [UiFieldMeta],
}

const PFA_FIELDS: &[UiFieldMeta] = &[
    UiFieldMeta {
        section: "Notes",
        key: "same_width_notes",
        label: "Same Width Notes",
        control: UiControlKind::Toggle,
        default_hint: "off",
    },
    UiFieldMeta {
        section: "Notes",
        key: "border_width",
        label: "Border Width",
        control: UiControlKind::Slider { min: 0.0, max: 6.0 },
        default_hint: "1.0",
    },
    UiFieldMeta {
        section: "Keyboard",
        key: "middle_c",
        label: "Middle C Marker",
        control: UiControlKind::Toggle,
        default_hint: "off",
    },
    UiFieldMeta {
        section: "Keyboard",
        key: "top_color",
        label: "Top Preset",
        control: UiControlKind::Choice,
        default_hint: "red",
    },
    UiFieldMeta {
        section: "Keyboard",
        key: "top_bar_color",
        label: "Top Bar Hex",
        control: UiControlKind::Text,
        default_hint: "#950A06",
    },
];

const FLAT_FIELDS: &[UiFieldMeta] = &[UiFieldMeta {
    section: "Notes",
    key: "palette",
    label: "Palette",
    control: UiControlKind::Choice,
    default_hint: "default_track_colors",
}];

const PIANO_TRAIL_CLASSIC_FIELDS: &[UiFieldMeta] = &[
    UiFieldMeta {
        section: "Scene",
        key: "same_width_notes",
        label: "Same Width Notes",
        control: UiControlKind::Toggle,
        default_hint: "on",
    },
    UiFieldMeta {
        section: "Scene",
        key: "box_notes",
        label: "Box Notes",
        control: UiControlKind::Toggle,
        default_hint: "off",
    },
    UiFieldMeta {
        section: "Scene",
        key: "show_keyboard",
        label: "Show Keyboard",
        control: UiControlKind::Toggle,
        default_hint: "on",
    },
    UiFieldMeta {
        section: "Camera",
        key: "fov",
        label: "FOV",
        control: UiControlKind::Slider {
            min: 20.0,
            max: 120.0,
        },
        default_hint: "60deg",
    },
    UiFieldMeta {
        section: "Camera",
        key: "view_height",
        label: "View Height",
        control: UiControlKind::Slider {
            min: -2.0,
            max: 2.0,
        },
        default_hint: "0.5",
    },
    UiFieldMeta {
        section: "Camera",
        key: "view_offset",
        label: "View Offset",
        control: UiControlKind::Slider {
            min: -2.0,
            max: 2.0,
        },
        default_hint: "0.4",
    },
];

pub const PROJECTOR_EDITOR_META: &[ProjectorEditorMeta] = &[
    ProjectorEditorMeta {
        renderer: RendererKind::Pfa,
        label: "PFA",
        fields: PFA_FIELDS,
    },
    ProjectorEditorMeta {
        renderer: RendererKind::Flat,
        label: "Flat",
        fields: FLAT_FIELDS,
    },
    ProjectorEditorMeta {
        renderer: RendererKind::PianoTrailClassic,
        label: "Piano Trail Classic",
        fields: PIANO_TRAIL_CLASSIC_FIELDS,
    },
];

pub fn projector_editor_meta(renderer: RendererKind) -> &'static ProjectorEditorMeta {
    PROJECTOR_EDITOR_META
        .iter()
        .find(|meta| meta.renderer == renderer)
        .unwrap_or(&PROJECTOR_EDITOR_META[0])
}
